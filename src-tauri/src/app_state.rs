//! Shared, thread-safe application state managed by Tauri. Backends are trait objects so engines
//! and platform adapters swap without touching the dictation controller or command layer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;

use crate::asr::{CancelToken, SpeechBackend, WhisperBackend};
use crate::audio::{AudioCapture, CpalCapture};
use crate::insert::TextInserter;
use crate::models::{self, ModelRuntimeFacts};
use crate::session::Session;
use crate::storage::{db::file_stamp, Settings, Store};

pub struct AppState {
    pub session: Mutex<Session>,
    /// In-memory copy of the persisted settings (written through to `store` on every change).
    pub settings: Mutex<Settings>,
    pub store: Store,
    /// False when the database could not be opened and we fell back to memory (shown in the UI).
    pub storage_persistent: bool,
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
    pub fn new(default_model_dir: PathBuf, inserter: Box<dyn TextInserter>, db_path: Option<PathBuf>) -> Self {
        let (store, storage_persistent) = match db_path.as_deref().map(Store::open) {
            Some(Ok(s)) => (s, true),
            other => {
                if let Some(Err(e)) = other {
                    eprintln!("[voxelle] database unavailable, using memory only: {e}");
                }
                (Store::open_in_memory().expect("in-memory sqlite"), false)
            }
        };
        let settings = store.load_settings().ok().flatten().unwrap_or_default();
        let model_dir = settings.model_dir.clone().map(PathBuf::from).unwrap_or_else(|| default_model_dir.clone());

        // A past SHA-256 check still counts only if the file is byte-for-byte the same size and
        // has not been modified since; otherwise it is re-verified before the next load.
        let mut facts = HashMap::new();
        for (id, stamp, mut f) in store.load_model_facts().unwrap_or_default() {
            if let Some(entry) = models::catalog_entry(&id) {
                f.integrity_verified = stamp.is_some() && stamp == file_stamp(&models::model_path(&model_dir, entry));
            }
            facts.insert(id, f);
        }

        Self {
            session: Mutex::new(Session::new()),
            settings: Mutex::new(settings),
            store,
            storage_persistent,
            default_model_dir,
            model_facts: Mutex::new(facts),
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

    /// Update one model's facts in memory and persist them (with the file stamp if verified).
    pub fn update_facts(&self, id: &str, f: impl FnOnce(&mut ModelRuntimeFacts)) {
        let snapshot = {
            let mut all = self.model_facts.lock().unwrap();
            let entry = all.entry(id.to_string()).or_default();
            f(entry);
            entry.clone()
        };
        let stamp = if snapshot.integrity_verified {
            models::catalog_entry(id).and_then(|e| file_stamp(&models::model_path(&self.model_dir(), e)))
        } else {
            None
        };
        if let Err(e) = self.store.save_model_facts(id, stamp, &snapshot) {
            eprintln!("[voxelle] could not save model facts: {e}");
        }
    }

    pub fn save_settings(&self, s: &Settings) -> Result<(), String> {
        self.store.save_settings(s).map_err(|e| e.to_string())
    }

    /// Apply the retention setting (no-op when "keep forever").
    pub fn apply_retention(&self) {
        let days = self.settings.lock().unwrap().retention_days;
        if let Some(d) = days {
            let _ = self.store.purge_older_than(d, chrono::Utc::now());
        }
    }
}
