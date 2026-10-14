//! Shared, thread-safe application state managed by Tauri. Backends are trait objects so engines
//! and platform adapters swap without touching the dictation controller or command layer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;

use crate::asr::{CancelToken, SpeechBackend, WhisperBackend};
use crate::audio::{AudioCapture, CpalCapture};
use crate::insert::TextInserter;
use crate::models::ModelRuntimeFacts;
use crate::session::Session;
use crate::storage::{HistoryEntry, Settings};

pub struct AppState {
    pub session: Mutex<Session>,
    pub settings: Mutex<Settings>,
    pub history: Mutex<Vec<HistoryEntry>>,
    /// Default model directory (app local data); `settings.model_dir` overrides it.
    pub default_model_dir: PathBuf,
    pub model_facts: Mutex<HashMap<String, ModelRuntimeFacts>>,
    pub audio: Mutex<Box<dyn AudioCapture>>,
    pub asr: Mutex<Box<dyn SpeechBackend>>,
    /// Cancels in-flight transcription without taking the `asr` lock.
    pub asr_cancel: CancelToken,
    pub inserter: Mutex<Box<dyn TextInserter>>,
    pub id_counter: AtomicU64,
    /// Recording or processing. Lock-free so the keyboard hook can read it without blocking.
    pub session_active: AtomicBool,
    /// Live hotkey bindings (set once the hook is installed); settings updates swap them.
    pub hotkeys: Mutex<Option<crate::hotkeys::SharedBindings>>,
}

impl AppState {
    pub fn new(default_model_dir: PathBuf, inserter: Box<dyn TextInserter>) -> Self {
        Self {
            session: Mutex::new(Session::new()),
            settings: Mutex::new(Settings::default()),
            history: Mutex::new(Vec::new()),
            default_model_dir,
            model_facts: Mutex::new(HashMap::new()),
            audio: Mutex::new(Box::new(CpalCapture::new())),
            asr: Mutex::new(Box::new(WhisperBackend::new())),
            asr_cancel: CancelToken::default(),
            inserter: Mutex::new(inserter),
            id_counter: AtomicU64::new(1),
            session_active: AtomicBool::new(false),
            hotkeys: Mutex::new(None),
        }
    }

    pub fn model_dir(&self) -> PathBuf {
        self.settings
            .lock()
            .unwrap()
            .model_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_model_dir.clone())
    }

    pub fn facts(&self, id: &str) -> ModelRuntimeFacts {
        self.model_facts
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .unwrap_or_default()
    }
}
