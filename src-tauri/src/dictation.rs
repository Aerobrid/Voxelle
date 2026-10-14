//! Dictation controller: the one place that drives a session through
//! start → record → stop → transcribe → insert. Both UI commands and global hotkeys call these
//! functions; they block, so callers run them on worker threads, never the UI thread.
//!
//! Locks are held briefly and never across transcription, so the UI can always read the state
//! and cancel stays responsive.

use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::app_state::AppState;
use crate::asr::{AsrError, LoadOptions};
use crate::audio::{CapturedAudio, ASR_SAMPLE_RATE};
use crate::insert::{sanitize_for_insertion, InsertOutcome, InsertResult};
use crate::models::{self, catalog_entry, Verify};
use crate::session::{Event, SessionSnapshot, SessionState};
use crate::storage::HistoryEntry;

pub const SESSION_EVENT: &str = "session_update";
pub const RESULT_EVENT: &str = "dictation_result";

/// Shorter clips are almost always accidental taps; whisper also hallucinates on them.
const MIN_RECORDING_MS: u64 = 300;
/// Peak below this means the mic delivered digital silence (muted/wrong device).
const SILENCE_PEAK: f32 = 1e-4;

/// Timings for one dictation, reported to the UI and used for the P0 measurements.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationMetrics {
    /// Hotkey/UI request → microphone stream running.
    pub start_latency_ms: u64,
    pub audio_ms: u64,
    pub model_load_ms: u64,
    pub transcribe_ms: u64,
    /// Stop request → transcript ready.
    pub stop_to_transcript_ms: u64,
    /// Stop request → insertion attempt finished.
    pub stop_to_insert_ms: u64,
    pub real_time_factor: f32,
    pub gpu: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationResult {
    pub history_id: Option<String>,
    /// Recognized text exactly as produced by the speech model (never AI-edited).
    pub transcript: String,
    pub language: Option<String>,
    pub insert: InsertResult,
    pub metrics: DictationMetrics,
}

fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

fn emit(app: &AppHandle, snap: &SessionSnapshot) {
    state(app).session_active.store(
        matches!(snap.state, SessionState::Recording | SessionState::Processing),
        std::sync::atomic::Ordering::SeqCst,
    );
    crate::overlay::sync(app, snap, snap.generation);
    let _ = app.emit(SESSION_EVENT, snap.clone());
}

/// Apply a failure event for `generation` (if still current) with a user-facing message.
fn fail(app: &AppHandle, generation: u64, event: Event, message: String) -> String {
    let st = state(app);
    let mut session = st.session.lock().unwrap();
    if session.generation() == generation && session.apply(event).is_ok() {
        session.set_message(message.clone());
        emit(app, &session.snapshot());
    }
    message
}

pub fn start(app: &AppHandle) -> Result<(), String> {
    let requested = Instant::now();
    let st = state(app);
    let device = st.settings.lock().unwrap().input_device_id.clone();

    // Hold the session lock across mic open so a racing stop/toggle sees a consistent state.
    let mut session = st.session.lock().unwrap();
    session.apply(Event::Start).map_err(|e| e.to_string())?;
    let generation = session.generation();
    st.asr_cancel.reset();

    let opened = st.audio.lock().unwrap().start(device.as_deref());
    if let Err(e) = opened {
        st.audio.lock().unwrap().abort(); // release the mic on failure
        let _ = session.apply(Event::DeviceError);
        session.set_message(e.to_string());
        emit(app, &session.snapshot());
        return Err(e.to_string());
    }
    let start_latency_ms = requested.elapsed().as_millis() as u64;
    emit(app, &session.snapshot());
    drop(session);
    eprintln!("[voxelle] recording started ({start_latency_ms} ms)");
    *LAST_START_LATENCY.lock().unwrap() = start_latency_ms;

    spawn_level_monitor(app.clone(), generation);
    Ok(())
}

static LAST_START_LATENCY: std::sync::Mutex<u64> = std::sync::Mutex::new(0);

/// Pushes level/elapsed to the UI while recording and turns a device disconnect into a clean
/// failure (mic released) instead of a hung session.
fn spawn_level_monitor(app: AppHandle, generation: u64) {
    let started = Instant::now();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(50));
        let st = state(&app);
        let mut session = st.session.lock().unwrap();
        if session.generation() != generation || session.state() != SessionState::Recording {
            return;
        }
        let (level, disconnected) = {
            let audio = st.audio.lock().unwrap();
            (audio.level(), audio.disconnected())
        };
        if disconnected {
            st.audio.lock().unwrap().abort();
            let _ = session.apply(Event::DeviceError);
            session.set_message("Microphone disconnected — recording stopped. Reconnect it or pick another input in Settings.");
            emit(&app, &session.snapshot());
            return;
        }
        session.set_level(level);
        session.set_elapsed_ms(started.elapsed().as_millis() as u64);
        emit(&app, &session.snapshot());
    });
}

