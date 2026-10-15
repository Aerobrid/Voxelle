//! Settings + history types and their SQLite persistence (`db`). The raw transcript is always kept
//! separate from any edited text.

pub mod db;

pub use db::Store;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: String,
    pub punctuation: bool,
    pub auto_capitalize: bool,
    pub remove_filler_words: bool,
    pub store_transcripts: bool,
    pub retention_days: Option<u32>,
    pub prefer_gpu: bool,
    /// cpal device id; None = system default microphone.
    pub input_device_id: Option<String>,
    /// Keep the mic open this many seconds after a dictation so the next one starts instantly
    /// (Bluetooth headsets otherwise drop the first ~1-3 s). 0 = off; u32::MAX = until quit.
    pub keep_mic_ready_secs: u32,
    /// Closing the window keeps Voxelle running in the tray (shortcuts keep working).
    pub close_to_tray: bool,
    pub model_dir: Option<String>,
    pub selected_speech_model_id: Option<String>,
    pub toggle_hotkey: String,
    pub push_to_talk_hotkey: Option<String>,
    pub cancel_hotkey: Option<String>,
    pub offline_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            punctuation: true,
            auto_capitalize: true,
            remove_filler_words: false, // cleanup off by default
            store_transcripts: true,
            retention_days: None, // keep forever until the user chooses
            // GPU when this build has it: on the dev machine Vulkan was 12-27x faster (decisions.md).
            prefer_gpu: crate::asr::GPU_COMPILED,
            input_device_id: None,
            keep_mic_ready_secs: 0, // privacy: mic released after every dictation by default
            close_to_tray: true,
            model_dir: None,
            selected_speech_model_id: None,
            toggle_hotkey: "Ctrl+Shift+Space".into(),
            push_to_talk_hotkey: Some("RightCtrl".into()),
            cancel_hotkey: Some("Escape".into()),
            offline_mode: true, // offline by default
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at: String, // ISO-8601
    pub raw_transcript: String,
    pub edited_text: Option<String>,
    pub duration_ms: u64,
    pub language: Option<String>,
}

impl Settings {
    /// How long capture should keep the device open after a recording.
    pub fn keep_mic_ready(&self) -> std::time::Duration {
        match self.keep_mic_ready_secs {
            0 => std::time::Duration::ZERO,
            u32::MAX => std::time::Duration::MAX,
            s => std::time::Duration::from_secs(s as u64),
        }
    }
}
