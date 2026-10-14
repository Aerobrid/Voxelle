//! Shared, thread-safe application state managed by Tauri. Backends are trait objects so real
//! implementations (cpal audio, whisper ASR, platform inserter) swap in per slice without changing
//! the command layer. In-memory settings/history move to SQLite in the storage slice.

use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use crate::asr::{NullSpeechBackend, SpeechBackend};
use crate::audio::{AudioCapture, StubAudioCapture};
use crate::insert::{StubInserter, TextInserter};
use crate::models::{default_catalog, ModelInfo};
use crate::session::Session;
use crate::storage::{HistoryEntry, Settings};

pub struct AppState {
    pub session: Mutex<Session>,
    pub settings: Mutex<Settings>,
    pub history: Mutex<Vec<HistoryEntry>>,
    pub models: Mutex<Vec<ModelInfo>>,
    pub audio: Mutex<Box<dyn AudioCapture>>,
    pub asr: Mutex<Box<dyn SpeechBackend>>,
    pub inserter: Mutex<Box<dyn TextInserter>>,
    pub id_counter: AtomicU64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(Session::new()),
            settings: Mutex::new(Settings::default()),
            history: Mutex::new(Vec::new()),
            models: Mutex::new(default_catalog()),
            audio: Mutex::new(Box::new(StubAudioCapture)),
            asr: Mutex::new(Box::new(NullSpeechBackend)),
            inserter: Mutex::new(Box::new(StubInserter)),
            id_counter: AtomicU64::new(1),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
