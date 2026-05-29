//! Conversational voice routes — `POST /api/v1/voice/{tts,stt,pitch-turn}`.
//!
//! These are the **V2 gateway layer** of the live-voice plan
//! (`docs/skycot/design/2026-05-29-gap-closure-plans/01-live-voice.md`):
//! a vendor-neutral HTTP skin over the `argo-voice-io` crate's
//! [`SpeechToText`] / [`TextToSpeech`] traits.
//!
//! - `/voice/tts`        — text in, base64 audio out (synthesize).
//! - `/voice/stt`        — base64 audio in, transcript out (transcribe).
//! - `/voice/pitch-turn` — base64 audio in → transcribe → **the existing
//!   truth-bounded pitch answer** → synthesize → base64 audio out.
//!
//! The pitch-turn route does NOT duplicate any pitch/LLM/truth-boundary
//! logic. It reuses the live `/api/v1/pitch/ask` flow
//! ([`super::routes_pitch_ask::ask_response`]) in-process via the injectable
//! `answer` closure of [`argo_voice_io::voice_turn`]: the closure builds a
//! synthetic pitch request, calls the existing handler, and extracts the
//! already-gated answer. So the truth boundary (canonical-Q&A +
//! `argo_capability::evaluate_claim` Allow/Escalate/Block + audit) holds
//! identically whether Portia is typed or spoken.
//!
//! NB: this is distinct from `routes_channels_voice` / `routes_voice`, which
//! handle Twilio **DTMF** (keypad-tone) approval callbacks — that is not
//! conversational voice and is intentionally left untouched.

use super::store_helpers::body_from_request;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use argo_voice_io::{voice_turn, AudioClip, Transcript, VoiceId};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

mod provider;
#[cfg(test)]
#[path = "routes_voice_io_tests.rs"]
mod tests;

use provider::{select_stt, select_tts};

/// Default voice id when a request omits `voice_id`. Portia is the demo
/// Pitch Agent's voice (a manifest parameter in the real adapter path).
const DEFAULT_VOICE_ID: &str = "portia";

/// Body for `POST /api/v1/voice/tts`.
#[derive(Debug, Deserialize)]
struct TtsRequest {
    /// Text to synthesize. Required and non-empty.
    text: String,
    /// Provider voice id; defaults to [`DEFAULT_VOICE_ID`] when omitted.
    #[serde(default)]
    voice_id: Option<String>,
}

/// Body for `POST /api/v1/voice/stt`.
#[derive(Debug, Deserialize)]
struct SttRequest {
    /// Base64-encoded audio bytes to transcribe. Required and non-empty.
    audio_base64: String,
}

/// Body for `POST /api/v1/voice/pitch-turn`.
#[derive(Debug, Deserialize)]
struct PitchTurnRequest {
    /// Base64-encoded audio of the spoken question. Required.
    audio_base64: String,
    /// Provider voice id for the synthesized answer; defaults to Portia.
    #[serde(default)]
    voice_id: Option<String>,
    /// Optional pitch channel context (`Rehearsal` | `WebEmbed` |
    /// `LiveMeeting` | `Chat`). Defaults to `Rehearsal`, matching the
    /// text pitch endpoint.
    #[serde(default)]
    context: Option<String>,
}

/// Handle `POST /api/v1/voice/tts`: synthesize `text` to base64 audio.
///
/// Returns `{ audio_base64, format }` on success. Empty text is a 400.
pub(super) async fn tts_response(req: &str) -> (&'static str, String) {
    let parsed: TtsRequest = match serde_json::from_str(body_from_request(req)) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    if parsed.text.trim().is_empty() {
        return bad_request("`text` is required");
    }
    let voice = voice_id_or_default(parsed.voice_id.as_deref());
    let tts = select_tts();
    match tts.synthesize(&parsed.text, &voice).await {
        Ok(clip) => ok(json!({
            "audio_base64": STANDARD.encode(clip.bytes()),
            "format": clip.format(),
        })),
        Err(e) => map_voice_error("tts synthesize", &e),
    }
}

