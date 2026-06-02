//! Live-voice (WebRTC) session minting — `POST /api/v1/voice/realtime-session`.
//!
//! The browser cannot hold the OpenAI key, so this endpoint mints a
//! short-lived **ephemeral client secret** server-side (the real key never
//! leaves the gateway) with Portia's truth-boundary policy + company facts
//! baked into the session `instructions`. The browser then opens a WebRTC
//! peer connection straight to OpenAI's Realtime API with that ephemeral
//! secret (`POST /v1/realtime/calls`), so the audio path never transits the
//! gateway.
//!
//! This is the **streaming** counterpart to the discrete `pitch-turn` route
//! (DEC-021 latency upgrade). The truth boundary here is **LLM-primary**: the
//! realtime model is *instructed* to defer on forbidden claim classes (the
//! exact policy text the typed/loop pitch renders), but the deterministic
//! post-answer `evaluate_claim` swap cannot run on a token-streamed answer —
//! there is no "draft, then check, then speak" seam. The instructions
//! therefore carry the policy emphatically (see [`REALTIME_PREAMBLE`]). The
//! live test of "does streaming hold" is exactly: ask a forbidden-class
//! question out loud and confirm she defers; if she fabricates, fall back to
//! the discrete loop (which keeps the hard post-check).
//!
//! NO THEATER: with no `OPENAI_API_KEY` this is a 503, never a faked token.

use super::routes_pitch_ask::{company_facts_markdown, realtime_instructions, PitchChannel};
use super::store_helpers::body_from_request;
use argo_store::SessionStore;
use argo_types::Redacted;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// OpenAI endpoint that mints an ephemeral Realtime client secret.
const CLIENT_SECRETS_URL: &str = "https://api.openai.com/v1/realtime/client_secrets";
/// Realtime model. Pinned so a demo can't roll the model forward mid-session.
const REALTIME_MODEL: &str = "gpt-realtime-2";
/// Realtime voice for Portia. The Realtime API's voice set differs from
/// `tts-1` (so `shimmer` is not assumed valid here); `marin` is a
/// vendor-valid Realtime voice — warm and conversational for the demo.
const REALTIME_VOICE: &str = "marin";
/// Env var holding the OpenAI key (gateway-process secret, tenant-unscoped),
/// the same source the discrete STT/TTS adapters read.
const OPENAI_KEY_ENV: &str = "OPENAI_API_KEY";

/// Body for `POST /api/v1/voice/realtime-session`. All fields optional.
#[derive(Debug, Deserialize)]
struct RealtimeSessionRequest {
    /// Pitch channel context (`Rehearsal` | `WebEmbed` | `LiveMeeting` |
    /// `Chat`). Defaults to `Rehearsal`, matching the discrete pitch turn.
    #[serde(default)]
    context: Option<String>,
    /// Session persona: `pitch` (default) = Portia, truth-bounded; `interview`
    /// = a founder-story interviewer that asks Mark about his story.
    #[serde(default)]
    mode: Option<String>,
}

