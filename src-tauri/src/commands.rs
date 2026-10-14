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
    if let Some(shared) = state.hotkeys.lock().unwrap().as_ref() {
        *shared.write().unwrap() = bindings;
    }
    *settings = merged.clone();
    Ok(merged)
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Vec<ModelInfo> {
    let selected = state.settings.lock().unwrap().selected_speech_model_id.clone();
    models::describe(&state.model_dir(), selected.as_deref(), &|id| state.facts(id))
}

#[tauri::command]
pub fn select_model(state: State<AppState>, id: String) -> Result<(), String> {
    let entry = catalog_entry(&id).ok_or_else(|| format!("unknown model: {id}"))?;
    if !models::is_installed(&state.model_dir(), entry) {
        return Err(format!("{} is not installed", entry.name));
    }
    state.settings.lock().unwrap().selected_speech_model_id = Some(id);
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
        state
            .model_facts
            .lock()
            .unwrap()
            .entry(id)
            .or_default()
            .integrity_verified = ok;
        Ok(ok)
    })
    .await
}

#[tauri::command]
pub fn asr_capabilities(state: State<AppState>) -> Capabilities {
    state.asr.lock().unwrap().capabilities()
}

#[tauri::command]
pub fn get_history(state: State<AppState>) -> Vec<HistoryEntry> {
    state.history.lock().unwrap().clone()
}

#[tauri::command]
pub fn delete_history_entry(state: State<AppState>, id: String) -> Result<(), String> {
    let mut history = state.history.lock().unwrap();
    let before = history.len();
    history.retain(|e| e.id != id);
    if history.len() == before {
        return Err(format!("no such entry: {id}"));
    }
    Ok(())
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) {
    state.history.lock().unwrap().clear();
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
    let mut history = state.history.lock().unwrap();
    let entry = history
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("no such entry: {id}"))?;
    entry.edited_text = edited_text.filter(|t| !t.trim().is_empty() && *t != entry.raw_transcript);
    Ok(entry.clone())
}

#[tauri::command]
pub fn get_model_dir(state: State<AppState>) -> String {
    state.model_dir().display().to_string()
}
