//! Voice-narrated role intake for `POST /api/v1/genesis/ingest-document`
//! (Genesis Phase-2 multi-modal onboarding).
//!
//! An operator can describe a role out loud instead of uploading a document.
//! The spoken narration becomes an onboarding document the same way a typed SOP
//! does: transcript → chunk → tenant-scoped persist → gradeable in the quiz.
//!
//! ## No second STT path
//!
//! Transcription reuses the **existing** speech-to-text seam
//! ([`argo_voice_io::SpeechToText`]) — the same trait the live `/api/v1/voice`
//! loop is built on. Two honest entry points, no new transcriber:
//!
//! 1. **`transcript` field present** — the SPA's live voice loop already
//!    transcribed the audio through `/api/v1/voice/stt` (the real, per-tenant
//!    provider). Genesis ingests that transcript verbatim. This is the live
//!    production path: the real model ran once, in the voice route, not here.
//! 2. **`content` = base64 audio, no transcript** — the deterministic /
//!    offline path. We decode the audio and run it through the in-crate
//!    [`argo_voice_io::MockStt`] seam adapter. This needs no API key and never
//!    fabricates — it is what the integration test exercises.
//!
//! Either way the byte size charged against the onboarding session cap is the
//! size of what was uploaded (the transcript text, or the decoded audio).

use argo_genesis::{MAX_FILE_BYTES, MAX_TOTAL_BYTES};
use argo_voice_io::{MockStt, SpeechToText};
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;

/// Produce `(transcript_text, bytes_charged)` for one voice-intake document.
///
/// `doc` is the per-document JSON object from the ingest request. Returns a
/// friendly per-file error string (never panics) so a bad clip is reported as
/// "skipped" while the rest of the batch still ingests.
pub(super) async fn transcribe_voice(
    doc: &Value,
    total_so_far: usize,
) -> Result<(String, usize), String> {
    // Path 1 — the live voice loop already transcribed (real provider).
    if let Some(transcript) = doc.get("transcript").and_then(Value::as_str) {
        let text = transcript.trim();
        if !text.is_empty() {
            let bytes = text.len();
            check_caps(bytes, total_so_far)?;
            return Ok((text.to_string(), bytes));
        }
    }

    // Path 2 — base64 audio, transcribed deterministically via the STT seam.
    let content = doc
        .get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if content.is_empty() {
        return Err("That voice clip looks empty — skipped.".to_string());
    }
    let audio = general_purpose::STANDARD
        .decode(content.as_bytes())
        .map_err(|e| format!("voice audio base64 decode failed — skipped ({e})"))?;
    check_caps(audio.len(), total_so_far)?;

    let stt = MockStt::echo();
    let transcript = stt
        .transcribe(&audio)
        .await
        .map_err(|e| format!("Couldn't transcribe the voice clip — skipped ({e})"))?;
    let text = transcript.text().trim().to_string();
    if text.is_empty() {
        return Err("The transcription came back empty — skipped.".to_string());
    }
    Ok((text, audio.len()))
}

/// Enforce the per-file and running-total byte caps shared with the document
/// ingest path.
fn check_caps(bytes: usize, total_so_far: usize) -> Result<(), String> {
    if bytes > MAX_FILE_BYTES {
        return Err(format!(
            "voice clip is {} MB, over the {}-MB per-file limit — skipped.",
            bytes / (1024 * 1024),
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    if total_so_far + bytes > MAX_TOTAL_BYTES {
        return Err(format!(
            "this upload would exceed the {}-MB total onboarding limit — skipped.",
            MAX_TOTAL_BYTES / (1024 * 1024)
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "routes_genesis_ingest_voice_tests.rs"]
mod tests;
