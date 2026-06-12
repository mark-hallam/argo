//! Real-provider half of the Ghostwriter draft path
//! (`POST /api/v1/skills/draft-from-prompt`).
//!
//! `routes_ghostwriter` historically wired the deterministic
//! [`OfflineGhostwriterClient`] unconditionally, so every draft was canned.
//! This module turns the gated real-LLM path on: it implements the narrow
//! [`GhostwriterClient`] trait against the SAME shared transport the Genesis
//! author / archetype classifier and the Pitch Agent use
//! (`argo_agent_runner::{build_transport, load_manifest}` →
//! `argo_provider::Transport`), reusing the genesis
//! [`resolve_provider_id`](super::routes_genesis_stream_llm::resolve_provider_id)
//! resolution (DEC-012 dev/demo default `claude-code-cli`).
//!
//! ## Degrade fail-safe — never a 500, never silent theater
//! [`draft_with_fallback`] tries the real provider FIRST when a provider is
//! configured, then falls back to the deterministic offline template on ANY
//! failure (transport/decode/validation), tagging the response `source` so the
//! caller can tell a real draft (`provider`) from the canned fallback
//! (`offline`). This is the NO_THEATER posture: when a real key is present and
//! working the operator gets a real model draft; when it isn't, the endpoint
//! still returns a valid IR rather than erroring.
//!
//! The gate mirrors `routes_genesis_author::author_enabled`: an explicit env
//! override always wins, else it defaults **on when a provider key is present**
//! (so the live gateway gets the real path without an extra export, while
//! key-less test runs stay deterministic).

use super::routes_genesis_stream_llm::resolve_provider_id;
use argo_agent_runner::{build_transport, load_manifest, TransportGate};
use argo_provider::{ProviderRequest, ProviderResponse, Sampling};
use argo_skill::ir::{generate_from_prompt, GhostwriterClient, GhostwriterError, SkillGraph};
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use async_trait::async_trait;
use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

/// Explicit override for the Ghostwriter real-LLM draft pass. When set it wins
/// outright (`1`/`true`/`on`/`yes` forces on, `0`/`false`/`off`/`no` forces
/// off). When **unset**, the pass defaults **on whenever a provider key is
/// present** (mirrors `ARGO_GENESIS_LLM_AUTHOR`) and off otherwise — so the
/// live gateway gets the real draft without an extra export, while key-less
/// test runs stay on the deterministic offline template.
pub(super) const GHOSTWRITER_LLM_ENV: &str = "ARGO_GHOSTWRITER_LLM";

/// Default draft model. The `claude-code-cli` subprocess driver ignores this
/// (the CLI auto-routes), but the HTTP families need a concrete id, so it is
/// pinned here rather than `"latest"`. Mirrors `GENESIS_MODEL`.
const GHOSTWRITER_MODEL: &str = "claude-sonnet-4-6";

/// A full `SkillGraph` JSON is far larger than the genesis classifier's
/// one-slug answer, so this path needs generous headroom (the genesis
/// `complete_once` caps at 64 tokens and would truncate the graph).
const GHOSTWRITER_MAX_TOKENS: u32 = 4096;

/// Whether the real-LLM draft pass should run.
///
/// Three-state, with the explicit env override always winning:
/// - [`GHOSTWRITER_LLM_ENV`] truthy (`1`/`true`/`on`/`yes`) → **on**.
/// - [`GHOSTWRITER_LLM_ENV`] falsy (`0`/`false`/`off`/`no`) → **off**.
/// - unset / empty / unrecognised → **default**: on when a provider key is
///   present (reuses the canonical `startup_check::provider_key_present`),
///   off when none is.
pub(super) fn ghostwriter_llm_enabled() -> bool {
    match std::env::var(GHOSTWRITER_LLM_ENV)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "1" | "true" | "on" | "yes" => true,
        "0" | "false" | "off" | "no" => false,
        _ => super::startup_check::provider_key_present(),
    }
}