/// Handle `POST /api/v1/voice/stt`: transcribe base64 audio to text.
///
/// Returns `{ transcript, confidence? }` on success. A missing/invalid
/// base64 body, or empty audio, is a 400.
pub(super) async fn stt_response(req: &str) -> (&'static str, String) {
    let parsed: SttRequest = match serde_json::from_str(body_from_request(req)) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let audio = match decode_audio(&parsed.audio_base64) {
        Ok(bytes) => bytes,
        Err(resp) => return resp,
    };
    let stt = select_stt();
    match stt.transcribe(&audio).await {
        Ok(transcript) => ok(transcript_json(&transcript)),
        Err(e) => map_voice_error("stt transcribe", &e),
    }
}

/// Handle `POST /api/v1/voice/pitch-turn`: one full spoken pitch turn.
///
/// Pipeline: STT(audio) → the existing truth-bounded `/pitch/ask` answer
/// → TTS(answer). Returns
/// `{ transcript, answer_text, audio_base64, format, truth_boundary? }`.
///
/// The `answer` closure reuses [`super::routes_pitch_ask::ask_response`]
/// in-process, so the spoken answer is gated by the *same* truth boundary
/// as the typed pitch — an over-claim question still defers.
pub(super) async fn pitch_turn_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let parsed: PitchTurnRequest = match serde_json::from_str(body_from_request(req)) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let audio = match decode_audio(&parsed.audio_base64) {
        Ok(bytes) => bytes,
        Err(resp) => return resp,
    };
    let voice = voice_id_or_default(parsed.voice_id.as_deref());
    let channel = parsed.context.as_deref().unwrap_or("Rehearsal").to_string();

    // Truth boundary lives entirely inside `pitch_answer` — the voice
    // layer never inspects or rewrites the answer it gets back.
    let stt = select_stt();
    let tts = select_tts();
    let mut boundary: Option<Value> = None;
    // `voice_turn` accepts `&(impl Trait + ?Sized)`, so the boxed adapters
    // pass as `&dyn` trait objects via `as_ref()`.
    let result = voice_turn(stt.as_ref(), tts.as_ref(), &voice, &audio, |heard: Transcript| {
        let boundary = &mut boundary;
        let channel = channel.as_str();
        async move {
            let (answer, tb) = pitch_answer(heard.text(), channel, store).await?;
            *boundary = tb;
            Ok::<_, ArgoError>(answer)
        }
    })
    .await;

    match result {
        Ok(outcome) => ok(pitch_turn_json(&outcome.heard, &outcome.answer_text, &outcome.spoken, boundary)),
        Err(e) => map_voice_error("voice pitch-turn", &e),
    }
}

/// Call the existing truth-bounded pitch handler in-process and return
/// `(answer_text, truth_boundary_json)`.
///
/// This is the no-duplication seam: we forward the transcribed question to
/// [`super::routes_pitch_ask::ask_response`] — the exact handler the typed
/// `/api/v1/pitch/ask` endpoint uses — then read the already-gated `answer`
/// and `disposition` out of its JSON envelope. No LLM call, truth-boundary
/// evaluation, or refusal logic is re-implemented here.
async fn pitch_answer(
    question: &str,
    channel: &str,
    store: Option<&Arc<SessionStore>>,
) -> ArgoResult<(String, Option<Value>)> {
    let inner_req = synth_pitch_request(question, channel);
    let (status, body) = super::routes_pitch_ask::ask_response(&inner_req, store).await;
    if !status.starts_with("200") {
        return Err(ArgoError::Provider(format!(
            "pitch answer failed ({status}): {body}"
        )));
    }
    let parsed: Value = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("pitch answer parse: {e}")))?;
    let answer = parsed
        .get("answer")
        .and_then(Value::as_str)
        .ok_or_else(|| ArgoError::Provider("pitch answer missing `answer` field".into()))?
        .to_string();
    let boundary = json!({
        "disposition": parsed.get("disposition").cloned().unwrap_or(Value::Null),
        "source": parsed.get("source").cloned().unwrap_or(Value::Null),
        "block_reason": parsed.get("block_reason").cloned().unwrap_or(Value::Null),
        "channel": parsed.get("channel").cloned().unwrap_or(Value::Null),
    });
    Ok((answer, Some(boundary)))
}

