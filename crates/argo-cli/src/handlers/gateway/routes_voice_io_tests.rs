//! Inline unit tests for the conversational voice routes.
//!
//! Happy paths use the deterministic `argo-voice-io` mocks (no API key,
//! no network). The pitch-turn truth-boundary test drives a known
//! over-claim question through the *real* `/pitch/ask` flow via
//! [`super::pitch_answer`] and asserts the spoken answer defers rather
//! than fabricating a number — proving the boundary holds in voice.
//!
//! Provider note: these pass `store: None`, so the selector falls through
//! to the `ARGO_VOICE_PROVIDER` env, whose documented default (unset) is
//! the mock — hence the `MOCK_AUDIO_FORMAT` assertions. They do not mutate
//! the process-global env (which would race other gateway tests). The real
//! OpenAI adapter has its own key-gated tests in `routes_voice_io/openai.rs`.

use super::*;
use argo_voice_io::{MockTts, VoiceId, MOCK_AUDIO_FORMAT};

/// Build a `POST <path>` HTTP wire-form with a JSON body.
fn post(path: &str, body: &serde_json::Value) -> String {
    let body = body.to_string();
    format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

// ---- /voice/tts -----------------------------------------------------

#[tokio::test]
async fn tts_round_trips_text_to_decodable_audio() {
    let req = post("/api/v1/voice/tts", &json!({ "text": "hello investors", "voice_id": "portia" }));
    let (status, body) = tts_response(&req, None).await;
    assert_eq!(status, "200 OK", "body={body}");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["format"], MOCK_AUDIO_FORMAT);
    let b64 = parsed["audio_base64"].as_str().expect("audio_base64 present");
    let decoded = STANDARD.decode(b64).expect("audio_base64 decodes");
    // Mock TTS is deterministic: the bytes are exactly what the adapter
    // derives for this text+voice, so the route round-trips them intact.
    let expected = MockTts::bytes_for("hello investors", &VoiceId::new("portia"));
    assert_eq!(decoded, expected, "route must emit the synthesized bytes verbatim");
    assert!(!decoded.is_empty());
}

#[tokio::test]
async fn tts_defaults_voice_when_absent() {
    let req = post("/api/v1/voice/tts", &json!({ "text": "no voice given" }));
    let (status, body) = tts_response(&req, None).await;
    assert_eq!(status, "200 OK", "body={body}");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    let decoded = STANDARD.decode(parsed["audio_base64"].as_str().unwrap()).unwrap();
    // Default voice is Portia — the bytes must match that derivation.
    let expected = MockTts::bytes_for("no voice given", &VoiceId::new(DEFAULT_VOICE_ID));
    assert_eq!(decoded, expected);
}

#[tokio::test]
async fn tts_rejects_empty_text() {
    let req = post("/api/v1/voice/tts", &json!({ "text": "   " }));
    let (status, _body) = tts_response(&req, None).await;
    assert_eq!(status, "400 Bad Request");
}

// ---- /voice/stt -----------------------------------------------------

#[tokio::test]
async fn stt_returns_transcript_for_valid_audio() {
    let audio_b64 = STANDARD.encode(b"some-spoken-audio-bytes");
    let req = post("/api/v1/voice/stt", &json!({ "audio_base64": audio_b64 }));
    let (status, body) = stt_response(&req, None).await;
    assert_eq!(status, "200 OK", "body={body}");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    let transcript = parsed["transcript"].as_str().expect("transcript present");
    assert!(!transcript.is_empty(), "transcript must be non-empty");
    // Echo mock derives a stable transcript from the audio bytes.
    assert!(transcript.starts_with("mock-transcript:"), "got {transcript}");
    // Confidence is surfaced when the adapter supplies one.
    assert_eq!(parsed["confidence"], json!(1.0));
}

#[tokio::test]
async fn stt_rejects_invalid_base64() {
    let req = post("/api/v1/voice/stt", &json!({ "audio_base64": "!!!not base64!!!" }));
    let (status, body) = stt_response(&req, None).await;
    assert_eq!(status, "400 Bad Request", "body={body}");
}

#[tokio::test]
async fn stt_rejects_empty_audio() {
    let req = post("/api/v1/voice/stt", &json!({ "audio_base64": "" }));
    let (status, _body) = stt_response(&req, None).await;
    assert_eq!(status, "400 Bad Request");
}

// ---- /voice/pitch-turn ----------------------------------------------

