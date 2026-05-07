//! cpal-based microphone capture (16 kHz mono f32 PCM).
//!
//! Compiled only with the `whisper-native` feature. Two entry
//! points:
//!
//! - [`enumerate_input_devices`] returns the names of the input
//!   devices cpal sees. Used at startup so the CLI handler can
//!   degrade to TUI list view cleanly when there is no mic.
//! - [`record_one_phrase`] captures audio for `record_seconds`,
//!   resamples / channel-collapses to 16 kHz mono if needed, and
//!   returns the f32 PCM samples ready to hand to
//!   [`crate::recognizer_whisper::WhisperRecognizer`].
//!
//! Resampling is intentionally crude — drop-sample / nearest
//! neighbour. whisper.cpp tolerates moderate aliasing for the
//! 5-word phrase set this loop targets, and good resampling
//! adds another C++ crate. Operators who need higher fidelity can
//! pre-record with `ffmpeg -i in.wav -ac 1 -ar 16000 out.wav` and
//! feed the WAV via the (separate) `mcp-servers/voice` stack.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TARGET_RATE: u32 = 16_000;

/// Names of every input device cpal sees on this host.
///
/// Returns an empty vec when the platform refuses to enumerate
/// (e.g. mic permission denied) — the CLI handler treats that as
/// "no mic available, fall back to TUI list".
pub fn enumerate_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| d.name().ok())
        .collect()
}

/// Capture `record_seconds` of audio from the default input device,
/// resample to 16 kHz mono f32, and return the PCM samples.
///
/// Errors:
/// - "no default input device" (mic permission denied or no
///   hardware) — caller should fall back to the TUI list.
/// - "config / build_input_stream / play / recv" — surface upstream
///   error string verbatim.
pub fn record_one_phrase(record_seconds: f32) -> Result<Vec<f32>, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no default input device".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("default_input_config: {e}"))?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let sample_format = config.sample_format();

    let buffer: std::sync::Arc<std::sync::Mutex<Vec<f32>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let buf_for_cb = buffer.clone();

    let err_fn = |err| {
        tracing::warn!(target: "argo_voice_tui::audio", "cpal stream error: {err}");
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config.config(),
                move |data: &[f32], _| {
                    let mut buf = match buf_for_cb.lock() {
                        Ok(b) => b,
                        Err(_) => return,
                    };
                    buf.extend_from_slice(data);
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build_input_stream f32: {e}"))?,
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &config.config(),
                move |data: &[i16], _| {
                    let mut buf = match buf_for_cb.lock() {
                        Ok(b) => b,
                        Err(_) => return,
                    };
                    buf.extend(data.iter().map(|s| f32::from(*s) / f32::from(i16::MAX)));
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build_input_stream i16: {e}"))?,
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                &config.config(),
                move |data: &[u16], _| {
                    let mut buf = match buf_for_cb.lock() {
                        Ok(b) => b,
                        Err(_) => return,
                    };
                    buf.extend(
                        data.iter()
                            .map(|s| (f32::from(*s) - 32768.0) / 32768.0),
                    );
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build_input_stream u16: {e}"))?,
        other => return Err(format!("unsupported sample format: {other:?}")),
    };

    stream.play().map_err(|e| format!("stream play: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(
        (record_seconds * 1000.0) as u64,
    ));
    drop(stream);

    let raw = buffer.lock().map_err(|e| format!("lock: {e}"))?.clone();
    Ok(downmix_and_resample(&raw, channels, sample_rate))
}

fn downmix_and_resample(samples: &[f32], channels: usize, source_rate: u32) -> Vec<f32> {
    let mono: Vec<f32> = if channels <= 1 {
        samples.to_vec()
    } else {
        samples
            .chunks_exact(channels)
            .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
            .collect()
    };
    if source_rate == TARGET_RATE {
        return mono;
    }
    let ratio = TARGET_RATE as f32 / source_rate as f32;
    let out_len = (mono.len() as f32 * ratio) as usize;
    (0..out_len)
        .map(|i| {
            let src_idx = (i as f32 / ratio) as usize;
            mono.get(src_idx).copied().unwrap_or(0.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_stereo_to_mono() {
        let stereo = [0.5, -0.5, 1.0, -1.0]; // (0.0, 0.0)
        let mono = downmix_and_resample(&stereo, 2, TARGET_RATE);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.0).abs() < 1e-6);
        assert!((mono[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn resample_passthrough_at_target() {
        let s = vec![0.1; 1000];
        let r = downmix_and_resample(&s, 1, TARGET_RATE);
        assert_eq!(r.len(), 1000);
    }

    #[test]
    fn resample_downsample_count() {
        // 32 kHz → 16 kHz halves the sample count.
        let s = vec![0.1; 1000];
        let r = downmix_and_resample(&s, 1, 32_000);
        // 1000 * 0.5 = 500 samples
        assert_eq!(r.len(), 500);
    }
}
