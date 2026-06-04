//! Live (real-call) provider-key validation for
//! `POST /api/v1/onboarding/provider-key`.
//!
//! Sibling of `routes_onboarding_helpers.rs`, which carries the
//! shape-only fast gate (`validate_key_shape`). Split into its own file
//! to keep both under the 500-LOC budget and to isolate the network
//! seam (this is the only onboarding helper that dials a provider).
//!
//! ## Contract
//!
//! The handler runs the cheap shape pre-check first; only a
//! well-shaped key reaches here. This module then — and only for a real
//! (non-mock) workspace and a provider we ship a manifest for —
//! performs ONE bounded, 1-token completion with the **submitted** key
//! via the existing `argo-provider` transport abstraction (no
//! hand-rolled per-provider HTTP).
//!
//! Verdicts:
//! - `Valid` — the provider accepted the key (2xx round-trip).
//! - `Invalid` — the provider returned a clear `401`/`403`. The save is
//!   rejected (NO THEATER: never a green success on a bad key).
//! - `Undetermined` — live validation is off, no shipped manifest for
//!   the provider, or the call failed for a reason that does NOT prove
//!   the key bad (timeout, connection refused, `5xx`, `429`). We do not
//!   block a possibly-valid key on a transient blip; the save proceeds
//!   on the strength of the shape gate, exactly as before this brief.
//!
//! The live call is gated **off by default** so CI, unit tests, and the
//! built-in mock-provider demo path never touch the network. The live
//! gateway opts in by exporting `ARGO_LIVE_KEY_VALIDATION=1`, mirroring
//! the Genesis (`ARGO_GENESIS_LLM_CLASSIFIER`) and pitch
//! (`ARGO_PITCH_LLM_FALLBACK`) opt-in discipline.

use argo_provider::{
    AnthropicTransport, OpenAiCompatTransport, OpenAiResponsesTransport, ProviderFamily,
    ProviderManifest, ProviderRequest, Sampling, Transport,
};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use chrono::Utc;
use std::time::Duration;
use uuid::Uuid;

/// Env var opting the live one-token validation call in. Off by default
/// (mirrors `ARGO_GENESIS_LLM_CLASSIFIER`).
pub(super) const LIVE_KEY_VALIDATION_ENV: &str = "ARGO_LIVE_KEY_VALIDATION";

/// Wall-clock ceiling on the validation round-trip so a slow provider
/// can never hang the onboarding save. On elapse we return
/// [`LiveKeyOutcome::Undetermined`] (don't block a possibly-valid key).
const LIVE_VALIDATION_TIMEOUT: Duration = Duration::from_secs(8);

/// Verdict from the live validation call. See module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LiveKeyOutcome {
    /// Provider accepted the key.
    Valid,
    /// Provider returned a clear auth failure; the carried message is
    /// user-facing.
    Invalid(String),
    /// Could not prove the key bad (gate off, no manifest, transient
    /// failure, timeout). The save proceeds.
    Undetermined,
}