/// Handle `POST /api/v1/voice/realtime-session`: mint an ephemeral OpenAI
/// Realtime client secret carrying Portia's truth-bounded instructions.
///
/// Returns `{ client_secret, model, voice, expires_at? }`. No key → 503
/// (never a faked token). The `_store` arg keeps the uniform dispatch
/// signature; this route is process-scoped (a provider secret), like the
/// other voice routes.
pub(super) async fn realtime_session_response(
    req: &str,
    _store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let parsed: RealtimeSessionRequest = match serde_json::from_str(body_from_request(req)) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let channel = parse_channel(parsed.context.as_deref());

    // Two personas. "interview" = a founder-story interviewer that draws Mark's
    // story out (NOT truth-bounded — it interviews the founder, it doesn't
    // represent the company). "pitch" (default) = Portia, reusing the exact
    // truth-boundary policy + facts the typed/loop pitch renders — no parallel copy.
    // v1 personas are selected by `mode`. v2 (teammate-aware voice) will source
    // the persona from a hired teammate's Genesis charter via `worker_id` so
    // these collapse into one path — "talk to <any teammate> live, in their own
    // persona + memory". For now: pitch (Portia), interview, coach (Cora).
    let instructions = match parsed.mode.as_deref() {
        Some("interview") => ground_in_skycot(INTERVIEW_INSTRUCTIONS),
        Some("coach") => ground_in_skycot(COACH_INSTRUCTIONS),
        _ => match realtime_instructions(channel) {
            Ok(s) => format!("{REALTIME_PREAMBLE}\n\n{s}"),
            Err(e) => return service_unavailable(&format!("pitch corpus/policies: {e}")),
        },
    };

    let key = match read_openai_key() {
        Ok(k) => k,
        Err(resp) => return resp,
    };

    // Session shape per the GA Realtime API (validated against
    // /v1/realtime/client_secrets): turn_detection lives under `audio.input`,
    // voice under `audio.output` — NOT as top-level `session.*` fields.
    let session = json!({
        "session": {
            "type": "realtime",
            "model": REALTIME_MODEL,
            "instructions": instructions,
            "audio": {
                "input": { "turn_detection": { "type": "server_vad" } },
                "output": { "voice": REALTIME_VOICE },
            },
        }
    });

    let client = match reqwest::Client::builder()
        .user_agent(concat!("argo-voice-realtime/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => return service_unavailable(&format!("http client: {e}")),
    };

    let resp = match client
        .post(CLIENT_SECRETS_URL)
        .bearer_auth(key.expose())
        .json(&session)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return service_unavailable(&format!("openai realtime mint: {e}")),
    };

    let status = resp.status();
    let value: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return service_unavailable(&format!("openai realtime decode: {e}")),
    };
    if !status.is_success() {
        return service_unavailable(&format!("openai realtime HTTP {status}: {value}"));
    }

    // The mint response carries the ephemeral secret. Accept either a bare
    // top-level `value` or the nested `client_secret.value` shape.
    let secret = value
        .get("value")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/client_secret/value").and_then(Value::as_str));
    let Some(secret) = secret else {
        return service_unavailable("openai realtime: mint response missing client secret");
    };
    let expires_at = value
        .get("expires_at")
        .or_else(|| value.pointer("/client_secret/expires_at"))
        .cloned()
        .unwrap_or(Value::Null);

    ok(json!({
        "client_secret": secret,
        "model": REALTIME_MODEL,
        "voice": REALTIME_VOICE,
        "expires_at": expires_at,
    }))
}

/// Spoken-realtime preamble prepended to the rendered pitch system prompt.
/// Reinforces the truth boundary for a *streamed* answer (there is no
/// deterministic post-check backstop in this mode) and sets a live-voice tone.
const REALTIME_PREAMBLE: &str = "You are speaking LIVE, out loud, in a real-time voice \
conversation. Keep answers short and conversational — one to three spoken sentences. This is \
the highest-stakes setting for your truth boundary: there is no post-answer safety net here, so \
YOU are the only guardrail. If a question targets a forbidden or escalate-only claim class below \
(valuation, revenue, customer counts, unannounced roadmap, or anything not in your approved \
facts), do NOT improvise a number, range, or hedge — say plainly that you are not authorized to \
share it and that Mark handles that one directly. When in doubt, defer. Never invent a fact to \
fill a silence.";

/// Founder-story interviewer persona for `mode=interview`. NOT truth-bounded:
/// it interviews Mark (the founder), it does not represent the company. The
/// captured transcripts become the raw material for his founder narrative +
/// building-in-public content.
const INTERVIEW_INSTRUCTIONS: &str = "You are a warm, sharp founder-story interviewer — think a \
great podcast host or an early-stage investor having a first real conversation. You are \
interviewing Mark Hallam, the founder of SKYCOT, an AI-teammates platform he built himself — as a \
non-developer — by directing a swarm of AI agents to build it. Your job is to draw out his \
founder story for use in fundraising and building in public. Cover, in a natural order: the \
moment he decided this had to exist; his background and why he is the right person to build it; \
what it was actually like building a deep technical product as a non-developer with AI; why this \
matters to him personally; and his vision and what he wants next. Rules: ask ONE question at a \
time, then listen. Follow up on whatever is most interesting in his answer before moving on. Keep \
YOUR turns short — he is the star; you are drawing him out, not lecturing or pitching. Be warm and \
genuinely curious. Open by introducing yourself in one sentence and asking him to start with the \
moment he knew SKYCOT had to exist. When his story feels complete, thank him warmly and stop.";

/// Cora — Mark's founder coach — as a LIVE voice persona (`mode=coach`). v1
/// sources her persona from the role brief she's hired against in Genesis;
/// v2 will load her actual charter + memory by `worker_id` so the voice IS
/// the roster teammate (and remembers prior sessions).
const COACH_INSTRUCTIONS: &str = "You are Cora, Mark Hallam's AI founder coach at SKYCOT, \
speaking LIVE, out loud, in a real-time voice conversation. You are his accountability partner \
and strategic advisor: you hold him to his goals, prepare and rehearse his investor pitches \
(starting with his investor meeting), advise on go-to-market for a pre-seed, pre-traction founder \
including a building-in-public cadence, act as his sounding board through the 'if you were me, \
what would you do' lens, and surface his blind spots. Your defining trait: you are truth-bounded \
and direct — you always give Mark the honest read, never flattery or false comfort. You are a \
coach, not a cheerleader: when he is wrong, avoiding something, or fooling himself, you say so \
kindly but clearly. Speak warmly, conversationally, and briefly — one to three spoken sentences \
per turn; ask one question at a time, listen, and follow up on what matters. You coach, draft, \
and advise; you never take public or external actions on your own. Open by greeting Mark by name \
and asking what he wants to work on today.";

