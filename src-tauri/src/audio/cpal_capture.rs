//! Microphone capture via cpal. The cpal `Stream` is not `Send` on every platform, so it lives on a
//! dedicated worker thread for the length of one recording; dropping it there releases the mic.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample};

use super::{downmix_into, level_from_samples, AudioCapture, AudioError, CapturedAudio, InputDevice};

/// Hard cap on one recording so a stuck hotkey cannot grow memory without bound.
const MAX_RECORDING_SECS: u64 = 15 * 60;

#[derive(Default)]
struct Shared {
    buffer: Mutex<Vec<f32>>,
    level_bits: AtomicU32,
    disconnected: AtomicBool,
}

struct Active {
    shared: Arc<Shared>,
    sample_rate: u32,
    stop_tx: mpsc::Sender<()>,
    worker: JoinHandle<()>,
}

#[derive(Default)]
pub struct CpalCapture {
    active: Option<Active>,
}

impl CpalCapture {
    pub fn new() -> Self {
        Self::default()
    }

    fn finish(&mut self) -> Option<(Arc<Shared>, u32)> {
        let active = self.active.take()?;
        let _ = active.stop_tx.send(());
        let _ = active.worker.join(); // stream dropped => device released
        Some((active.shared, active.sample_rate))
    }
}

fn map_err(e: cpal::Error) -> AudioError {
    match e.kind() {
        ErrorKind::PermissionDenied => AudioError::PermissionDenied,
        ErrorKind::DeviceNotAvailable => AudioError::NoDevice(e.to_string()),
        _ => AudioError::Unavailable(e.to_string()),
    }
}

fn pick_device(device_id: Option<&str>) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    match device_id {
        Some(id) => {
            let parsed = id
                .parse()
                .map_err(|_| AudioError::NoDevice(format!("invalid device id: {id}")))?;
            host.device_by_id(&parsed)
                .ok_or_else(|| AudioError::NoDevice(format!("device not found: {id}")))
        }
        None => host
            .default_input_device()
            .ok_or_else(|| AudioError::NoDevice("no default microphone".into())),
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let max_samples = (config.sample_rate as u64 * MAX_RECORDING_SECS) as usize;
    let err_shared = shared.clone();
    let mut scratch: Vec<f32> = Vec::new();
    let mut mono: Vec<f32> = Vec::new();
    device.build_input_stream(
        config.clone(),
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            scratch.clear();
            scratch.extend(data.iter().map(|s| s.to_sample::<f32>()));
            mono.clear();
            downmix_into(&scratch, channels, &mut mono);
            shared
                .level_bits
                .store(level_from_samples(&mono).to_bits(), Ordering::Relaxed);
            // try_lock: never block the realtime callback; stop() only locks after the stream ends.
            if let Ok(mut buf) = shared.buffer.try_lock() {
                let room = max_samples.saturating_sub(buf.len());
                buf.extend_from_slice(&mono[..mono.len().min(room)]);
            }
        },
        move |err: cpal::Error| {
            if matches!(
                err.kind(),
                ErrorKind::DeviceNotAvailable | ErrorKind::StreamInvalidated
            ) {
                err_shared.disconnected.store(true, Ordering::Relaxed);
            }
        },
        None,
    )
}

impl AudioCapture for CpalCapture {
    fn list_input_devices(&self) -> Result<Vec<InputDevice>, AudioError> {
        let host = cpal::default_host();
        let default_id = host
            .default_input_device()
            .and_then(|d| d.id().ok())
            .map(|id| id.to_string());
        let devices = host.input_devices().map_err(map_err)?;
        Ok(devices
            .filter_map(|d| {
                let id = d.id().ok()?.to_string();
                let name = d
                    .description()
                    .map(|desc| desc.name().to_string())
                    .unwrap_or_else(|_| id.clone());
                Some(InputDevice {
                    is_default: default_id.as_deref() == Some(id.as_str()),
                    id,
                    name,
                })
            })
            .collect())
    }

    fn start(&mut self, device_id: Option<&str>) -> Result<(), AudioError> {
        if self.active.is_some() {
            return Err(AudioError::Unavailable("already recording".into()));
        }
        let device = pick_device(device_id)?;
        let supported = device.default_input_config().map_err(map_err)?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.config();
        let sample_rate = config.sample_rate;
        let shared = Arc::new(Shared::default());

        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), AudioError>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new()
            .name("voxelle-audio".into())
            .spawn(move || {
                let stream = match format {
                    SampleFormat::F32 => build_stream::<f32>(&device, &config, worker_shared),
                    SampleFormat::I16 => build_stream::<i16>(&device, &config, worker_shared),
                    SampleFormat::U16 => build_stream::<u16>(&device, &config, worker_shared),
                    SampleFormat::I32 => build_stream::<i32>(&device, &config, worker_shared),
                    SampleFormat::I8 => build_stream::<i8>(&device, &config, worker_shared),
                    SampleFormat::U8 => build_stream::<u8>(&device, &config, worker_shared),
                    SampleFormat::F64 => build_stream::<f64>(&device, &config, worker_shared),
                    other => {
                        let _ = ready_tx.send(Err(AudioError::Unavailable(format!(
                            "unsupported sample format {other}"
                        ))));
                        return;
                    }
                };
                let stream = match stream.and_then(|s| s.play().map(|_| s)) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = ready_tx.send(Err(map_err(e)));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                // Park until stop/abort (or the owner is dropped), then drop the stream.
                let _ = stop_rx.recv();
                drop(stream);
            })
            .map_err(|e| AudioError::Unavailable(format!("spawn audio thread: {e}")))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.active = Some(Active {
                    shared,
                    sample_rate,
                    stop_tx,
                    worker,
                });
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = worker.join();
                Err(e)
            }
            Err(_) => {
                let _ = stop_tx.send(());
                Err(AudioError::Unavailable(
                    "microphone did not start within 5 s".into(),
                ))
            }
        }
    }

    fn stop(&mut self) -> Result<CapturedAudio, AudioError> {
        let (shared, sample_rate) = self.finish().ok_or(AudioError::NotRecording)?;
        let pcm = std::mem::take(&mut *shared.buffer.lock().unwrap());
        if pcm.is_empty() && shared.disconnected.load(Ordering::Relaxed) {
            return Err(AudioError::Disconnected);
        }
        Ok(CapturedAudio { pcm, sample_rate })
    }

    fn abort(&mut self) {
        self.finish();
    }

    fn level(&self) -> f32 {
        self.active
            .as_ref()
            .map(|a| f32::from_bits(a.shared.level_bits.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }

    fn is_capturing(&self) -> bool {
        self.active.is_some()
    }

    fn disconnected(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.shared.disconnected.load(Ordering::Relaxed))
    }
}

impl Drop for CpalCapture {
    fn drop(&mut self) {
        self.finish(); // release the mic on app exit
    }
}
