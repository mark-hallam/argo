//! `backend="whisper"` STT via the whisper-rs (whisper.cpp)
//! Rust bindings. Opt-in via the `whisper-native` feature.
//!
//! Configuration (when feature is enabled):
//!   - `ARGO_WHISPER_MODEL` (required) — absolute path to a
//!     ggml whisper model file (e.g. `ggml-base.en.bin`).
//!     Download from
//!     <https://huggingface.co/ggerganov/whisper.cpp>.
//!   - The audio file passed to `transcribe` MUST be a 16-kHz
//!     WAV (PCM16 mono — the most common whisper-friendly
//!     shape). For other formats / sample rates, decode +
//!     resample externally first (`ffmpeg -i in.x -ac 1 -ar
//!     16000 out.wav`), or use the `backend="local"` shell-out
//!     path with `whisper-cli`.
//!
//! Without `--features whisper-native`, the path returns a
//! "rebuild with feature" error so callers know the path
//! forward.
use argo_mcp::{CallToolResult, McpContent};

#[cfg(feature = "whisper-native")]
fn err(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

#[cfg(feature = "whisper-native")]
pub async fn transcribe(audio_path: &str, language: Option<&str>) -> CallToolResult {
    let model_path = match std::env::var("ARGO_WHISPER_MODEL") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => {
            return err(
                "ARGO_WHISPER_MODEL is not set. Point it at a ggml whisper model file \
                 (e.g. ggml-base.en.bin) — download from \
                 https://huggingface.co/ggerganov/whisper.cpp"
                    .to_string(),
            );
        }
    };

    // hound + whisper.cpp are sync-only; do the work on a blocking
    // thread so we don't stall the runtime.
    let audio = audio_path.to_string();
    let lang = language.map(|s| s.to_string());
    let join = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let mut reader = hound::WavReader::open(&audio)
            .map_err(|e| format!("open {audio}: {e}"))?;
        let spec = reader.spec();
        if spec.sample_rate != 16_000 {
            return Err(format!(
                "audio sample rate must be 16000 Hz (got {}) — re-encode with \
                 `ffmpeg -i in.wav -ac 1 -ar 16000 out.wav`",
                spec.sample_rate
            ));
        }
        if spec.channels != 1 {
            return Err(format!(
                "audio must be mono (got {} channels) — re-encode with \
                 `ffmpeg -i in.wav -ac 1 -ar 16000 out.wav`",
                spec.channels
            ));
        }
        // PCM16 → f32 via the whisper-rs helper. For float WAVs
        // we already have the f32 samples; just collect.
        let mono_samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let pcm: Vec<i16> = reader
                    .samples::<i16>()
                    .collect::<Result<_, _>>()
                    .map_err(|e| format!("decode int samples: {e}"))?;
                let mut out = vec![0_f32; pcm.len()];
                whisper_rs::convert_integer_to_float_audio(&pcm, &mut out)
                    .map_err(|e| format!("convert i16 → f32: {e}"))?;
                out
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<_, _>>()
                .map_err(|e| format!("decode float samples: {e}"))?,
        };

        let ctx = whisper_rs::WhisperContext::new_with_params(
            &model_path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| format!("load whisper model {model_path}: {e}"))?;
        let mut state = ctx
            .create_state()
            .map_err(|e| format!("whisper create_state: {e}"))?;
        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        if let Some(lang) = lang.as_deref() {
            params.set_language(Some(lang));
        }
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        state
            .full(params, &mono_samples)
            .map_err(|e| format!("whisper full: {e}"))?;
        let mut out = String::new();
        for segment in state.as_iter() {
            out.push_str(&segment.to_string());
        }
        Ok(out.trim().to_string())
    })
    .await;
    match join {
        Ok(Ok(text)) => CallToolResult {
            content: vec![McpContent::Text { text }],
            is_error: false,
        },
        Ok(Err(e)) => err(e),
        Err(e) => err(format!("join: {e}")),
    }
}

#[cfg(not(feature = "whisper-native"))]
pub async fn transcribe(_audio_path: &str, _language: Option<&str>) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "backend=whisper requires the `whisper-native` feature. Rebuild with \
                   `cargo build -p argo-mcp-voice --features whisper-native` (heavy native \
                   build — needs cmake + a C++ compiler). For a no-deps alternative, set \
                   ARGO_TRANSCRIBE_CMD and use `backend=local`."
                .into(),
        }],
        is_error: true,
    }
}