/// Ground a live-voice persona (Cora's coach, the founder interviewer) in the
/// canonical SKYCOT facts + a spoken-delivery rule, so she works from the *real*
/// product (not generically) and never voices a raw identifier aloud. Degrades
/// to persona + delivery rule if the facts can't load (never fails the session).
fn ground_in_skycot(persona: &str) -> String {
    let spoken = "When you reference anything technical, say it the way a person says it out \
loud — \"tenant ID\", not \"tenant_id\"; a listener must never hear the word \"underscore\".";
    match company_facts_markdown() {
        Ok(facts) => format!(
            "{persona}\n\n{spoken}\n\n## What you know about SKYCOT (ground everything you say in \
these facts; never invent beyond them)\n{facts}"
        ),
        Err(_) => format!("{persona}\n\n{spoken}"),
    }
}

/// Map the optional `context` string to a [`PitchChannel`], defaulting to
/// `Rehearsal` for an absent/blank/unrecognised value (matching the discrete
/// pitch endpoint's default).
fn parse_channel(context: Option<&str>) -> PitchChannel {
    match context.map(str::trim).unwrap_or("") {
        "WebEmbed" => PitchChannel::WebEmbed,
        "LiveMeeting" => PitchChannel::LiveMeeting,
        "Chat" => PitchChannel::Chat,
        _ => PitchChannel::Rehearsal,
    }
}

/// Read the OpenAI key (tenant-unscoped gateway-process secret) into a
/// [`Redacted`] so it can never leak through `Debug`. Absent/empty → a 503
/// response tuple (never a faked token).
fn read_openai_key() -> Result<Redacted<String>, (&'static str, String)> {
    let key = argo_config::cred::get_unscoped(OPENAI_KEY_ENV)
        .map_err(|_| service_unavailable(&format!("{OPENAI_KEY_ENV} not set")))?;
    if key.trim().is_empty() {
        return Err(service_unavailable(&format!("{OPENAI_KEY_ENV} is empty")));
    }
    Ok(Redacted::new(key))
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

/// `503 Service Unavailable` JSON envelope (no key, or an OpenAI mint error).
fn service_unavailable(msg: &str) -> (&'static str, String) {
    (
        "503 Service Unavailable",
        json!({ "error": { "code": "voice_unavailable", "message": msg } }).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_channel_maps_known_and_defaults_rehearsal() {
        assert!(matches!(parse_channel(Some("WebEmbed")), PitchChannel::WebEmbed));
        assert!(matches!(parse_channel(Some("LiveMeeting")), PitchChannel::LiveMeeting));
        assert!(matches!(parse_channel(Some("Chat")), PitchChannel::Chat));
        assert!(matches!(parse_channel(Some("  Rehearsal  ")), PitchChannel::Rehearsal));
        assert!(matches!(parse_channel(None), PitchChannel::Rehearsal));
        assert!(matches!(parse_channel(Some("bogus")), PitchChannel::Rehearsal));
    }

    #[tokio::test]
    async fn bad_body_is_400() {
        let req = "POST /api/v1/voice/realtime-session HTTP/1.1\r\nContent-Type: application/json\r\n\r\nnot json";
        let (status, _body) = realtime_session_response(req, None).await;
        assert!(status.starts_with("400"), "expected 400, got {status}");
    }

    #[test]
    fn coach_and_interview_are_grounded_in_company_facts() {
        // Cora's coach/interview voices must carry the SAME canonical SKYCOT
        // facts Portia's pitch uses — else she coaches generically and can't
        // catch an over-claim. (`load_cached` reads repo facts in tests, like
        // the pitch-facts tests.)
        let coach = ground_in_skycot(COACH_INSTRUCTIONS);
        assert!(coach.contains("Cora"), "persona preserved");
        assert!(
            coach.contains("What you know about SKYCOT"),
            "company facts section must be appended (len {})",
            coach.len()
        );
        assert!(
            coach.len() > COACH_INSTRUCTIONS.len() + 200,
            "facts must materially expand the prompt"
        );
        assert!(coach.contains("underscore"), "spoken-delivery rule present");
        assert!(ground_in_skycot(INTERVIEW_INSTRUCTIONS).contains("What you know about SKYCOT"));
    }
}
