//! Tests for the voice-intake extractor — transcript passthrough (the live
//! path), base64-audio transcription via the STT seam, and the byte caps.

use super::*;
// `Engine` (for `.encode`) is already in scope via `use super::*` (the parent
// module imports it); only the `general_purpose` path binding is needed here.
use base64::engine::general_purpose;
use serde_json::json;

#[tokio::test]
async fn transcript_passthrough_is_used_verbatim() {
    // The live voice loop already transcribed via /voice/stt; Genesis ingests
    // the transcript verbatim — no re-transcription.
    let transcript = "Greet every visitor by name and notify the host within two minutes.";
    let doc = json!({ "filename": "intro.wav", "format": "voice", "transcript": transcript });
    let (text, bytes) = transcribe_voice(&doc, 0).await.expect("ok");
    assert_eq!(text, transcript);
    assert_eq!(bytes, transcript.len());
}

#[tokio::test]
async fn empty_voice_is_rejected() {
    let doc = json!({ "filename": "x.wav", "format": "voice", "content": "   " });
    let err = transcribe_voice(&doc, 0).await.unwrap_err();
    assert!(err.to_lowercase().contains("empty"));
}

#[tokio::test]
async fn base64_audio_is_transcribed_via_the_seam() {
    // No transcript field → decode the audio and run it through the in-crate
    // STT seam (MockStt::echo), proving the seam is actually invoked.
    let audio = general_purpose::STANDARD.encode(b"some-fake-audio-bytes");
    let doc = json!({ "filename": "clip.wav", "format": "voice_base64", "content": audio });
    let (text, bytes) = transcribe_voice(&doc, 0).await.expect("ok");
    assert!(
        text.starts_with("mock-transcript:"),
        "the STT seam should have produced the transcript, got: {text}"
    );
    assert_eq!(bytes, b"some-fake-audio-bytes".len());
}

#[tokio::test]
async fn total_cap_is_enforced() {
    let doc = json!({ "filename": "x.wav", "format": "voice", "transcript": "a short transcript" });
    let err = transcribe_voice(&doc, MAX_TOTAL_BYTES).await.unwrap_err();
    assert!(err.to_lowercase().contains("total"));
}
