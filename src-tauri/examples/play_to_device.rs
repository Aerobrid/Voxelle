//! Test harness: play a WAV file on a named output device (e.g. a virtual audio cable whose other
//! end is selected as Voxelle's microphone), so the full dictation loop can be exercised without a
//! person speaking. Optionally plays a lead-in of silence first.
//!
//!   cargo run --example play_to_device -- --device "CABLE Input" --wav ../bench/corpus/audio/s01.wav [--lead-ms 500]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut device_name, mut wav, mut lead_ms) = (None::<String>, None::<String>, 0u64);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--device" => device_name = args.next(),
            "--wav" => wav = args.next(),
            "--lead-ms" => lead_ms = args.next().unwrap().parse().unwrap(),
            other => panic!("unknown arg {other}"),
        }
    }
    let device_name = device_name.expect("--device");
    let mut reader = hound::WavReader::open(wav.expect("--wav")).expect("open wav");
    let spec = reader.spec();
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader.samples::<i32>().map(|s| s.unwrap() as f32 * scale).collect()
        }
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    let mono: Vec<f32> = interleaved
        .chunks(spec.channels as usize)
    .map(|f| f.iter().sum::<f32>() / f.len() as f32)
    .collect();

    let host = cpal::default_host();
    let device = host
        .output_devices()
        .expect("output devices")
        .find(|d| d.description().map(|x| x.name().contains(&device_name)).unwrap_or(false))
        .unwrap_or_else(|| panic!("no output device containing '{device_name}'"));
    let config: cpal::StreamConfig = device.default_output_config().expect("config").config();
    let rate = config.sample_rate;
    let channels = config.channels as usize;
    let mut pcm = vec![0.0f32; (rate as u64 * lead_ms / 1000) as usize];
    pcm.extend(voxelle_core::audio::resample(&mono, spec.sample_rate, rate));
    let total = pcm.len();
    let pcm = Arc::new(pcm);
    let pos = Arc::new(AtomicUsize::new(0));
    let (p, q) = (pcm.clone(), pos.clone());
    let stream = device
        .build_output_stream(
            config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in out.chunks_mut(channels) {
                    let i = q.fetch_add(1, Ordering::Relaxed);
                    let s = p.get(i).copied().unwrap_or(0.0);
                    frame.iter_mut().for_each(|x| *x = s);
                }
            },
            |e| eprintln!("output error: {e}"),
            None,
        )
        .expect("build output stream");
    stream.play().expect("play");
    while pos.load(Ordering::Relaxed) < total {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(150));
    println!("played {} ms on {}", total as u64 * 1000 / rate as u64, device_name);
}