/// The real-provider draft client if the pass is enabled, else `None`.
/// `None` makes [`draft_with_fallback`] take the deterministic path with no
/// provider call at all (the key-less test / demo default).
pub(super) fn provider_client_if_enabled(
    store: Option<&Arc<SessionStore>>,
) -> Option<GhostwriterProviderClient> {
    ghostwriter_llm_enabled().then(|| GhostwriterProviderClient {
        provider_id: resolve_provider_id(),
        store: store.cloned(),
    })
}

/// Draft a `SkillGraph`, real-provider-first with deterministic fallback.
///
/// - `primary` present + succeeds → `(graph, "provider")`.
/// - `primary` present + fails (transport/decode/validation) → fall back to
///   `fallback`, returning `(graph, "offline")`. Graceful degrade, no error
///   surfaced to the caller for a provider hiccup.
/// - `primary` `None` (pass disabled) → `fallback` directly → `(graph,
///   "offline")`.
///
/// Only a failure of the deterministic `fallback` itself bubbles up as an
/// `Err` (effectively never, since the offline template is dependency-free).
pub(super) async fn draft_with_fallback(
    prompt: &str,
    primary: Option<&dyn GhostwriterClient>,
    fallback: &dyn GhostwriterClient,
) -> Result<(SkillGraph, &'static str), GhostwriterError> {
    if let Some(primary) = primary {
        match generate_from_prompt(prompt, primary).await {
            Ok(graph) => return Ok((graph, "provider")),
            Err(err) => {
                tracing::warn!(
                    target: "gateway::ghostwriter",
                    error = %err,
                    "ghostwriter provider draft failed; falling back to offline template",
                );
            }
        }
    }
    let graph = generate_from_prompt(prompt, fallback).await?;
    Ok((graph, "offline"))
}

/// [`GhostwriterClient`] over the shared provider transport — the SAME
/// machinery as the Genesis author / classifier and the Pitch Agent.
pub(super) struct GhostwriterProviderClient {
    provider_id: String,
    /// Live store for the anonymous spend gate (`None` off the prod path).
    store: Option<Arc<SessionStore>>,
}

#[async_trait]
impl GhostwriterClient for GhostwriterProviderClient {
    async fn generate(&self, prompt: &str) -> ArgoResult<String> {
        complete_draft(&self.provider_id, prompt, self.store.as_ref()).await
    }
}

