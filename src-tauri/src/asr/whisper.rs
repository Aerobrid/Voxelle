//! whisper.cpp backend via whisper-rs. CPU always works; when the build has a GPU feature
//! (`gpu-vulkan` / `gpu-cuda`) and the caller asks for it, the GPU is tried first and a load
//! failure falls back to CPU instead of leaving the user without dictation.

use std::path::{Path, PathBuf};

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

use super::{join_segments, AsrError, CancelToken, Capabilities, LoadOptions, SpeechBackend, Transcription};

pub const GPU_COMPILED: bool = cfg!(any(feature = "gpu-vulkan", feature = "gpu-cuda"));
/// Segments whisper rates above this no-speech probability are discarded.
const NO_SPEECH_DROP: f32 = 0.6;

struct Loaded {
    path: PathBuf,
    // Reused across calls: allocating compute buffers per utterance costs latency (esp. on GPU).
    state: WhisperState,
    gpu_active: bool,
    multilingual: bool,
}

pub struct WhisperBackend {
    loaded: Option<Loaded>,
    threads: usize,
}

impl Default for WhisperBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WhisperBackend {
    pub fn new() -> Self {
        // Route whisper.cpp/ggml logging away from stdout (it is noisy and may echo paths).
        whisper_rs::install_logging_hooks();
        Self {
            loaded: None,
            threads: default_threads(),
        }
    }

    fn try_load(path: &Path, use_gpu: bool) -> Result<(WhisperContext, WhisperState), AsrError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| AsrError::LoadFailed("model path is not valid UTF-8".into()))?;
        let mut params = WhisperContextParameters::default();
        params.use_gpu(use_gpu);
        let ctx = WhisperContext::new_with_params(path_str, params)
            .map_err(|e| AsrError::LoadFailed(e.to_string()))?;
        let state = ctx
            .create_state()
            .map_err(|e| AsrError::LoadFailed(e.to_string()))?;
        Ok((ctx, state))
    }
}

unsafe extern "C" fn abort_if_cancelled(user_data: *mut std::ffi::c_void) -> bool {
    // SAFETY: user_data is `CancelToken::flag_ptr()`, valid for the duration of `full()`.
    let flag = unsafe { &*(user_data as *const std::sync::atomic::AtomicBool) };
    flag.load(std::sync::atomic::Ordering::SeqCst)
}

/// Physical cores are what matter for ggml; leave one for the UI and audio threads.
pub fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .clamp(1, 8)
}

/// Whisper language codes are short ASCII letters ("en", "haw"). Anything else is rejected rather
/// than passed to C (which would also panic on interior NULs).
fn valid_language(code: &str) -> bool {
    (2..=5).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_lowercase())
}

impl SpeechBackend for WhisperBackend {
    fn load(&mut self, model_path: &Path, opts: &LoadOptions) -> Result<(), AsrError> {
        if self
            .loaded
            .as_ref()
            .is_some_and(|l| l.path == model_path && l.gpu_active == (opts.use_gpu && GPU_COMPILED))
        {
            return Ok(());
        }
        self.unload();
        if !model_path.is_file() {
            return Err(AsrError::LoadFailed(format!(
                "model file not found: {}",
                model_path.display()
            )));
        }
        self.threads = opts.threads.max(1);
        let want_gpu = opts.use_gpu && GPU_COMPILED;
        let (ctx, state, gpu_active) = match Self::try_load(model_path, want_gpu) {
            Ok((c, s)) => (c, s, want_gpu),
            Err(gpu_err) if want_gpu => {
                // GPU init can fail (driver, VRAM). CPU fallback keeps dictation working.
                let (c, s) = Self::try_load(model_path, false).map_err(|cpu_err| {
                    AsrError::LoadFailed(format!("gpu: {gpu_err}; cpu: {cpu_err}"))
                })?;
                (c, s, false)
            }
            Err(e) => return Err(e),
        };
        let multilingual = ctx.is_multilingual();
        // `state` holds its own reference to the model; the context handle can go.
        drop(ctx);
        self.loaded = Some(Loaded {
            path: model_path.to_path_buf(),
            state,
            gpu_active,
            multilingual,
        });
        Ok(())
    }