/// `true` when [`LIVE_KEY_VALIDATION_ENV`] is set to a truthy value
/// (`1` / `true` / `on` / `yes`, case-insensitive). Absent / empty /
/// any other value reads as off.
pub(super) fn live_validation_enabled() -> bool {
    matches!(
        std::env::var(LIVE_KEY_VALIDATION_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Map the onboarding provider label (as inferred by
/// `infer_provider`, or supplied by the client) to a shipped provider
/// manifest id we can do a real key-bearing call against. Returns
/// `None` for providers we don't ship an HTTP manifest for
/// (`google`, `azure`, `custom`, …) or for credential schemes that
/// aren't a single API key (`bedrock` uses AWS creds) — the caller maps
/// `None` to [`LiveKeyOutcome::Undetermined`] (shape-validated only).
pub(super) fn manifest_id_for_provider(provider: &str) -> Option<&'static str> {
    match provider {
        "anthropic" => Some("anthropic"),
        "openai" => Some("openai-compatible"),
        _ => None,
    }
}

/// Cheapest concrete model id for the probe. HTTP families need a
/// concrete id (no `"latest"`); we pick the smallest model so the
/// 1-token call costs a fraction of a cent.
fn probe_model(manifest_id: &str) -> &'static str {
    match manifest_id {
        "anthropic" => "claude-haiku-4-5",
        "openai-compatible" => "gpt-4o-mini",
        _ => "latest",
    }
}

/// Classify a provider transport error string into a key verdict.
///
/// A clear `401`/`403` is the only signal we treat as "the key is bad".
/// Everything else — connection refused, timeout, `5xx`, `429`
/// rate-limit — is [`LiveKeyOutcome::Undetermined`]: we could not prove
/// the key bad, so onboarding must not block on it. The transports emit
/// `"<provider> HTTP <status>: <body>"` for non-2xx responses
/// (`anthropic/mod.rs`, `openai_compat/mod.rs`), so a substring match on
/// the status is sufficient and provider-agnostic.
pub(super) fn classify_provider_error(msg: &str) -> LiveKeyOutcome {
    if msg.contains("HTTP 401") || msg.contains("HTTP 403") {
        LiveKeyOutcome::Invalid(
            "This provider rejected the API key (unauthorized). Check the key is \
             correct and still active, then try again."
                .to_string(),
        )
    } else {
        LiveKeyOutcome::Undetermined
    }
}

/// Run the gated, bounded live validation. Safe to call unconditionally
/// from the handler: returns [`LiveKeyOutcome::Undetermined`] without
/// touching the network whenever the gate is off or the provider has no
/// shipped manifest.
pub(super) async fn validate_key_live(provider: &str, api_key: &str) -> LiveKeyOutcome {
    if !live_validation_enabled() {
        return LiveKeyOutcome::Undetermined;
    }
    let Some(manifest_id) = manifest_id_for_provider(provider) else {
        return LiveKeyOutcome::Undetermined;
    };
    match tokio::time::timeout(LIVE_VALIDATION_TIMEOUT, one_token_probe(manifest_id, api_key)).await
    {
        Ok(Ok(())) => LiveKeyOutcome::Valid,
        Ok(Err(e)) => classify_provider_error(&e.to_string()),
        // Timed out — a slow provider must not hang the save.
        Err(_elapsed) => LiveKeyOutcome::Undetermined,
    }
}

/// One non-streaming, 1-token completion against `manifest_id` using the
/// submitted `api_key`. Mirrors the genesis path's `complete_once`
/// (load manifest from `<cwd>/manifests/providers/<id>.toml`, then
/// `complete`), but injects the caller's key instead of resolving one
/// from the environment.
async fn one_token_probe(manifest_id: &str, api_key: &str) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let manifest = argo_agent_runner::load_manifest(&cwd, manifest_id).map_err(|e| {
        ArgoError::Provider(format!("live key validation: load `{manifest_id}` manifest: {e}"))
    })?;
    let model = probe_model(manifest_id).to_string();
    let transport = build_probe_transport(manifest, api_key)?;
    let req = ProviderRequest {
        model,
        messages: vec![probe_message()],
        tools: Vec::new(),
        system: None,
        sampling: Sampling {
            temperature: Some(0.0),
            max_tokens: Some(1),
            ..Sampling::default()
        },
    };
    transport.complete(&req).await.map(|_resp| ())
}

/// Build the HTTP transport for the probe with the submitted key.
///
/// Only the key-bearing HTTP families are reachable here — subprocess
/// (`claude-code-cli`) and Bedrock (AWS creds, not a single API key)
/// have no `manifest_id` mapping, so [`manifest_id_for_provider`] never
/// routes them in. A future family without a mapping fails loud rather
/// than silently passing the key check.
fn build_probe_transport(
    manifest: ProviderManifest,
    api_key: &str,
) -> ArgoResult<Box<dyn Transport>> {
    let t: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Anthropic => Box::new(AnthropicTransport::from_manifest(manifest, api_key)?),
        ProviderFamily::OpenaiCompatible => {
            Box::new(OpenAiCompatTransport::from_manifest(manifest, api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, api_key)?)
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "live key validation: provider family {other:?} is not key-probe-able"
            )))
        }
    };
    Ok(t)
}

/// The minimal user turn for the probe — one token in, one token out.
fn probe_message() -> Message {
    Message {
        id: MessageId(Uuid::now_v7()),
        session_id: SessionId(Uuid::nil()),
        role: MessageRole::User,
        content: vec![ContentBlock::Text { text: "Hi".to_string() }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

#[cfg(test)]
#[path = "routes_onboarding_live_key_tests.rs"]
mod tests;
