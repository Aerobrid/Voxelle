//! Tauri IPC command layer. The renderer calls only these validated commands (no direct OS access).
//! The full dictation flow is wired here; with stub backends it fails honestly at the first
//! unwired step (audio/ASR) — it never fabricates a transcript or a false "inserted".

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;
use crate::insert::{sanitize_for_insertion, InsertOutcome, InsertResult};
use crate::models::ModelInfo;
use crate::session::{Event, SessionSnapshot};
use crate::storage::{HistoryEntry, Settings};

const SESSION_EVENT: &str = "session_update";

fn emit(app: &AppHandle, snap: &SessionSnapshot) {
    let _ = app.emit(SESSION_EVENT, snap.clone());
}

#[tauri::command]
pub fn get_session_snapshot(state: State<AppState>) -> SessionSnapshot {
    state.session.lock().unwrap().snapshot()
}

#[tauri::command]
pub fn start_dictation(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    // Enforce idle/terminal -> recording. Rejects a second concurrent session.
    session.apply(Event::Start).map_err(|e| e.to_string())?;
    emit(&app, &session.snapshot());

    let mut audio = state.audio.lock().unwrap();
    match audio.start(None) {
        Ok(()) => Ok(()),
        Err(e) => {
            audio.abort(); // release the mic on failure
            let _ = session.apply(Event::DeviceError);
            session.set_message(e.to_string());
            emit(&app, &session.snapshot());
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub fn stop_dictation(app: AppHandle, state: State<AppState>) -> Result<InsertResult, String> {
    let mut session = state.session.lock().unwrap();
    session.apply(Event::Stop).map_err(|e| e.to_string())?; // -> processing
    emit(&app, &session.snapshot());

    // Collect captured audio (also releases the device).
    let captured = state.audio.lock().unwrap().stop();
    let captured = match captured {
        Ok(c) => c,
        Err(e) => {
            let _ = session.apply(Event::AsrError);
            session.set_message(format!("audio: {e}"));
            emit(&app, &session.snapshot());
            return Err(e.to_string());
        }
    };

    // Transcribe locally — no network, no tools (security.md §1).
    let language = {
        let s = state.settings.lock().unwrap();
        if s.language == "auto" {
            None
        } else {
            Some(s.language.clone())
        }
    };
    let transcript = state.asr.lock().unwrap().transcribe(
        &captured.pcm,
        captured.sample_rate,
        language.as_deref(),
    );
    let transcript = match transcript {
        Ok(t) => t,
        Err(e) => {
            let _ = session.apply(Event::AsrError);
            session.set_message(e.to_string());
            emit(&app, &session.snapshot());
            return Err(e.to_string());
        }
    };

    session
        .apply(Event::TranscriptReady)
        .map_err(|e| e.to_string())?; // -> inserting
    emit(&app, &session.snapshot());

    // Persist the RAW transcript (never overwritten by AI output), if storage is enabled.
    {
        let settings = state.settings.lock().unwrap();
        if settings.store_transcripts {
            let n = state.id_counter.fetch_add(1, Ordering::Relaxed);
            let entry = HistoryEntry {
                id: format!("h{n}"),
                created_at: chrono::Utc::now().to_rfc3339(),
                raw_transcript: transcript.clone(),
                edited_text: None,
                duration_ms: (captured.pcm.len() as u64 * 1000)
                    / (captured.sample_rate.max(1) as u64),
                language: language.clone(),
            };
            state.history.lock().unwrap().insert(0, entry);
        }
    }

    // Insert at the cursor. Sanitized; never auto-submits.
    let sanitized = sanitize_for_insertion(&transcript);
    let result = state.inserter.lock().unwrap().insert(&sanitized);
    let event = match result.outcome {
        InsertOutcome::Inserted => Event::Inserted,
        InsertOutcome::CopiedOnly => Event::CopiedOnly,
        InsertOutcome::Failed => Event::InsertError,
    };
    let _ = session.apply(event);
    if result.outcome == InsertOutcome::Failed {
        if let Some(m) = &result.message {
            session.set_message(m.clone());
        }
    }
    emit(&app, &session.snapshot());
    Ok(result)
}

#[tauri::command]
pub fn cancel_dictation(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    session.apply(Event::Cancel).map_err(|e| e.to_string())?;
    state.audio.lock().unwrap().abort(); // release mic
    state.asr.lock().unwrap().cancel();
    emit(&app, &session.snapshot());
    Ok(())
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
            obj.insert(k.clone(), v.clone());
        }
    }
    let merged: Settings = serde_json::from_value(current).map_err(|e| e.to_string())?;
    *settings = merged.clone();
    Ok(merged)
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Vec<ModelInfo> {
    state.models.lock().unwrap().clone()
}

#[tauri::command]
pub fn select_model(state: State<AppState>, id: String) -> Result<(), String> {
    let mut models = state.models.lock().unwrap();
    let mut found = false;
    for m in models.iter_mut() {
        m.selected = m.id == id;
        if m.id == id {
            found = true;
        }
    }
    if !found {
        return Err(format!("unknown model: {id}"));
    }
    state.settings.lock().unwrap().selected_speech_model_id = Some(id);
    Ok(())
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
pub fn list_input_devices(state: State<AppState>) -> Result<Vec<String>, String> {
    state
        .audio
        .lock()
        .unwrap()
        .list_input_devices()
        .map_err(|e| e.to_string())
}