    fn transcribe(
        &mut self,
        pcm_16k: &[f32],
        language: Option<&str>,
        cancel: &CancelToken,
    ) -> Result<Transcription, AsrError> {
        let loaded = self.loaded.as_mut().ok_or(AsrError::NoModel)?;
        if cancel.is_cancelled() {
            return Err(AsrError::Cancelled);
        }
        let language = match language {
            Some(code) if !valid_language(code) => {
                return Err(AsrError::Failed(format!("invalid language code: {code}")))
            }
            // English-only models ignore other languages; force "en" so detection can't misfire.
            _ if !loaded.multilingual => Some("en"),
            other => other,
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(self.threads as i32);
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_translate(false);
        params.set_no_context(true); // each dictation is independent
        params.set_no_timestamps(true);
        params.set_suppress_nst(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // whisper-rs 0.16 `set_abort_callback_safe` stores the closure as `Box<dyn FnMut>` but its
        // trampoline reads it back as the concrete closure type, so the hook returns garbage and
        // aborts decoding at random. Use a plain C hook over our flag instead. The flag outlives
        // `full()` because `cancel` is borrowed for this whole call.
        unsafe {
            params.set_abort_callback(Some(abort_if_cancelled));
            params.set_abort_callback_user_data(cancel.flag_ptr() as *mut std::ffi::c_void);
        }

        let result = loaded.state.full(params, pcm_16k);
        if cancel.is_cancelled() {
            return Err(AsrError::Cancelled);
        }
        result.map_err(|e| AsrError::Failed(e.to_string()))?;

        let mut texts = Vec::new();
        for seg in loaded.state.as_iter() {
            // whisper invents short phrases ("you", "Thank you.") on non-speech; trust its own
            // no-speech estimate and drop those segments.
            if seg.no_speech_probability() > NO_SPEECH_DROP {
                continue;
            }
            texts.push(seg.to_str_lossy().map_err(|e| AsrError::Failed(e.to_string()))?.into_owned());
        }
        let detected = whisper_rs::get_lang_str(loaded.state.full_lang_id_from_state()).map(str::to_string);
        Ok(Transcription {
            text: join_segments(texts.iter().map(String::as_str)),
            language: detected.or_else(|| language.map(str::to_string)),
        })
    }

    fn unload(&mut self) {
        // Dropping state + context frees RAM/VRAM.
        self.loaded = None;
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            gpu_compiled: GPU_COMPILED,
            gpu_active: self.loaded.as_ref().is_some_and(|l| l.gpu_active),
            streaming: false,
            multilingual: self.loaded.as_ref().is_some_and(|l| l.multilingual),
        }
    }

    fn loaded_model(&self) -> Option<&Path> {
        self.loaded.as_ref().map(|l| l.path.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_are_validated() {
        assert!(valid_language("en"));
        assert!(valid_language("haw"));
        assert!(!valid_language("e"));
        assert!(!valid_language("EN"));
        assert!(!valid_language("en\0"));
        assert!(!valid_language("en-us-xx"));
    }

    #[test]
    fn missing_model_is_a_clear_error_not_a_crash() {
        let mut b = WhisperBackend::new();
        let err = b
            .load(Path::new("definitely/not/here.bin"), &LoadOptions { use_gpu: false, threads: 1 })
            .unwrap_err();
        assert!(matches!(err, AsrError::LoadFailed(m) if m.contains("not found")));
        let err = b.transcribe(&[0.0; 16_000], None, &CancelToken::default()).unwrap_err();
        assert!(matches!(err, AsrError::NoModel));
    }

    #[test]
    fn corrupt_model_file_fails_to_load_cleanly() {
        let dir = std::env::temp_dir().join(format!("voxelle-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bogus = dir.join("bogus-model.bin");
        std::fs::write(&bogus, b"this is not a ggml model").unwrap();
        let mut b = WhisperBackend::new();
        let err = b.load(&bogus, &LoadOptions { use_gpu: false, threads: 1 }).unwrap_err();
        assert!(matches!(err, AsrError::LoadFailed(_)));
        assert!(b.loaded_model().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