pub fn cancel(app: &AppHandle) -> Result<(), String> {
    let st = state(app);
    let mut session = st.session.lock().unwrap();
    session.apply(Event::Cancel).map_err(|e| e.to_string())?;
    st.asr_cancel.cancel(); // stops whisper at its next abort check; no asr lock needed
    st.audio.lock().unwrap().abort(); // release mic (no-op if already stopped)
    emit(app, &session.snapshot());
    Ok(())
}

/// Toggle semantics for a single hotkey: start when idle/terminal, stop when recording.
/// Presses during processing/inserting are ignored (never a second session).
pub fn toggle(app: &AppHandle) -> Result<(), String> {
    let current = state(app).session.lock().unwrap().state();
    match current {
        SessionState::Recording => stop(app).map(|_| ()),
        s if s == SessionState::Idle || s.is_terminal() => start(app),
        _ => Ok(()),
    }
}

pub fn stop(app: &AppHandle) -> Result<DictationResult, String> {
    let stop_requested = Instant::now();
    let st = state(app);
    let generation = {
        let mut session = st.session.lock().unwrap();
        session.apply(Event::Stop).map_err(|e| e.to_string())?;
        emit(app, &session.snapshot());
        session.generation()
    };

    let captured = st.audio.lock().unwrap().stop(); // releases the device
    let captured = match captured {
        Ok(c) => c,
        Err(e) => return Err(fail(app, generation, Event::AsrError, e.to_string())),
    };
    if let Some(problem) = audio_problem(&captured) {
        return Err(fail(app, generation, Event::AsrError, problem));
    }

    let (language, use_gpu, model_id, store) = {
        let s = st.settings.lock().unwrap();
        let lang = (s.language != "auto").then(|| s.language.clone());
        (lang, s.prefer_gpu, s.selected_speech_model_id.clone(), s.store_transcripts)
    };

    let mut metrics = DictationMetrics {
        start_latency_ms: *LAST_START_LATENCY.lock().unwrap(),
        audio_ms: captured.duration_ms(),
        ..Default::default()
    };

    let pcm = captured.to_asr_rate();
    let transcribed = {
        let mut asr = st.asr.lock().unwrap();
        let load_started = Instant::now();
        if let Err(e) = ensure_model_loaded(&st, asr.as_mut(), model_id.as_deref(), use_gpu) {
            drop(asr);
            return Err(fail(app, generation, Event::AsrError, e));
        }
        metrics.model_load_ms = load_started.elapsed().as_millis() as u64;
        metrics.gpu = asr.capabilities().gpu_active;
        let t = Instant::now();
        let r = asr.transcribe(&pcm, language.as_deref(), &st.asr_cancel);
        metrics.transcribe_ms = t.elapsed().as_millis() as u64;
        r
    };
    metrics.stop_to_transcript_ms = stop_requested.elapsed().as_millis() as u64;
    metrics.real_time_factor = if metrics.audio_ms > 0 {
        metrics.transcribe_ms as f32 / (pcm.len() as f32 * 1000.0 / ASR_SAMPLE_RATE as f32)
    } else {
        0.0
    };

    if transcribed.is_ok() {
        if let Some(id) = model_id.as_deref() {
            // "Measured speed on this machine" for the model manager comes from real dictations.
            let backend = if metrics.gpu { "GPU (Vulkan)" } else { "CPU" };
            let rtf = metrics.real_time_factor;
            st.update_facts(id, |f| {
                f.measured_rtf = Some(rtf);
                f.measured_backend = Some(backend.into());
            });
        }
    }

    let transcription = match transcribed {
        Ok(t) => t,
        Err(AsrError::Cancelled) => return Err("cancelled".into()),
        Err(e) => return Err(fail(app, generation, Event::AsrError, e.to_string())),
    };
    if transcription.text.is_empty() {
        return Err(fail(
            app,
            generation,
            Event::AsrError,
            "No speech recognized. Try speaking closer to the microphone.".into(),
        ));
    }

    {
        let mut session = st.session.lock().unwrap();
        if session.generation() != generation || session.state() != SessionState::Processing {
            return Err("cancelled".into()); // cancelled while transcribing: discard
        }
        session.apply(Event::TranscriptReady).map_err(|e| e.to_string())?;
        emit(app, &session.snapshot());
    }

    // Persist the RAW transcript (never overwritten by AI output) unless storage is disabled.
    let history_id = store.then(|| {
        // Unique across restarts: time-based plus a per-run counter.
        let id = format!(
            "h{}-{}",
            chrono::Utc::now().timestamp_micros(),
            st.id_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let saved = st.store.insert_history(
            &HistoryEntry {
                id: id.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                raw_transcript: transcription.text.clone(),
                edited_text: None,
                duration_ms: metrics.audio_ms,
                language: transcription.language.clone(),
            },
        );
        st.apply_retention();
        match saved {
            Ok(()) => Some(id),
            Err(e) => {
                eprintln!("[voxelle] could not save history: {e}");
                None
            }
        }
    }).flatten();

    let sanitized = sanitize_for_insertion(&transcription.text);
    let insert = st.inserter.lock().unwrap().insert(&sanitized);
    metrics.stop_to_insert_ms = stop_requested.elapsed().as_millis() as u64;
    {
        let mut session = st.session.lock().unwrap();
        let event = match insert.outcome {
            InsertOutcome::Inserted => Event::Inserted,
            InsertOutcome::CopiedOnly => Event::CopiedOnly,
            InsertOutcome::Failed => Event::InsertError,
        };
        if session.generation() == generation && session.apply(event).is_ok() {
            if let Some(m) = &insert.message {
                session.set_message(m.clone());
            }
            emit(app, &session.snapshot());
        }
    }

    eprintln!(
        "[voxelle] dictation: audio {} ms, load {} ms, transcribe {} ms, stop→text {} ms, stop→insert {} ms, rtf {:.3}, gpu {}, outcome {:?}",
        metrics.audio_ms,
        metrics.model_load_ms,
        metrics.transcribe_ms,
        metrics.stop_to_transcript_ms,
        metrics.stop_to_insert_ms,
        metrics.real_time_factor,
        metrics.gpu,
        insert.outcome
    );
    let result = DictationResult {
        history_id,
        transcript: transcription.text,
        language: transcription.language,
        insert,
        metrics,
    };
    let _ = app.emit(RESULT_EVENT, result.clone());
    Ok(result)
}

fn audio_problem(c: &CapturedAudio) -> Option<String> {
    if c.duration_ms() < MIN_RECORDING_MS {
        return Some("Recording too short — hold the key a little longer.".into());
    }
    let peak = c.pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak < SILENCE_PEAK {
        return Some(
            "The microphone delivered only silence. Check that it is not muted and that the right input is selected.".into(),
        );
    }
    if crate::audio::peak_frame_dbfs(&c.pcm, c.sample_rate) < crate::audio::SPEECH_FLOOR_DBFS {
        return Some("No speech detected — the recording was too quiet. Try speaking closer to the microphone.".into());
    }
    None
}

/// Load the selected model if it is not already loaded. Verifies format + SHA-256 once per run
/// before handing the file to the native loader.
pub fn ensure_model_loaded(
    st: &AppState,
    asr: &mut dyn crate::asr::SpeechBackend,
    model_id: Option<&str>,
    use_gpu: bool,
) -> Result<(), String> {
    let id = model_id.ok_or_else(|| AsrError::NoModel.to_string())?;
    let entry = catalog_entry(id).ok_or_else(|| format!("Unknown model '{id}'."))?;
    let path = models::model_path(&st.model_dir(), entry);
    if !models::is_installed(&st.model_dir(), entry) {
        return Err(format!(
            "{} is not installed (expected {}). Install it in Model Manager.",
            entry.name,
            path.display()
        ));
    }
    if !st.facts(id).integrity_verified {
        if !models::has_ggml_magic(&path).map_err(|e| e.to_string())? {
            return Err(format!("{} is not a whisper.cpp model file.", path.display()));
        }
        match models::verify_file(&path, entry.size_bytes, entry.sha256).map_err(|e| e.to_string())? {
            Verify::Ok => {
                st.update_facts(id, |f| f.integrity_verified = true);
            }
            other => {
                return Err(format!(
                    "{} failed its integrity check ({other:?}). Remove and reinstall it.",
                    entry.name
                ))
            }
        }
    }
    let opts = LoadOptions {
        use_gpu,
        threads: crate::asr::default_threads(),
    };
    asr.load(&path, &opts).map_err(|e| e.to_string())
}

/// Load (and warm up) the selected model in the background so the first dictation doesn't pay for
/// model load and GPU pipeline setup (~1-2 s on the dev machine). Safe to call repeatedly.
pub fn preload_selected_model(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let st = state(&app);
        let (model_id, use_gpu) = {
            let s = st.settings.lock().unwrap();
            (s.selected_speech_model_id.clone(), s.prefer_gpu)
        };
        if model_id.is_none() {
            return;
        }
        let t = Instant::now();
        let mut asr = st.asr.lock().unwrap();
        match ensure_model_loaded(&st, asr.as_mut(), model_id.as_deref(), use_gpu) {
            Ok(()) => {
                // One short pass compiles GPU pipelines / touches weights. Low-level tone, not
                // silence, so whisper does not spend time in its temperature-fallback retries.
                let warm: Vec<f32> = (0..ASR_SAMPLE_RATE as usize)
                    .map(|i| 0.01 * (i as f32 * 0.07).sin())
                    .collect();
                let _ = asr.transcribe(&warm, Some("en"), &crate::asr::CancelToken::default());
                eprintln!("[voxelle] model ready in {} ms (gpu: {})", t.elapsed().as_millis(), asr.capabilities().gpu_active);
            }
            Err(e) => eprintln!("[voxelle] model preload failed: {e}"),
        }
    });
}