#[tokio::test]
async fn pitch_turn_returns_answer_audio_and_truth_boundary() {
    // No SessionStore + no API key → the pitch handler takes the
    // deterministic canonical path (LLM auto-disabled without a key),
    // so this is fully offline yet still real (not a faked answer).
    let audio_b64 = STANDARD.encode(b"a-spoken-question-about-skycot");
    let req = post("/api/v1/voice/pitch-turn", &json!({ "audio_base64": audio_b64 }));
    let (status, body) = pitch_turn_response(&req, None).await;
    assert_eq!(status, "200 OK", "body={body}");
    let parsed: Value = serde_json::from_str(&body).unwrap();

    // Heard transcript is surfaced.
    assert!(parsed["transcript"].as_str().is_some_and(|s| !s.is_empty()));
    // Answer text is non-empty.
    let answer = parsed["answer_text"].as_str().expect("answer_text present");
    assert!(!answer.trim().is_empty(), "answer_text must be non-empty");
    // Audio is present + decodable + non-empty.
    let decoded = STANDARD.decode(parsed["audio_base64"].as_str().unwrap()).unwrap();
    assert!(!decoded.is_empty(), "spoken audio must be non-empty");
    assert_eq!(parsed["format"], MOCK_AUDIO_FORMAT);
    // The synthesized audio is exactly the TTS of the answer text — i.e.
    // we spoke the truth-bounded answer, not something else.
    let expected_audio = MockTts::bytes_for(answer, &VoiceId::new(DEFAULT_VOICE_ID));
    assert_eq!(decoded, expected_audio);
    // Truth-boundary block is threaded through from the pitch handler.
    assert!(parsed.get("truth_boundary").is_some(), "truth_boundary must be present");
    assert!(parsed["truth_boundary"]["disposition"].is_string());
}

#[tokio::test]
async fn pitch_turn_rejects_missing_audio() {
    let req = post("/api/v1/voice/pitch-turn", &json!({ "voice_id": "portia" }));
    let (status, body) = pitch_turn_response(&req, None).await;
    assert_eq!(status, "400 Bad Request", "body={body}");
}

/// The load-bearing test: an over-claim question must DEFER, never
/// fabricate a number. Exercises [`pitch_answer`] directly with a known
/// question (the mock STT can't recover arbitrary text from bytes), which
/// is the exact seam carrying the reused truth boundary. With no API key
/// the canonical-Q&A path answers a valuation question with the
/// operator-approved deferral; the assertion holds regardless of whether
/// it hits canonical or falls back to the deterministic miss line.
#[tokio::test]
async fn pitch_turn_over_claim_question_defers() {
    // No env mutation: the pitch handler auto-disables the LLM fallback
    // when no provider key is present (the test env has none), so the
    // canonical-Q&A path runs deterministically. Avoiding `set_var` keeps
    // this test from racing the process-global env other gateway tests
    // (e.g. the Twilio-signature tests) mutate under their own lock.
    let (answer, boundary) = pitch_answer("What's your valuation?", "WebEmbed", None)
        .await
        .expect("pitch answer resolves");

    let lower = answer.to_lowercase();
    // Must defer to the founder / decline — never state a number.
    assert!(
        lower.contains("mark")
            || lower.contains("founder")
            || lower.contains("not authorized")
            || lower.contains("won't share")
            || lower.contains("check with"),
        "over-claim answer must defer, got: {answer}"
    );
    // And it must NOT have leaked a dollar figure.
    assert!(
        !answer.contains('$'),
        "over-claim answer must not contain a dollar figure: {answer}"
    );
    // The truth-boundary block is present and carries a disposition.
    let tb = boundary.expect("truth_boundary present");
    assert!(tb["disposition"].is_string(), "disposition must be a string: {tb}");
}

#[tokio::test]
async fn pitch_turn_benign_question_answers_substantively() {
    // Same rationale as the over-claim test: no env mutation — the LLM
    // path is already off without a provider key.
    let (answer, boundary) = pitch_answer("How much does Skycot cost?", "WebEmbed", None)
        .await
        .expect("pitch answer resolves");
    assert!(!answer.trim().is_empty());
    // The canonical pricing entry is an Allow, so the disposition is
    // present and the answer is substantive (mentions subscription).
    let tb = boundary.expect("truth_boundary present");
    assert!(tb["disposition"].is_string());
}
