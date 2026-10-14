//! Tauri IPC command layer. The renderer calls only these validated commands (no direct OS access).
//! Anything that can block (mic open, transcription, hashing) runs on a blocking worker thread.

use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::asr::Capabilities;
use crate::audio::InputDevice;
use crate::dictation::{self, DictationResult};
use crate::models::{self, catalog_entry, ModelInfo, Verify};
use crate::session::SessionSnapshot;
use crate::storage::{HistoryEntry, Settings};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("worker failed: {e}"))?
}

#[tauri::command]
pub fn get_session_snapshot(state: State<AppState>) -> SessionSnapshot {
    state.session.lock().unwrap().snapshot()
}

#[tauri::command]
pub async fn start_dictation(app: AppHandle) -> Result<(), String> {
    blocking(move || dictation::start(&app)).await
}

#[tauri::command]
pub async fn stop_dictation(app: AppHandle) -> Result<DictationResult, String> {
    blocking(move || dictation::stop(&app)).await
}

#[tauri::command]
pub async fn cancel_dictation(app: AppHandle) -> Result<(), String> {
    blocking(move || dictation::cancel(&app)).await
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<AppState>,
    patch: serde_json::Value,
) -> Result<Settings, String> {
    let mut settings = state.settings.lock().unwrap();
    let mut current = serde_json::to_value(&*settings).map_err(|e| e.to_string())?;
    if let (Some(obj), Some(p)) = (current.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            // Reject unknown keys instead of silently ignoring them.
            if !obj.contains_key(k) {
                return Err(format!("unknown setting: {k}"));
            }
            obj.insert(k.clone(), v.clone());
        }
    } else {
        return Err("settings patch must be an object".into());
    }
    let merged: Settings = serde_json::from_value(current).map_err(|e| e.to_string())?;
    // Validate shortcuts before accepting anything, then hot-swap them into the live hook.
    let bindings = crate::hotkeys::Bindings::from_settings(&merged)?;
    let reload_model = settings.selected_speech_model_id != merged.selected_speech_model_id
        || settings.prefer_gpu != merged.prefer_gpu;
    state.save_settings(&merged)?; // persist first: never apply a setting that was not saved
    if let Some(shared) = state.hotkeys.lock().unwrap().as_ref() {
        *shared.write().unwrap() = bindings;
    }
    *settings = merged.clone();
    drop(settings);
    if reload_model {
        crate::dictation::preload_selected_model(&app);
    }
    Ok(merged)
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Vec<ModelInfo> {
    let selected = state.settings.lock().unwrap().selected_speech_model_id.clone();
    models::describe(&state.model_dir(), selected.as_deref(), &|id| state.facts(id))
}

#[tauri::command]
pub fn select_model(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = catalog_entry(&id).ok_or_else(|| format!("unknown model: {id}"))?;
    if !models::is_installed(&state.model_dir(), entry) {
        return Err(format!("{} is not installed", entry.name));
    }
    {
        let mut s = state.settings.lock().unwrap();
        let mut next = s.clone();
        next.selected_speech_model_id = Some(id);
        state.save_settings(&next)?;
        *s = next;
    }
    crate::dictation::preload_selected_model(&app);
    Ok(())
}

/// Recompute the SHA-256 of an installed model and compare with the pinned value.
#[tauri::command]
pub async fn verify_model(app: AppHandle, id: String) -> Result<bool, String> {
    blocking(move || {
        use tauri::Manager;
        let state = app.state::<AppState>();
        let entry = catalog_entry(&id).ok_or_else(|| format!("unknown model: {id}"))?;
        let path = models::model_path(&state.model_dir(), entry);
        let ok = models::has_ggml_magic(&path).unwrap_or(false)
            && models::verify_file(&path, entry.size_bytes, entry.sha256).map_err(|e| e.to_string())?
                == Verify::Ok;
        state.update_facts(&id, |f| f.integrity_verified = ok);
        Ok(ok)
    })
    .await
}

#[tauri::command]
pub fn asr_capabilities(state: State<AppState>) -> Capabilities {
    state.asr.lock().unwrap().capabilities()
}

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Result<Vec<HistoryEntry>, String> {
    state.store.list_history().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history_entry(state: State<AppState>, id: String) -> Result<(), String> {
    match state.store.delete_history(&id) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("no such entry: {id}")),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) -> Result<usize, String> {
    state.store.clear_history().map_err(|e| e.to_string())
}

/// Write all history to a JSON file in the user's Downloads folder and return its path.
#[tauri::command]
pub fn export_history(app: AppHandle, state: State<AppState>) -> Result<String, String> {
    use tauri::Manager;
    let entries = state.store.list_history().map_err(|e| e.to_string())?;
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let path = dir.join(format!(
        "voxelle-history-{}.json",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "format": "voxelle-history",
        "version": 1,
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "entries": entries,
    }))
    .map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub persistent: bool,
    pub data_dir: String,
    pub model_dir: String,
}

#[tauri::command]
pub fn storage_info(app: AppHandle, state: State<AppState>) -> StorageInfo {
    use tauri::Manager;
    StorageInfo {
        persistent: state.storage_persistent,
        data_dir: app
            .path()
            .app_data_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        model_dir: state.model_dir().display().to_string(),
    }
}

#[tauri::command]
pub async fn list_input_devices(app: AppHandle) -> Result<Vec<InputDevice>, String> {
    blocking(move || {
        use tauri::Manager;
        let state = app.state::<AppState>();
        let devices = state.audio.lock().unwrap().list_input_devices();
        devices.map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    crate::insert::copy_text(&text)
}

/// Store a user correction as the edited version. The raw transcript is never modified.
#[tauri::command]
pub fn set_history_edit(
    state: State<AppState>,
    id: String,
    edited_text: Option<String>,
) -> Result<HistoryEntry, String> {
    let raw = state
        .store
        .list_history()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("no such entry: {id}"))?
        .raw_transcript;
    let edited = edited_text.filter(|t| !t.trim().is_empty() && *t != raw);
    state
        .store
        .set_edit(&id, edited.as_deref())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no such entry: {id}"))
}

#[tauri::command]
pub fn get_model_dir(state: State<AppState>) -> String {
    state.model_dir().display().to_string()
}
