//! Speech-recognition boundary. Engine-agnostic so whisper.cpp is just the first backend
//! (P0 §0.2). Backends receive 16 kHz mono f32 and never touch the network or any tool.

pub mod wer;
mod whisper;

pub use whisper::{default_threads, WhisperBackend, GPU_BACKEND, GPU_COMPILED};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AsrError {
    #[error("no speech model loaded — install or select a model in Model Manager")]
    NoModel,
    #[error("failed to load speech model: {0}")]
    LoadFailed(String),
    #[error("transcription cancelled")]
    Cancelled,
    #[error("asr failure: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// GPU acceleration compiled into this build (e.g. Vulkan).
    pub gpu_compiled: bool,
    /// GPU API compiled in ("Vulkan" / "CUDA"), if any.
    pub gpu_backend: Option<&'static str>,
    /// GPU is actually in use for the loaded model (false after a CPU fallback).
    pub gpu_active: bool,
    pub streaming: bool,
    pub multilingual: bool,
}

#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub use_gpu: bool,
    pub threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transcription {
    pub text: String,
    /// Language the model used (detected when the request was "auto").
    pub language: Option<String>,
}

/// Shared cancel flag. Cloned into the backend's abort hook so cancel never needs the backend lock.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    /// Raw pointer to the flag for native abort hooks. Valid while any clone of the token lives.
    pub(crate) fn flag_ptr(&self) -> *const AtomicBool {
        Arc::as_ptr(&self.0)
    }
}

/// load → transcribe (cancellable via token) → unload, plus capabilities.
/// Implementations are called from worker threads, never the UI thread.
pub trait SpeechBackend: Send {
    fn load(&mut self, model_path: &std::path::Path, opts: &LoadOptions) -> Result<(), AsrError>;
    fn transcribe(
        &mut self,
        pcm_16k: &[f32],
        language: Option<&str>,
        cancel: &CancelToken,
    ) -> Result<Transcription, AsrError>;
    fn unload(&mut self);
    fn capabilities(&self) -> Capabilities;
    /// Path of the loaded model, if any.
    fn loaded_model(&self) -> Option<&std::path::Path>;
}

/// Drop whole segments that are only a non-speech annotation (e.g. "[BLANK_AUDIO]", "(music)"),
/// which whisper emits on silence, then join and tidy whitespace.
pub fn join_segments<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    let kept: Vec<&str> = segments
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !is_annotation(s))
        .collect();
    kept.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_annotation(s: &str) -> bool {
    let wrapped = |open: char, close: char| s.starts_with(open) && s.ends_with(close);
    (wrapped('[', ']') || wrapped('(', ')') || wrapped('*', '*')) && !s[1..s.len() - 1].contains(['[', '(', ']', ')'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_and_normalizes_whitespace() {
        assert_eq!(join_segments([" Hello there.", "  How are  you?"]), "Hello there. How are you?");
    }

    #[test]
    fn drops_non_speech_annotations() {
        assert_eq!(join_segments(["[BLANK_AUDIO]"]), "");
        assert_eq!(join_segments([" (music)", " Okay."]), "Okay.");
        assert_eq!(join_segments(["*silence*"]), "");
    }

    #[test]
    fn keeps_speech_containing_brackets() {
        assert_eq!(join_segments(["Use [brackets] here."]), "Use [brackets] here.");
        assert_eq!(join_segments(["(a) first, (b) second"]), "(a) first, (b) second");
    }

    #[test]
    fn cancel_token_round_trip() {
        let t = CancelToken::default();
        let clone = t.clone();
        assert!(!clone.is_cancelled());
        t.cancel();
        assert!(clone.is_cancelled());
        t.reset();
        assert!(!clone.is_cancelled());
    }
}
