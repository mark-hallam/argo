//! Real-LLM generation path for `POST /api/v1/genesis`.
//!
//! The v1 Genesis pipeline classifies the role with a deterministic
//! 12-archetype keyword scorer that never calls a provider. This module
//! turns the gated LLM path on: it implements the narrow
//! [`argo_genesis::ProviderClient`] trait against the SAME provider
//! abstraction the Pitch Agent uses
//! (`argo_agent_runner::{build_transport, load_manifest}` →
//! `argo_provider::Transport`), so the archetype is chosen by a real
//! model reading the operator's role description rather than by keyword
//! overlap.
//!
//! Per DEC-012 the dev/demo provider is `claude-code-cli` (billed
//! against the operator's Claude Code subscription, not the metered
//! Anthropic API). The provider id is read from
//! [`GENESIS_LLM_PROVIDER_ENV`], falling back to the pitch path's
//! `ARGO_PITCH_LLM_PROVIDER`, then to `claude-code-cli`. No HTTP API is
//! hardcoded — the manifest's `transport_kind` decides the driver.
//!
//! ## No silent fall-through
//!
//! `argo_genesis::classify_llm` already falls back to the keyword
//! scorer on ANY provider error / unknown slug, so a transient model
//! hiccup never breaks Genesis. To prove the LLM path actually fired
//! (no theater), the adapter flips an internal flag the moment a
//! provider round-trip returns a usable slug; the handler reads
//! [`GenesisLlmAdapter::fired`] afterward and surfaces `llm` / `broad` /
//! `keyword` in the SSE progress detail + audit row.

use argo_agent_runner::{build_transport, load_manifest};
use argo_genesis::{
    plan, plan_or_broad, Archetype, BroadRoleClient, GenesisConfig, GenesisError, GenesisPlan,
    ProviderClient,
};
use argo_provider::{ProviderRequest, ProviderResponse, Sampling};
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId};
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

/// Explicit opt-in for the real-LLM archetype classifier. Defaults
/// **off** so test runs (which inherit the operator's `ANTHROPIC_API_KEY`
/// and so satisfy the upstream provider-key gate) stay on the
/// deterministic keyword path — no subprocess dial-out, no flakiness,
/// byte-identical archetypes. The live gateway turns it on by exporting
/// `ARGO_GENESIS_LLM_CLASSIFIER=1` in `~/.argo/env.local`. Mirrors the
/// pitch path's `ARGO_PITCH_LLM_FALLBACK` opt-in discipline.
pub(super) const GENESIS_LLM_CLASSIFIER_ENV: &str = "ARGO_GENESIS_LLM_CLASSIFIER";

/// Env var naming the provider manifest used for Genesis LLM
/// classification. Defaults are resolved in [`resolve_provider_id`].
pub(super) const GENESIS_LLM_PROVIDER_ENV: &str = "ARGO_GENESIS_LLM_PROVIDER";

