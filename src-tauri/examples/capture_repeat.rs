//! Repeats the dictation thread's sequence (mic start → stop → transcribe → clipboard) several times
//! on ONE thread, like the hotkey worker does, and reports per round: mic start latency and how long
//! the device delivered digital silence before real samples arrived (Bluetooth profile switching).
//!
//!   cargo run --example capture_repeat --features gpu-vulkan -- [--model <file.bin>] [--rounds 4] [--gpu] [--keep <secs>]

use std::time::{Duration, Instant};

use voxelle_core::asr::{CancelToken, LoadOptions, SpeechBackend, WhisperBackend};
use voxelle_core::audio::{AudioCapture, CpalCapture};

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut model, mut rounds, mut gpu, mut keep) = (None::<String>, 4u32, false, 0u64);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => model = args.next(),
            "--rounds" => rounds = args.next().unwrap().parse().unwrap(),
            "--gpu" => gpu = true,
            "--keep" => keep = args.next().unwrap().parse().unwrap(),
            other => panic!("unknown arg {other}"),
        }
    }
    let mut asr = WhisperBackend::new();
    if let Some(m) = &model {
        asr.load(m.as_ref(), &LoadOptions { use_gpu: gpu, threads: voxelle_core::asr::default_threads() })
            .expect("load model");
    }
    let mut cap = CpalCapture::new();
    cap.set_keep_open(Duration::from_secs(keep));
    for round in 1..=rounds {
        let t = Instant::now();
        if let Err(e) = cap.start(None) {
            println!("round {round}: START FAILED: {e}");
            continue;
        }
        let start_ms = t.elapsed().as_millis();
        std::thread::sleep(Duration::from_millis(2500));
        let first_audio = cap.first_audio_ms();
        let audio = cap.stop().expect("stop");
        let first = audio.pcm.iter().position(|s| s.abs() > 1e-4).unwrap_or(audio.pcm.len());
        println!(
            "round {round}: start {start_ms} ms, first audio after {first_audio:?} ms, captured {} ms of 2500, digital silence before first sample {} ms, mic still open: {}",
            audio.duration_ms(),
            first as u64 * 1000 / audio.sample_rate as u64,
            cap.is_open()
        );
        if model.is_some() {
            let _ = asr.transcribe(&audio.to_asr_rate(), Some("en"), &CancelToken::default());
        }
        let _ = voxelle_core::insert::copy_text("voxelle capture_repeat probe");
    }
    if keep > 0 {
        std::thread::sleep(Duration::from_secs(keep + 1));
        println!("after {} s idle: mic still open: {}", keep + 1, cap.is_open());
        cap.start(None).expect("start after idle close");
        std::thread::sleep(Duration::from_millis(500));
        cap.abort();
        println!("after cancel: capturing {}, mic still open: {}", cap.is_capturing(), cap.is_open());
    }
}
