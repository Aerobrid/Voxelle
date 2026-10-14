//! Audio capture boundary. The cpal implementation lands in the P0 audio slice (compiled + tested
//! with the toolchain present); until then the stub reports honestly that it is not wired.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("audio capture not available: {0}")]
    Unavailable(String),
    #[error("no input device: {0}")]
    NoDevice(String),
    #[error("microphone permission denied")]
    PermissionDenied,
    #[error("input device disconnected")]
    Disconnected,
}

/// Capture runs on a dedicated thread; `stop` returns the captured mono PCM (f32, -1..1).
pub trait AudioCapture: Send {
    fn list_input_devices(&self) -> Result<Vec<String>, AudioError>;
    fn start(&mut self, device: Option<&str>) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<CapturedAudio, AudioError>;
    /// Release the device without returning audio (cancel/error/exit path).
    fn abort(&mut self);
    fn level(&self) -> f32;
    fn is_capturing(&self) -> bool;
}

pub struct CapturedAudio {
    pub pcm: Vec<f32>,
    pub sample_rate: u32,
}

/// Placeholder until cpal is wired. Never fabricates audio.
pub struct StubAudioCapture;

impl AudioCapture for StubAudioCapture {
    fn list_input_devices(&self) -> Result<Vec<String>, AudioError> {
        Err(AudioError::Unavailable(
            "cpal capture not yet wired (P0 audio slice)".into(),
        ))
    }
    fn start(&mut self, _device: Option<&str>) -> Result<(), AudioError> {
        Err(AudioError::Unavailable(
            "cpal capture not yet wired (P0 audio slice)".into(),
        ))
    }
    fn stop(&mut self) -> Result<CapturedAudio, AudioError> {
        Err(AudioError::Unavailable(
            "cpal capture not yet wired (P0 audio slice)".into(),
        ))
    }
    fn abort(&mut self) {}
    fn level(&self) -> f32 {
        0.0
    }
    fn is_capturing(&self) -> bool {
        false
    }
}