/// Build a synthetic `POST /api/v1/pitch/ask` HTTP wire-form so the spoken
/// turn flows through the *same* handler as a typed question. Channel is
/// echoed straight into the pitch request body; the pitch handler defaults
/// an unrecognised value to `Rehearsal` itself.
fn synth_pitch_request(question: &str, channel: &str) -> String {
    let body = json!({ "question": question, "channel": channel }).to_string();
    format!(
        "POST /api/v1/pitch/ask HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Decode the `audio_base64` field, mapping failures to a 400. Empty input
/// (after decode) is rejected here so the mock adapters' own empty-audio
/// guard never has to fire on the happy wire path.
fn decode_audio(b64: &str) -> Result<Vec<u8>, (&'static str, String)> {
    if b64.trim().is_empty() {
        return Err(bad_request("`audio_base64` is required"));
    }
    let bytes = STANDARD
        .decode(b64.trim())
        .map_err(|e| bad_request(&format!("`audio_base64` is not valid base64: {e}")))?;
    if bytes.is_empty() {
        return Err(bad_request("`audio_base64` decoded to zero bytes"));
    }
    Ok(bytes)
}

/// A [`VoiceId`] from the request, or [`DEFAULT_VOICE_ID`] when absent or
/// blank.
fn voice_id_or_default(requested: Option<&str>) -> VoiceId {
    match requested.map(str::trim) {
        Some(v) if !v.is_empty() => VoiceId::new(v),
        _ => VoiceId::new(DEFAULT_VOICE_ID),
    }
}

/// JSON body for an STT result: `{ transcript, confidence? }`.
fn transcript_json(transcript: &Transcript) -> Value {
    let mut body = json!({ "transcript": transcript.text() });
    if let Some(conf) = transcript.confidence() {
        if let Some(map) = body.as_object_mut() {
            map.insert("confidence".into(), json!(conf));
        }
    }
    body
}

/// JSON body for a full pitch turn.
fn pitch_turn_json(
    heard: &Transcript,
    answer_text: &str,
    spoken: &AudioClip,
    boundary: Option<Value>,
) -> Value {
    let mut body = json!({
        "transcript": heard.text(),
        "answer_text": answer_text,
        "audio_base64": STANDARD.encode(spoken.bytes()),
        "format": spoken.format(),
    });
    if let (Some(map), Some(conf)) = (body.as_object_mut(), heard.confidence()) {
        map.insert("confidence".into(), json!(conf));
    }
    if let (Some(map), Some(tb)) = (body.as_object_mut(), boundary) {
        map.insert("truth_boundary".into(), tb);
    }
    body
}

/// Map an [`ArgoError`] from the voice stack to a gateway response. Schema
/// errors are caller faults (400); everything else is a provider/adapter
/// fault (503).
fn map_voice_error(stage: &str, err: &ArgoError) -> (&'static str, String) {
    match err {
        ArgoError::Schema(msg) => bad_request(&format!("{stage}: {msg}")),
        other => (
            "503 Service Unavailable",
            json!({ "error": { "code": "voice_unavailable", "message": format!("{stage}: {other}") } })
                .to_string(),
        ),
    }
}

/// `200 OK` JSON envelope.
fn ok(body: Value) -> (&'static str, String) {
    ("200 OK", body.to_string())
}

/// `400 Bad Request` JSON envelope.
fn bad_request(msg: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": msg } }).to_string(),
    )
}
