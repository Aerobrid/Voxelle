//! Speech-recognition boundary. Engine-agnostic so whisper.cpp is just the first backend
//! (P0 §0.2). The whisper-rs implementation lands in the ASR slice; the null backend reports
//! honestly that no model is loaded rather than inventing a transcript.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AsrError {
    #[error("no speech model loaded")]
    NoModel,
    #[error("asr backend not available: {0}")]
    Unavailable(String),
    #[error("transcription cancelled")]
    Cancelled,
    #[error("asr failure: {0}")]
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Capabilities {
    pub gpu: bool,
    pub streaming: bool,
    pub languages: Vec<String>,
}

/// load → transcribe → cancel → unload, plus capabilities. Implementations run off the UI thread.
pub trait SpeechBackend: Send {
    fn load(&mut self, model_path: &str) -> Result<(), AsrError>;
    fn transcribe(
        &mut self,
        pcm: &[f32],
        sample_rate: u32,
        language: Option<&str>,
    ) -> Result<String, AsrError>;
    fn cancel(&mut self);
    fn unload(&mut self);
    fn capabilities(&self) -> Capabilities;
    fn is_loaded(&self) -> bool;
}

/// No model loaded. Every call errors honestly.
pub struct NullSpeechBackend;

impl SpeechBackend for NullSpeechBackend {
    fn load(&mut self, _model_path: &str) -> Result<(), AsrError> {
        Err(AsrError::Unavailable(
            "whisper backend not yet wired (P0 asr slice)".into(),
        ))
    }
    fn transcribe(
        &mut self,
        _pcm: &[f32],
        _sample_rate: u32,
        _language: Option<&str>,
    ) -> Result<String, AsrError> {
        Err(AsrError::NoModel)
    }
    fn cancel(&mut self) {}
    fn unload(&mut self) {}
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            gpu: false,
            streaming: false,
            languages: vec![],
        }
    }
    fn is_loaded(&self) -> bool {
        false
    }
}