/// `true` when [`GENESIS_LLM_CLASSIFIER_ENV`] is set to a truthy value
/// (`1` / `true` / `on` / `yes`, case-insensitive). Absent / empty /
/// any other value reads as off.
pub(super) fn llm_classifier_enabled() -> bool {
    matches!(
        std::env::var(GENESIS_LLM_CLASSIFIER_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Default classification model. The `claude-code-cli` subprocess
/// driver ignores this (the CLI auto-routes), but the HTTP families
/// need a concrete id, so it is pinned here rather than `"latest"`.
const GENESIS_MODEL: &str = "claude-sonnet-4-6";

/// Classification prompts are tiny (one slug back). 64 tokens is ample
/// headroom for the `<slug>` answer the prompt asks for.
const GENESIS_MAX_TOKENS: u32 = 64;

/// Resolve the provider id for the Genesis LLM path.
///
/// Order: `ARGO_GENESIS_LLM_PROVIDER` → `ARGO_PITCH_LLM_PROVIDER`
/// (so a single dev/demo export drives both LLM surfaces) →
/// `claude-code-cli` (the DEC-012 dev/demo default).
pub(super) fn resolve_provider_id() -> String {
    for key in [GENESIS_LLM_PROVIDER_ENV, "ARGO_PITCH_LLM_PROVIDER"] {
        if let Ok(v) = std::env::var(key) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "claude-code-cli".to_string()
}

/// Resolve a role description into a materialized [`GenesisPlan`].
///
/// When the real-LLM classifier is opted in
/// ([`llm_classifier_enabled`]), the role is routed through
/// [`argo_genesis::plan_or_broad`]: a confident archetype role takes the
/// archetype path; a genuinely off-taxonomy role is synthesized from scratch
/// into a `CapabilitySpec` and materialized (DEC-025 P1). Otherwise the
/// deterministic keyword pipeline runs with no provider call at all (the test
/// default — keeps archetypes byte-identical and avoids a subprocess
/// dial-out).
///
/// Returns the plan plus a `"llm"` / `"broad"` / `"keyword"` mode label. No
/// theater: the broad/from-scratch path only fires when the classifier was
/// not confident, the archetype path only when a provider round-trip returned
/// a recognised slug, and the label reflects which one actually fired — so a
/// transient model failure both keeps Genesis working AND reports `keyword`
/// honestly.
pub(super) async fn resolve_plan(
    role_description: &str,
    persona: &str,
) -> Result<(GenesisPlan, &'static str), GenesisError> {
    if !llm_classifier_enabled() {
        return Ok((plan(role_description, persona)?, "keyword"));
    }
    // DEC-025 P1: route through `plan_or_broad`, not `plan_with`. This turns
    // ON the from-scratch materializer — a genuinely off-taxonomy role
    // (classifier confidence < 0.6) is synthesized into a bespoke
    // `CapabilitySpec` and materialized into a real loadable vertical,
    // instead of being forced into the nearest archetype. A confident
    // archetype role still takes the archetype path unchanged.
    //
    // The judge is intentionally `None` HERE: the LLM semantic safety judge
    // runs at the hire gate (`run_hire_gate` → `run_safety_signal`), which
    // the SSE handler invokes immediately after this. That seam is the one
    // place the degraded-mode distinction lives — a judge that CAN'T run
    // (provider/quota failure) degrades to `needs_review` (held in dry-run,
    // audited), NOT a hard `Block`. Wiring the judge into synthesis here
    // would hard-fail the whole pipeline on any transport hiccup (the exact
    // self-inflicted DoS the design doc's degraded mode exists to prevent),
    // so the judge fires once, correctly, at the gate.
    let adapter = GenesisLlmAdapter::new(resolve_provider_id());
    let config = GenesisConfig {
        use_llm_classifier: true,
    };
    let outcome = plan_or_broad(
        role_description,
        persona,
        &config,
        Some(&adapter),
        Some(&adapter),
        /* judge = */ None,
        /* allow_broad_role = */ true,
    )
    .await?;
    let mode = outcome_mode(&outcome, &adapter);
    let mut built = outcome.materialize()?;
    // DEC-025 #8: LLM-authoring pass — flag-gated (default OFF -> no-op) +
    // degrade-fail-safe. Only the from-scratch (`Broad`) path carries a spec;
    // the archetype path leaves `spec` None and is unaffected. The authored
    // persona/responsibilities/onboarding flow into prompts/system.md +
    // persona.md via the unchanged write path.
    if let Some(spec) = built.spec.take() {
        built.spec = Some(super::routes_genesis_author::run_author_pass(spec).await);
    }
    Ok((built, mode))
}

/// Honest generation-mode label for the SSE progress detail + audit row:
/// `broad` when the from-scratch materializer produced the plan, `llm` when
/// the archetype classifier picked a confident archetype, else `keyword`
/// (the deterministic fallback fired).
fn outcome_mode(outcome: &argo_genesis::GenesisOutcome, adapter: &GenesisLlmAdapter) -> &'static str {
    if outcome.archetype_slug() == argo_genesis::BROAD_SENTINEL {
        "broad"
    } else if adapter.fired() {
        "llm"
    } else {
        "keyword"
    }
}

/// Multi-trait adapter that drives a real model through the shared transport
/// dispatcher for ALL three Genesis LLM surfaces: archetype classification
/// ([`ProviderClient`]), from-scratch role synthesis ([`BroadRoleClient`]),
/// and the semantic safety judge ([`GenesisJudgeClient`]). One adapter so a
/// single `plan_or_broad` call wires every LLM seam through the same provider.
///
/// Construction is cheap (records the provider id); the transport is built
/// lazily on the first call so the keyword-only path pays no provider-init
/// cost. `fired` flips when the archetype classifier returned a recognised
/// slug, so the handler can label the generation mode honestly.
pub(super) struct GenesisLlmAdapter {
    provider_id: String,
    fired: AtomicBool,
}

impl GenesisLlmAdapter {
    /// Build an adapter for `provider_id` (see [`resolve_provider_id`]).
    pub(super) fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            fired: AtomicBool::new(false),
        }
    }

    /// `true` iff at least one classifier round-trip returned a slug that
    /// resolved to a known [`Archetype`]. Read by the handler AFTER
    /// `plan_or_broad` to label the generation mode honestly: a `false`
    /// here means the keyword fallback produced the archetype.
    pub(super) fn fired(&self) -> bool {
        self.fired.load(Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl ProviderClient for GenesisLlmAdapter {
    async fn classify(&self, prompt: &str) -> ArgoResult<String> {
        let raw = complete_once(&self.provider_id, prompt).await?;
        // The `claude-code-cli` model occasionally wraps the slug in
        // reasoning prose ("developer_advocate / not in the list /
        // research_analyst"). `argo_genesis::classify_llm` matches the
        // returned string with `Archetype::from_slug` (exact, after
        // trim/lowercase/dash-normalise), so a prose-wrapped answer
        // would silently route to the keyword fallback. Extract the
        // first recognised slug here so a sensible model answer
        // actually drives the archetype.
        let cleaned = extract_archetype_slug(&raw).unwrap_or(raw);
        // Only count the call as "fired" when the response resolved to a
        // known archetype — labelling a fallback as `llm` would be
        // theater.
        if Archetype::from_slug(&cleaned).is_some() {
            self.fired.store(true, Ordering::Relaxed);
        }
        Ok(cleaned)
    }
}

#[async_trait::async_trait]
impl BroadRoleClient for GenesisLlmAdapter {
    async fn synthesize(&self, prompt: &str) -> ArgoResult<String> {
        // The broad-role prompt asks for a JSON envelope; the genesis parser
        // is tolerant of surrounding prose, so pass the raw response through.
        // (The safety judge runs separately at the hire gate, with the
        // degraded-mode distinction — see `resolve_plan`.)
        complete_once(&self.provider_id, prompt).await
    }
}

/// Pull the first recognised archetype slug out of a (possibly
/// prose-wrapped) model response. Scans whitespace/punctuation-split
/// tokens left-to-right and returns the first that
/// [`Archetype::from_slug`] accepts, so `"...research_analyst."` and
/// `"calendar_coordinator"` both resolve. Returns `None` when no token
/// matches (the caller then keeps the raw text, which routes to the
/// keyword fallback inside `classify_llm`).
fn extract_archetype_slug(raw: &str) -> Option<String> {
    // Fast path: the whole (trimmed) response is already a valid slug.
    if Archetype::from_slug(raw).is_some() {
        return Some(raw.trim().to_string());
    }
    raw.split(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | '"' | '\'' | '`' | ':'))
        .map(str::trim)
        .find(|tok| !tok.is_empty() && Archetype::from_slug(tok).is_some())
        .map(str::to_string)
}

/// One-shot completion against the named provider manifest. Mirrors the
/// pitch path's `build_pitch_transport` exactly: load the manifest from
/// `<cwd>/manifests/providers/<id>.toml` (subprocess vs HTTP decided by
/// the manifest), then `complete` a single user turn at temperature 0.
pub(super) async fn complete_once(provider_id: &str, prompt: &str) -> ArgoResult<String> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let manifest = load_manifest(&cwd, provider_id).map_err(|e| {
        ArgoError::Provider(format!(
            "genesis llm: load provider `{provider_id}` manifest: {e}"
        ))
    })?;
    let transport = build_transport(manifest).await?;
    let req = ProviderRequest {
        model: GENESIS_MODEL.to_string(),
        messages: vec![user_message(prompt)],
        tools: Vec::new(),
        system: None,
        sampling: Sampling {
            // Deterministic classification — same role, same slug.
            temperature: Some(0.0),
            max_tokens: Some(GENESIS_MAX_TOKENS),
            ..Sampling::default()
        },
    };
    let resp: ProviderResponse = transport.complete(&req).await?;
    Ok(resp.text.unwrap_or_default())
}

fn user_message(prompt: &str) -> Message {
    Message {
        id: MessageId(Uuid::now_v7()),
        session_id: SessionId(Uuid::nil()),
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: prompt.to_string(),
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

#[cfg(test)]
#[path = "routes_genesis_stream_llm_tests.rs"]
mod tests;
