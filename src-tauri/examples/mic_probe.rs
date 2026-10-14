//! Record a few seconds through the app's capture path and transcribe with the app's backend.
//! Nothing is inserted or stored. Measures mic start latency and stop→text latency.
//!
//!   cargo run --release --example mic_probe -- --model <file.bin> [--seconds 3] [--gpu]

use std::time::{Duration, Instant};

use voxelle_core::asr::{CancelToken, LoadOptions, SpeechBackend, WhisperBackend};
use voxelle_core::audio::{peak_frame_dbfs, AudioCapture, CpalCapture};

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut model, mut secs, mut gpu) = (None, 3u64, false);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => model = args.next(),
            "--seconds" => secs = args.next().unwrap().parse().unwrap(),
            "--gpu" => gpu = true,
            other => panic!("unknown arg {other}"),
        }
    }
    let mut cap = CpalCapture::new();
    for d in cap.list_input_devices().expect("list devices") {
        println!("device: {}{}", d.name, if d.is_default { " (default)" } else { "" });
    }
    let mut asr = WhisperBackend::new();
    if let Some(m) = &model {
        let t = Instant::now();
        asr.load(m.as_ref(), &LoadOptions { use_gpu: gpu, threads: voxelle_core::asr::default_threads() })
            .expect("load model");
        println!("model load: {} ms (gpu active: {})", t.elapsed().as_millis(), asr.capabilities().gpu_active);
    }

    let t = Instant::now();
    cap.start(None).expect("start mic");
    println!("mic start latency: {} ms", t.elapsed().as_millis());
    let mut peak_level = 0f32;
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(50));
        peak_level = peak_level.max(cap.level());
    }
    let stop = Instant::now();
    let audio = cap.stop().expect("stop");
    println!(
        "captured {} ms at {} Hz, peak meter {:.2}, loudest frame {:.1} dBFS, mic released: {}",
        audio.duration_ms(),
        audio.sample_rate,
        peak_level,
        peak_frame_dbfs(&audio.pcm, audio.sample_rate),
        !cap.is_capturing()
    );
    if model.is_some() {
        let pcm = audio.to_asr_rate();
        let out = asr.transcribe(&pcm, Some("en"), &CancelToken::default()).expect("transcribe");
        println!("stop→text: {} ms, text: {:?}", stop.elapsed().as_millis(), out.text);
    }
}
