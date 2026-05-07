//! Whisper.cpp [`Recognizer`] implementation.
//!
//! Compiled only with the `whisper-native` feature. Mirrors the
//! existing `mcp-servers/voice/src/whisper_native.rs` pattern:
//! requires `ARGO_WHISPER_MODEL` to point at a ggml model file
//! (e.g. `ggml-base.en.bin` from
//! <https://huggingface.co/ggerganov/whisper.cpp>); audio is
//! captured by [`crate::audio`] (cpal) into a 16-kHz f32 PCM
//! buffer and handed straight to whisper-rs.
//!
//! No tokio / async-trait here — the trait surface is sync, and
//! the runner is expected to drive transcription on a blocking
//! thread (`spawn_blocking` in the CLI handler).

use crate::recognizer::{Recognizer, RecognizerError};

/// Whisper.cpp-backed recognizer. Holds an initialized
/// `WhisperContext` and a callback that yields the next utterance
/// as 16-kHz f32 PCM samples.
pub struct WhisperRecognizer {
    ctx: whisper_rs::WhisperContext,
    next_samples: Box<dyn FnMut() -> Result<Vec<f32>, String> + Send>,
    language: Option<String>,
}

impl WhisperRecognizer {
    /// Build a new recognizer. `model_path` should point at a ggml
    /// model file. `next_samples` is the source-of-PCM callback —
    /// typically [`crate::audio::record_one_phrase`] in production,
    /// a closure over a Vec<f32> in tests.
    ///
    /// `language` is the ISO 639-1 code (e.g. "en") or `None` for
    /// auto-detect.
    pub fn new(
        model_path: &str,
        next_samples: Box<dyn FnMut() -> Result<Vec<f32>, String> + Send>,
        language: Option<String>,
    ) -> Result<Self, RecognizerError> {
        let ctx = whisper_rs::WhisperContext::new_with_params(
            model_path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| RecognizerError::Backend(format!("load whisper model {model_path}: {e}")))?;
        Ok(Self {
            ctx,
            next_samples,
            language,
        })
    }
}

impl Recognizer for WhisperRecognizer {
    fn transcribe_next(&mut self) -> Result<String, RecognizerError> {
        let samples = (self.next_samples)().map_err(RecognizerError::Backend)?;
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| RecognizerError::Backend(format!("whisper create_state: {e}")))?;
        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        if let Some(lang) = self.language.as_deref() {
            params.set_language(Some(lang));
        }
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        state
            .full(params, &samples)
            .map_err(|e| RecognizerError::Backend(format!("whisper full: {e}")))?;
        let mut out = String::new();
        for segment in state.as_iter() {
            out.push_str(&segment.to_string());
        }
        Ok(out.trim().to_string())
    }
}
