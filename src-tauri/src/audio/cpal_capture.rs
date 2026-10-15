//! Microphone capture via cpal, run by one process-lifetime audio thread.
//!
//! Why one thread: cpal's WASAPI backend creates a single global device enumerator (a COM object)
//! on whichever thread first touches audio, inside that thread's COM apartment. If that thread
//! exits, the apartment dies with it and later recordings fail with RPC_E_CHANGED_MODE ("Cannot
//! change thread mode after it is set"). So every cpal call (enumeration, device lookup, stream)
//! runs on `voxelle-audio`, which joins the multithreaded apartment before cpal runs and never
//! exits. (cpal then gets RPC_E_CHANGED_MODE from its own STA attempt, which it accepts.)
//!
//! The stream can optionally stay open for a while after a recording ("keep the mic ready"), so the
//! next dictation starts instantly. Bluetooth headsets need this: opening their mic switches them to
//! the hands-free profile and they deliver no audio for ~0.5–3 s. Between recordings the callback
//! drops samples; nothing is buffered unless a recording is active.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample};

use super::{downmix_into, level_from_samples, AudioCapture, AudioError, CapturedAudio, InputDevice};

/// Hard cap on one recording so a stuck hotkey cannot grow memory without bound.
const MAX_RECORDING_SECS: u64 = 15 * 60;
/// How long callers wait for the audio thread to open/close a device.
const DEVICE_TIMEOUT: Duration = Duration::from_secs(5);
/// `u64` sentinel for "no audio callback yet".
const NO_AUDIO: u64 = u64::MAX;

/// State shared between the realtime callback, the audio thread, and `CpalCapture`.
struct Shared {
    buffer: Mutex<Vec<f32>>,
    /// True while a recording is active; the callback only buffers then.
    recording: AtomicBool,
    level_bits: AtomicU32,
    disconnected: AtomicBool,
    /// Set (under this lock) once the stream is gone, so `start` never reuses a closed stream.
    closed: Mutex<bool>,
    /// Reference point for the two `_ms` timestamps below.
    epoch: Instant,
    recording_started_ms: AtomicU64,
    first_audio_ms: AtomicU64,
}

impl Shared {
    fn new() -> Self {
        Self {
            buffer: Mutex::new(Vec::new()),
            recording: AtomicBool::new(false),
            level_bits: AtomicU32::new(0),
            disconnected: AtomicBool::new(false),
            closed: Mutex::new(false),
            epoch: Instant::now(),
            recording_started_ms: AtomicU64::new(0),
            first_audio_ms: AtomicU64::new(NO_AUDIO),
        }
    }

    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    fn begin_recording(&self) {
        self.buffer.lock().unwrap().clear();
        self.first_audio_ms.store(NO_AUDIO, Ordering::SeqCst);
        self.recording_started_ms.store(self.now_ms(), Ordering::SeqCst);
        self.recording.store(true, Ordering::SeqCst);
    }
}

// ---------- the audio thread ----------

enum Cmd {
    List(mpsc::Sender<Result<Vec<InputDevice>, AudioError>>),
    /// Close any open stream, then open `device_id` (None = system default) recording into `shared`.
    Open {
        device_id: Option<String>,
        shared: Arc<Shared>,
        reply: mpsc::Sender<Result<u32, AudioError>>,
    },
    /// A recording ended; close after this long without a new one (None = keep open).
    Idle(Option<Duration>),
    /// A recording started on the open stream; cancel any pending close.
    Busy,
    /// Close the open stream (if any); acknowledge once the device is released.
    Close(Option<mpsc::Sender<()>>),
}

fn service() -> &'static mpsc::Sender<Cmd> {
    static SERVICE: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
    SERVICE.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("voxelle-audio".into())
            .spawn(move || run_service(rx))
            .expect("spawn audio thread");
        tx
    })
}

/// Send a command and wait for its reply.
fn request<T>(make: impl FnOnce(mpsc::Sender<T>) -> Cmd) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    service().send(make(tx)).ok()?;
    rx.recv_timeout(DEVICE_TIMEOUT).ok()
}

#[cfg(windows)]
fn join_multithreaded_apartment() {
    use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: called once at the start of the audio thread, before any COM use on it; balanced by
    // thread exit (the thread lives for the whole process).
    let hr = unsafe { CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32) };
    if hr < 0 {
        eprintln!("[voxelle] audio thread: CoInitializeEx failed (0x{hr:08x})");
    }
}

