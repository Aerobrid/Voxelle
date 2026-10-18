//! Repeatable ASR benchmark over bench/corpus using the exact backend the app runs.
//!
//!   cargo run --release --example asr_bench -- --model <file.bin> --corpus ../bench/corpus [--gpu] [--gap <secs>] [--limit <n>]
//!
//! Prints per-clip results and one JSON summary line (prefixed "RESULT ").

use std::path::PathBuf;
use std::time::Instant;

use voxelle_core::asr::{wer, CancelToken, LoadOptions, SpeechBackend, WhisperBackend};
use voxelle_core::audio::{resample, ASR_SAMPLE_RATE};

struct Args {
    model: PathBuf,
    corpus: PathBuf,
    gpu: bool,
    threads: usize,
    language: Option<String>,
    /// Idle seconds before each clip, like the pauses between real dictations (GPU clocks drop).
    gap_secs: u64,
    /// Only the first N clips.
    limit: usize,
}

fn parse_args() -> Args {
    let mut a = Args {
        model: PathBuf::new(),
        corpus: PathBuf::from("../bench/corpus"),
        gpu: false,
        threads: voxelle_core::asr::default_threads(),
        language: Some("en".into()),
        gap_secs: 0,
        limit: usize::MAX,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--model" => a.model = it.next().expect("--model <path>").into(),
            "--corpus" => a.corpus = it.next().expect("--corpus <dir>").into(),
            "--gpu" => a.gpu = true,
            "--threads" => a.threads = it.next().expect("--threads <n>").parse().expect("number"),
            "--auto-language" => a.language = None,
            "--gap" => a.gap_secs = it.next().expect("--gap <secs>").parse().expect("number"),
            "--limit" => a.limit = it.next().expect("--limit <n>").parse().expect("number"),
            other => panic!("unknown flag {other}"),
        }
    }
    assert!(a.model.is_file(), "model not found: {}", a.model.display());
    a
}

fn read_wav(path: &std::path::Path) -> Vec<f32> {
    let mut r = hound::WavReader::open(path).expect("open wav");
    let spec = r.spec();
    let ch = spec.channels as usize;
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>().map(|s| s.unwrap() as f32 / scale).collect()
        }
        hound::SampleFormat::Float => r.samples::<f32>().map(Result::unwrap).collect(),
    };
    let mut mono = Vec::with_capacity(raw.len() / ch);
    voxelle_core::audio::downmix_into(&raw, ch, &mut mono);
    resample(&mono, spec.sample_rate, ASR_SAMPLE_RATE)
}

fn main() {
    let args = parse_args();
    let tsv = std::fs::read_to_string(args.corpus.join("sentences.tsv")).expect("sentences.tsv");
    let clips: Vec<(String, String, Vec<f32>)> = tsv
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once('\t'))
        .take(args.limit)
        .map(|(id, text)| {
            let pcm = read_wav(&args.corpus.join("audio").join(format!("{id}.wav")));
            (id.to_string(), text.to_string(), pcm)
        })
        .collect();
    assert!(!clips.is_empty(), "empty corpus");

    let mut backend = WhisperBackend::new();
    let t = Instant::now();
    backend
        .load(&args.model, &LoadOptions { use_gpu: args.gpu, threads: args.threads })
        .expect("load model");
    let load_ms = t.elapsed().as_millis();
    let caps = backend.capabilities();
    let cancel = CancelToken::default();
    let lang = args.language.as_deref();

    // First call includes one-time allocations (and shader compilation on Vulkan); report it apart.
    let t = Instant::now();
    backend.transcribe(&clips[0].2, lang, &cancel).expect("warm-up");
    let first_ms = t.elapsed().as_millis();

    let (mut total_ms, mut audio_ms, mut errs, mut words) = (0f64, 0f64, 0usize, 0usize);
    let mut per_clip = Vec::new();
    for (id, reference, pcm) in &clips {
        std::thread::sleep(std::time::Duration::from_secs(args.gap_secs));
        let t = Instant::now();
        let out = backend.transcribe(pcm, lang, &cancel).expect("transcribe");
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let dur = pcm.len() as f64 * 1000.0 / ASR_SAMPLE_RATE as f64;
        let (e, n) = wer::word_errors(reference, &out.text);
        total_ms += ms;
        audio_ms += dur;
        errs += e;
        words += n;
        per_clip.push(ms);
        println!("{id}  {ms:7.0} ms  audio {dur:5.0} ms  errors {e}/{n}  | {}", out.text);
    }
    per_clip.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = per_clip[per_clip.len() / 2];
    let max = *per_clip.last().unwrap();
    let model = args.model.file_name().unwrap().to_string_lossy();
    println!(
        "RESULT {{\"model\":\"{model}\",\"gpuRequested\":{},\"gpuActive\":{},\"threads\":{},\"loadMs\":{load_ms},\"firstCallMs\":{first_ms},\"clips\":{},\"meanMs\":{:.0},\"p50Ms\":{p50:.0},\"maxMs\":{max:.0},\"rtf\":{:.3},\"wer\":{:.4},\"errors\":{errs},\"words\":{words}}}",
        args.gpu,
        caps.gpu_active,
        args.threads,
        clips.len(),
        total_ms / clips.len() as f64,
        total_ms / audio_ms,
        errs as f64 / words as f64,
    );
}
