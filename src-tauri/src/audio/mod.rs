//! Audio capture boundary (P0 §0.1). `CpalCapture` is the real implementation; the DSP helpers
//! here (downmix, level, resample) are pure so they are unit-tested without a microphone.

mod cpal_capture;

pub use cpal_capture::CpalCapture;

use serde::Serialize;
use thiserror::Error;

/// Sample rate every speech backend receives (whisper.cpp expects 16 kHz mono f32).
pub const ASR_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("audio capture not available: {0}")]
    Unavailable(String),
    #[error("no input device: {0}")]
    NoDevice(String),
    #[error("microphone access denied — allow microphone access for desktop apps in your OS privacy settings")]
    PermissionDenied,
    #[error("input device disconnected")]
    Disconnected,
    #[error("not recording")]
    NotRecording,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDevice {
    /// Stable identifier used to reselect the device (cpal DeviceId string).
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

/// Capture runs on a dedicated thread; `stop` returns the captured mono PCM (f32, -1..1).
pub trait AudioCapture: Send {
    fn list_input_devices(&self) -> Result<Vec<InputDevice>, AudioError>;
    /// Open the device (by id, or the system default) and start recording.
    fn start(&mut self, device_id: Option<&str>) -> Result<(), AudioError>;
    /// Stop, release the device, and return what was captured.
    fn stop(&mut self) -> Result<CapturedAudio, AudioError>;
    /// Release the device without returning audio (cancel/error/exit path).
    fn abort(&mut self);
    /// Most recent input level, 0..1 (perceptual dB scale).
    fn level(&self) -> f32;
    fn is_capturing(&self) -> bool;
    /// True once the open device reported it went away mid-recording.
    fn disconnected(&self) -> bool;
    /// Milliseconds from the start of the current recording until the device delivered its first
    /// samples; `None` while it has delivered nothing (e.g. a Bluetooth headset still switching).
    fn first_audio_ms(&self) -> Option<u64>;
    /// Keep the device open this long after a recording so the next one starts instantly.
    /// `Duration::ZERO` releases the mic right after each recording; `Duration::MAX` = until exit.
    fn set_keep_open(&mut self, keep: std::time::Duration);
    /// True while the device is open (recording, or kept ready between recordings).
    fn is_open(&self) -> bool;
}

pub struct CapturedAudio {
    pub pcm: Vec<f32>,
    pub sample_rate: u32,
}

impl CapturedAudio {
    pub fn duration_ms(&self) -> u64 {
        (self.pcm.len() as u64 * 1000) / (self.sample_rate.max(1) as u64)
    }

    /// Mono 16 kHz copy for the speech backend.
    pub fn to_asr_rate(&self) -> Vec<f32> {
        resample(&self.pcm, self.sample_rate, ASR_SAMPLE_RATE)
    }
}

/// Average interleaved frames down to mono.
pub fn downmix_into(interleaved: &[f32], channels: usize, out: &mut Vec<f32>) {
    if channels <= 1 {
        out.extend_from_slice(interleaved);
        return;
    }
    let inv = 1.0 / channels as f32;
    out.extend(
        interleaved
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() * inv),
    );
}