fn run_service(rx: mpsc::Receiver<Cmd>) {
    #[cfg(windows)]
    join_multithreaded_apartment();

    let mut current: Option<(cpal::Stream, Arc<Shared>)> = None;
    let mut close_at: Option<Instant> = None;
    let close = |current: &mut Option<(cpal::Stream, Arc<Shared>)>| {
        if let Some((stream, shared)) = current.take() {
            *shared.closed.lock().unwrap() = true;
            drop(stream); // releases the device
        }
    };
    loop {
        let cmd = match close_at {
            Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match cmd {
            Ok(Cmd::List(reply)) => {
                let _ = reply.send(list_devices());
            }
            Ok(Cmd::Open { device_id, shared, reply }) => {
                close(&mut current);
                close_at = None;
                match open_stream(device_id.as_deref(), &shared) {
                    Ok((stream, rate)) => {
                        current = Some((stream, shared));
                        let _ = reply.send(Ok(rate));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Ok(Cmd::Idle(linger)) => close_at = linger.map(|d| Instant::now() + d),
            Ok(Cmd::Busy) => close_at = None,
            Ok(Cmd::Close(ack)) => {
                close(&mut current);
                close_at = None;
                if let Some(ack) = ack {
                    let _ = ack.send(());
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                close_at = None;
                let idle = current.as_ref().is_some_and(|(_, shared)| {
                    let mut closed = shared.closed.lock().unwrap();
                    // A recording may have claimed the stream just before the deadline.
                    let idle = !shared.recording.load(Ordering::SeqCst);
                    if idle {
                        *closed = true;
                    }
                    idle
                });
                if idle {
                    close(&mut current);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    close(&mut current);
}

fn open_stream(device_id: Option<&str>, shared: &Arc<Shared>) -> Result<(cpal::Stream, u32), AudioError> {
    let device = pick_device(device_id)?;
    let supported = device.default_input_config().map_err(map_err)?;
    let config: cpal::StreamConfig = supported.config();
    let s = shared.clone();
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, s),
        SampleFormat::I16 => build_stream::<i16>(&device, &config, s),
        SampleFormat::U16 => build_stream::<u16>(&device, &config, s),
        SampleFormat::I32 => build_stream::<i32>(&device, &config, s),
        SampleFormat::I8 => build_stream::<i8>(&device, &config, s),
        SampleFormat::U8 => build_stream::<u8>(&device, &config, s),
        SampleFormat::F64 => build_stream::<f64>(&device, &config, s),
        other => return Err(AudioError::Unavailable(format!("unsupported sample format {other}"))),
    }
    .map_err(map_err)?;
    shared.begin_recording(); // before play(): the first callback already belongs to the recording
    stream.play().map_err(map_err)?;
    Ok((stream, config.sample_rate))
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
        None => {
            let default = host
                .default_input_device()
                .ok_or_else(|| AudioError::NoDevice("no default microphone".into()))?;
            // Resolve "the default" to the concrete endpoint. cpal opens its virtual default device
            // through ActivateAudioInterfaceAsync, which on the dev machine started failing with
            // RPC_E_CHANGED_MODE after a few recordings and then on every one; the concrete
            // endpoint goes through IMMDevice::Activate, which does not.
            Ok(default
                .id()
                .ok()
                .and_then(|id| host.device_by_id(&id))
                .unwrap_or(default))
        }
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
            if !shared.recording.load(Ordering::Relaxed) {
                return; // between recordings: drop samples, keep nothing
            }
            scratch.clear();
            scratch.extend(data.iter().map(|s| s.to_sample::<f32>()));
            mono.clear();
            downmix_into(&scratch, channels, &mut mono);
            if !mono.is_empty() && shared.first_audio_ms.load(Ordering::Relaxed) == NO_AUDIO {
                shared.first_audio_ms.store(shared.now_ms(), Ordering::Relaxed);
            }
            shared
                .level_bits
                .store(level_from_samples(&mono).to_bits(), Ordering::Relaxed);
            // try_lock: never block the realtime callback for long.
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

fn list_devices() -> Result<Vec<InputDevice>, AudioError> {
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

// ---------- the AudioCapture handle ----------

struct Active {
    shared: Arc<Shared>,
    sample_rate: u32,
    device_id: Option<String>,
}

pub struct CpalCapture {
    active: Option<Active>,
    /// How long to keep the stream open after a recording. Zero = release the mic immediately.
    keep_open: Duration,
}

impl Default for CpalCapture {
    fn default() -> Self {
        Self { active: None, keep_open: Duration::ZERO }
    }
}

impl CpalCapture {
    pub fn new() -> Self {
        Self::default()
    }

    /// Close the stream (if any) and wait for the device to be released.
    fn close(&mut self) {
        if self.active.take().is_some() {
            let _ = request(|ack| Cmd::Close(Some(ack)));
        }
    }

    /// Try to start a recording on the already-open stream for `device_id`.
    fn reuse(&mut self, device_id: Option<&str>) -> bool {
        let Some(active) = &self.active else { return false };
        if active.device_id.as_deref() != device_id || active.shared.disconnected.load(Ordering::SeqCst) {
            return false;
        }
        // Holding `closed` blocks the audio thread's idle-timeout close until we have claimed it.
        let closed = active.shared.closed.lock().unwrap();
        if *closed || service().send(Cmd::Busy).is_err() {
            return false;
        }
        active.shared.begin_recording();
        true
    }

    fn open(&mut self, device_id: Option<&str>) -> Result<(), AudioError> {
        let shared = Arc::new(Shared::new());
        let reply = request(|reply| Cmd::Open {
            device_id: device_id.map(str::to_owned),
            shared: shared.clone(),
            reply,
        });
        match reply {
            Some(Ok(sample_rate)) => {
                self.active = Some(Active { shared, sample_rate, device_id: device_id.map(str::to_owned) });
                Ok(())
            }
            Some(Err(e)) => Err(e),
            None => {
                // Queue a close so a late-opening device is released rather than left open.
                let _ = service().send(Cmd::Close(None));
                Err(AudioError::Unavailable("microphone did not start within 5 s".into()))
            }
        }
    }

    /// End the current recording and hand back its samples. The stream stays open per
    /// `keep_open` when `keep` is set, else the mic is released before returning.
    fn end_recording(&mut self, keep: bool) -> Option<(Vec<f32>, u32, bool)> {
        let active = self.active.as_ref()?;
        if !active.shared.recording.swap(false, Ordering::SeqCst) {
            return None;
        }
        let pcm = std::mem::take(&mut *active.shared.buffer.lock().unwrap());
        let disconnected = active.shared.disconnected.load(Ordering::SeqCst);
        let sample_rate = active.sample_rate;
        if keep && !self.keep_open.is_zero() && !disconnected {
            let linger = (self.keep_open != Duration::MAX).then_some(self.keep_open);
            if service().send(Cmd::Idle(linger)).is_err() {
                self.close();
            }
        } else {
            self.close();
        }
        Some((pcm, sample_rate, disconnected))
    }
}

impl AudioCapture for CpalCapture {
    fn list_input_devices(&self) -> Result<Vec<InputDevice>, AudioError> {
        request(Cmd::List).unwrap_or_else(|| Err(AudioError::Unavailable("audio thread did not respond".into())))
    }

    fn start(&mut self, device_id: Option<&str>) -> Result<(), AudioError> {
        if self.is_capturing() {
            return Err(AudioError::Unavailable("already recording".into()));
        }
        if self.reuse(device_id) {
            return Ok(());
        }
        self.close(); // different device, disconnected, or idle-closed
        self.open(device_id)
    }

    fn stop(&mut self) -> Result<CapturedAudio, AudioError> {
        let (pcm, sample_rate, disconnected) = self.end_recording(true).ok_or(AudioError::NotRecording)?;
        if pcm.is_empty() && disconnected {
            return Err(AudioError::Disconnected);
        }
        Ok(CapturedAudio { pcm, sample_rate })
    }

    fn abort(&mut self) {
        // Cancel/error: always release the mic, even with "keep ready" on.
        self.end_recording(false);
        self.close();
    }

    fn level(&self) -> f32 {
        self.active
            .as_ref()
            .filter(|a| a.shared.recording.load(Ordering::Relaxed))
            .map(|a| f32::from_bits(a.shared.level_bits.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }

    fn is_capturing(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.shared.recording.load(Ordering::SeqCst))
    }

    fn disconnected(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| a.shared.disconnected.load(Ordering::Relaxed))
    }

    fn first_audio_ms(&self) -> Option<u64> {
        let s = &self.active.as_ref()?.shared;
        let first = s.first_audio_ms.load(Ordering::Relaxed);
        (first != NO_AUDIO).then(|| first.saturating_sub(s.recording_started_ms.load(Ordering::Relaxed)))
    }

    fn set_keep_open(&mut self, keep: Duration) {
        self.keep_open = keep;
        if keep.is_zero() && !self.is_capturing() {
            self.close(); // turning the option off releases an idle mic right away
        }
    }

    fn is_open(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|a| !*a.shared.closed.lock().unwrap())
    }
}

impl Drop for CpalCapture {
    fn drop(&mut self) {
        self.close(); // release the mic on app exit
    }
}
