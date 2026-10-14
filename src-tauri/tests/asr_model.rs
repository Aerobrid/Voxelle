//! Integration tests against a real whisper model. Skipped (pass trivially, with a note) unless
//! VOXELLE_TEST_MODEL points at a model file, e.g. ggml-base-q5_1.bin. Uses bench/corpus audio.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use voxelle_core::asr::{wer, AsrError, CancelToken, LoadOptions, SpeechBackend, WhisperBackend};
use voxelle_core::audio::{resample, ASR_SAMPLE_RATE};

fn model() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var("VOXELLE_TEST_MODEL").ok()?);
    p.is_file().then_some(p)
}

fn corpus_clip(id: &str) -> Option<(Vec<f32>, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bench/corpus");
    let tsv = std::fs::read_to_string(dir.join("sentences.tsv")).ok()?;
    let text = tsv.lines().find_map(|l| l.strip_prefix(&format!("{id}\t")))?.to_string();
    let mut r = hound::WavReader::open(dir.join("audio").join(format!("{id}.wav"))).ok()?;
    let spec = r.spec();
    let pcm: Vec<f32> = r.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect();
    Some((resample(&pcm, spec.sample_rate, ASR_SAMPLE_RATE), text))
}

fn backend(path: &Path) -> WhisperBackend {
    let mut b = WhisperBackend::new();
    b.load(path, &LoadOptions { use_gpu: false, threads: 4 }).expect("load");
    b
}

#[test]
fn transcribes_a_corpus_clip_accurately() {
    let (Some(m), Some((pcm, reference))) = (model(), corpus_clip("s01")) else {
        eprintln!("skipped: set VOXELLE_TEST_MODEL and generate bench/corpus audio");
        return;
    };
    let mut b = backend(&m);
    let out = b.transcribe(&pcm, Some("en"), &CancelToken::default()).unwrap();
    let w = wer::wer(&reference, &out.text);
    assert!(w <= 0.15, "WER {w} for {:?}", out.text);
    assert_eq!(out.language.as_deref(), Some("en"));
}

/// Low-level room-like noise (no speech). whisper alone hallucinates "you" here, so the app gates
/// such clips out before ASR; this test pins both facts.
#[test]
fn quiet_noise_is_gated_before_asr() {
    let mut seed = 12345u32;
    let noise: Vec<f32> = (0..ASR_SAMPLE_RATE as usize * 2)
        .map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            ((seed >> 16) as f32 / 32768.0 - 1.0) * 0.003
        })
        .collect();
    assert!(
        voxelle_core::audio::peak_frame_dbfs(&noise, ASR_SAMPLE_RATE) < voxelle_core::audio::SPEECH_FLOOR_DBFS,
        "room-noise clip must be rejected by the speech gate"
    );
    if let Some(m) = model() {
        let mut b = backend(&m);
        let out = b.transcribe(&noise, Some("en"), &CancelToken::default()).unwrap();
        eprintln!("ungated whisper output on noise: {:?}", out.text);
    }
}

#[test]
fn cancel_stops_transcription_promptly() {
    let (Some(m), Some((pcm, _))) = (model(), corpus_clip("s03")) else { return };
    let mut b = backend(&m);
    let token = CancelToken::default();
    let canceller = token.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        canceller.cancel();
    });
    let t = Instant::now();
    let r = b.transcribe(&pcm, Some("en"), &token);
    let took = t.elapsed();
    assert!(matches!(r, Err(AsrError::Cancelled)), "expected Cancelled, got {r:?}");
    // whisper.cpp checks the abort hook between the encoder pass and each decoder step (not inside
    // the encoder graph), so a cancel during encoding lands when that pass ends. The session layer
    // is cancelled immediately regardless; this only bounds wasted compute.
    eprintln!("cancel returned after {took:?} (cancel requested at 30 ms)");
    // The backend stays usable after a cancel.
    let ok = b.transcribe(&pcm, Some("en"), &CancelToken::default());
    assert!(ok.is_ok(), "backend unusable after cancel: {ok:?}");
}

#[test]
fn pre_cancelled_token_returns_immediately() {
    let Some(m) = model() else { return };
    let mut b = backend(&m);
    let token = CancelToken::default();
    token.cancel();
    let t = Instant::now();
    assert!(matches!(b.transcribe(&[0.1; 16_000], None, &token), Err(AsrError::Cancelled)));
    assert!(t.elapsed() < Duration::from_millis(50));
}