/// RMS mapped onto a -60 dBFS..0 dBFS scale, so quiet speech still moves the meter.
pub fn level_from_samples(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mean_sq = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = mean_sq.sqrt();
    if rms <= 1e-6 {
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    ((db + 60.0) / 60.0).clamp(0.0, 1.0)
}

/// Loudest 30 ms frame, in dBFS. Speech sits roughly -35..-10 dBFS; a quiet room's noise floor
/// is usually below -50.
pub fn peak_frame_dbfs(pcm: &[f32], sample_rate: u32) -> f32 {
    let frame = ((sample_rate as usize * 30) / 1000).max(1);
    let best = pcm
        .chunks(frame)
        .map(|c| c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32)
        .fold(0.0f32, f32::max)
        .sqrt();
    if best <= 1e-9 {
        -120.0
    } else {
        20.0 * best.log10()
    }
}

/// Below this the clip almost certainly holds no voice; skipping ASR avoids hallucinated text.
pub const SPEECH_FLOOR_DBFS: f32 = -45.0;

/// Band-limited (Blackman-windowed sinc) resampler. Offline, whole-buffer: dictation clips are
/// short, so clarity beats streaming complexity. Weights are normalized per output sample so DC
/// gain stays 1 at the clip edges.
pub fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() || from_rate == 0 || to_rate == 0 {
        return input.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    // Cut off a bit below the lower Nyquist to leave room for the window's transition band.
    let cutoff = ratio.min(1.0) * 0.92;
    let half_width = (16.0 / cutoff).ceil() as i64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let last = input.len() as i64 - 1;

    let mut out = Vec::with_capacity(out_len);
    for n in 0..out_len {
        let t = n as f64 / ratio; // position in input samples
        let center = t.floor() as i64;
        let mut acc = 0.0f64;
        let mut wsum = 0.0f64;
        for k in (center - half_width + 1)..=(center + half_width) {
            if k < 0 || k > last {
                continue;
            }
            let x = t - k as f64;
            let w = sinc(cutoff * x) * blackman(x / half_width as f64);
            acc += input[k as usize] as f64 * w;
            wsum += w;
        }
        out.push(if wsum.abs() > 1e-12 { (acc / wsum) as f32 } else { 0.0 });
    }
    out
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Blackman window over x in [-1, 1]; zero outside.
fn blackman(x: f64) -> f64 {
    if x.abs() >= 1.0 {
        return 0.0;
    }
    let a = std::f64::consts::PI * (x + 1.0); // maps [-1,1] -> [0, 2π]
    0.42 - 0.5 * a.cos() + 0.08 * (2.0 * a).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, secs: f32, amp: f32) -> Vec<f32> {
        let n = (rate as f32 * secs) as usize;
        (0..n)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt()
    }

    fn zero_crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count()
    }

    #[test]
    fn downmix_averages_channels() {
        let mut out = Vec::new();
        downmix_into(&[1.0, 0.0, 0.5, 0.5, -1.0, 1.0], 2, &mut out);
        assert_eq!(out, vec![0.5, 0.5, 0.0]);
        let mut mono = Vec::new();
        downmix_into(&[0.1, 0.2], 1, &mut mono);
        assert_eq!(mono, vec![0.1, 0.2]);
    }

    #[test]
    fn level_is_zero_for_silence_and_rises_with_loudness() {
        assert_eq!(level_from_samples(&[]), 0.0);
        assert_eq!(level_from_samples(&[0.0; 512]), 0.0);
        let quiet = level_from_samples(&sine(440.0, 16_000, 0.05, 0.01));
        let loud = level_from_samples(&sine(440.0, 16_000, 0.05, 0.5));
        assert!(quiet > 0.0 && quiet < loud && loud <= 1.0, "{quiet} {loud}");
    }

    #[test]
    fn resample_identity_when_rates_match() {
        let x = sine(440.0, 16_000, 0.1, 0.5);
        assert_eq!(resample(&x, 16_000, 16_000), x);
    }

    #[test]
    fn resample_48k_to_16k_keeps_length_frequency_and_amplitude() {
        let x = sine(1000.0, 48_000, 1.0, 0.5);
        let y = resample(&x, 48_000, 16_000);
        assert_eq!(y.len(), 16_000);
        // 1 kHz for 1 s => ~2000 zero crossings regardless of rate.
        let zc = zero_crossings(&y) as i64;
        assert!((zc - 2000).abs() <= 2, "zero crossings {zc}");
        let r = rms(&y[200..y.len() - 200]);
        assert!((r - 0.5 / 2f32.sqrt()).abs() < 0.01, "rms {r}");
    }

    #[test]
    fn resample_44100_to_16k_handles_non_integer_ratio() {
        let x = sine(500.0, 44_100, 0.5, 0.5);
        let y = resample(&x, 44_100, 16_000);
        assert_eq!(y.len(), 8_000);
        let zc = zero_crossings(&y) as i64;
        assert!((zc - 500).abs() <= 2, "zero crossings {zc}");
    }

    #[test]
    fn resample_rejects_content_above_new_nyquist() {
        // 12 kHz cannot be represented at 16 kHz; it must be filtered, not aliased to 4 kHz.
        let x = sine(12_000.0, 48_000, 0.5, 0.5);
        let y = resample(&x, 48_000, 16_000);
        let r = rms(&y[200..y.len() - 200]);
        assert!(r < 0.01, "aliased energy rms {r}");
    }

    #[test]
    fn resample_keeps_dc_gain_at_edges() {
        let y = resample(&vec![0.25; 4800], 48_000, 16_000);
        assert!(y.iter().all(|v| (v - 0.25).abs() < 1e-3));
    }

    #[test]
    fn peak_frame_level_separates_noise_floor_from_speech() {
        let noise: Vec<f32> = (0..16_000).map(|i| if i % 2 == 0 { 0.001 } else { -0.001 }).collect();
        assert!(peak_frame_dbfs(&noise, 16_000) < SPEECH_FLOOR_DBFS);
        let mut speechy = noise.clone();
        speechy.extend(sine(300.0, 16_000, 0.2, 0.1));
        assert!(peak_frame_dbfs(&speechy, 16_000) > SPEECH_FLOOR_DBFS);
        assert_eq!(peak_frame_dbfs(&[0.0; 100], 16_000), -120.0);
    }

    #[test]
    fn duration_is_derived_from_rate() {
        let c = CapturedAudio {
            pcm: vec![0.0; 24_000],
            sample_rate: 48_000,
        };
        assert_eq!(c.duration_ms(), 500);
    }
}