/// One-shot draft completion against the named provider manifest. Mirrors the
/// genesis `complete_once` exactly (load the manifest from
/// `<cwd>/manifests/providers/<id>.toml`, subprocess-vs-HTTP decided by the
/// manifest), but with a draft-sized [`GHOSTWRITER_MAX_TOKENS`] budget and a
/// low (not zero) temperature so the graph stays well-formed without being
/// rigidly canned.
async fn complete_draft(
    provider_id: &str,
    prompt: &str,
    store: Option<&Arc<SessionStore>>,
) -> ArgoResult<String> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let manifest = load_manifest(&cwd, provider_id).map_err(|e| {
        ArgoError::Provider(format!(
            "ghostwriter llm: load provider `{provider_id}` manifest: {e}"
        ))
    })?;
    // Internet-facing gateway draft path: gate against the capped anonymous
    // tenant (WS5 open-wallet close).
    let transport = build_transport(manifest, TransportGate::Anonymous { store, cwd: &cwd }).await?;
    let req = ProviderRequest {
        model: GHOSTWRITER_MODEL.to_string(),
        messages: vec![user_message(prompt)],
        tools: Vec::new(),
        system: None,
        sampling: Sampling {
            temperature: Some(0.2),
            max_tokens: Some(GHOSTWRITER_MAX_TOKENS),
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
mod tests {
    use super::*;
    use argo_skill::ir::OfflineGhostwriterClient;

    /// RAII guard restoring the prior value of `ARGO_GHOSTWRITER_LLM`.
    struct LlmEnvGuard {
        prev: Option<String>,
        _guard: tokio::sync::OwnedMutexGuard<()>,
    }

    impl LlmEnvGuard {
        fn set(value: &str) -> Self {
            let guard = crate::handlers::gateway::shared_env_mutex().blocking_lock_owned();
            let prev = std::env::var(GHOSTWRITER_LLM_ENV).ok();
            std::env::set_var(GHOSTWRITER_LLM_ENV, value);
            Self { prev, _guard: guard }
        }
    }

    impl Drop for LlmEnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(GHOSTWRITER_LLM_ENV, v),
                None => std::env::remove_var(GHOSTWRITER_LLM_ENV),
            }
        }
    }

    /// A stub client returning a fixed, valid single-node `SkillGraph` JSON.
    /// Its label is distinct from the offline template so a test can prove
    /// the real path's output (not the canned fallback) came back.
    struct OkStub;

    const STUB_LABEL: &str = "STUB-PROVIDER-DRAFT";

    #[async_trait]
    impl GhostwriterClient for OkStub {
        async fn generate(&self, _prompt: &str) -> ArgoResult<String> {
            Ok(format!(
                r#"{{"entry":"n1","nodes":[{{"id":"n1","label":"{STUB_LABEL}","kind":{{"type":"action","instruction":"do the thing"}}}}],"edges":[]}}"#
            ))
        }
    }

    /// A stub client that always errors — stands in for a transport that
    /// failed to dial out (no key, manifest missing, subprocess died).
    struct ErrStub;

    #[async_trait]
    impl GhostwriterClient for ErrStub {
        async fn generate(&self, _prompt: &str) -> ArgoResult<String> {
            Err(ArgoError::Provider("forced transport error".into()))
        }
    }

    /// An explicit truthy override forces the pass ON regardless of key.
    #[test]
    fn explicit_truthy_override_forces_on() {
        for v in ["1", "true", "ON", "Yes", "  yes  "] {
            let _g = LlmEnvGuard::set(v);
            assert!(ghostwriter_llm_enabled(), "`{v}` must force the pass on");
        }
    }

    /// An explicit falsy override forces the pass OFF even when a key is set.
    #[test]
    fn explicit_falsy_override_forces_off() {
        for v in ["0", "false", "OFF", "No", "  no  "] {
            let _g = LlmEnvGuard::set(v);
            assert!(!ghostwriter_llm_enabled(), "`{v}` must force the pass off");
        }
    }

    /// Real path present + succeeds → the draft is the provider's output and
    /// `source` reads `provider`, NOT the offline canned text.
    #[tokio::test]
    async fn provider_path_taken_when_primary_succeeds() {
        let primary = OkStub;
        let fallback = OfflineGhostwriterClient;
        let (graph, source) =
            draft_with_fallback("Build a skill that does the thing", Some(&primary), &fallback)
                .await
                .expect("offline fallback never errors and primary succeeded");
        assert_eq!(source, "provider");
        assert_eq!(graph.nodes.len(), 1, "should be the stub's single-node graph");
        assert_eq!(graph.nodes[0].label, STUB_LABEL);
    }

    /// Primary forced to error → deterministic offline fallback engages
    /// (graceful degrade, `source: offline`, real graph, no error).
    #[tokio::test]
    async fn falls_back_to_offline_when_primary_errors() {
        let primary = ErrStub;
        let fallback = OfflineGhostwriterClient;
        let (graph, source) =
            draft_with_fallback("Build a skill that does the thing", Some(&primary), &fallback)
                .await
                .expect("offline fallback must yield a valid graph");
        assert_eq!(source, "offline");
        // The offline template is the 5-node fetch→…→act shape, definitely
        // not the stub's single node.
        assert!(graph.nodes.len() > 1, "offline template is multi-node");
        assert_ne!(graph.nodes[0].label, STUB_LABEL);
    }

    /// Pass disabled (`primary == None`) → offline directly, `source: offline`.
    #[tokio::test]
    async fn offline_directly_when_primary_absent() {
        let fallback = OfflineGhostwriterClient;
        let (graph, source) =
            draft_with_fallback("Build a skill that does the thing", None, &fallback)
                .await
                .expect("offline fallback must yield a valid graph");
        assert_eq!(source, "offline");
        assert!(graph.nodes.len() > 1);
    }
}
