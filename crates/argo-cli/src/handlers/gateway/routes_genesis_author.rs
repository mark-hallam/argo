//! Provider-touching half of the Genesis LLM-authoring pass.
//!
//! The pure half ([`argo_genesis::author`]) builds the authoring prompt, parses
//! the model's JSON, and folds it into the spec. THIS half dials the model over
//! the SAME shared transport the archetype classifier + safety judge + rubric
//! use (`complete_once` → `argo_provider::Transport`), flag-gated by
//! [`GENESIS_LLM_AUTHOR_ENV`] (mirroring `ARGO_GENESIS_LLM_CLASSIFIER` /
//! `ARGO_GENESIS_LLM_RUBRIC`), and degrades fail-safe.
//!
//! ## Degrade fail-safe — never a regression (the load-bearing invariant)
//! - **Flag off** (test/default posture) → spec returned UNCHANGED, NO provider
//!   call. The existing templated `system_prompt_from_spec` /
//!   `persona_prompt_from_spec` path runs exactly as today (byte-identical
//!   tests, zero demo impact).
//! - **Flag on, transport OR parse failed** → spec returned UNCHANGED (templated
//!   path) — `author_teammate` returns `None` on any failure.
//! - **Flag on, model produced a usable authoring** → the spec's
//!   `persona_description` / `responsibilities` / `onboarding_script` are
//!   enriched (the capability risk/approval floor is never touched).
//!
//! Adds NO new provider edge — one more narrow client over the existing
//! dispatcher + the SAME [`resolve_provider_id`] resolution.

use super::routes_genesis_stream_llm::{complete_once, resolve_provider_id};
use argo_genesis::{apply_authored, author_teammate, AuthorClient, CapabilitySpec};
use argo_types::ArgoResult;
use async_trait::async_trait;

/// Explicit opt-in for the LLM-authoring pass. Defaults **off** so test runs
/// stay deterministic (no dial-out) and the templated path is unchanged. The
/// live gateway turns it on via `ARGO_GENESIS_LLM_AUTHOR=1` in `~/.argo/env.local`.
pub(super) const GENESIS_LLM_AUTHOR_ENV: &str = "ARGO_GENESIS_LLM_AUTHOR";

/// `true` when [`GENESIS_LLM_AUTHOR_ENV`] is truthy (`1`/`true`/`on`/`yes`,
/// case-insensitive). Absent/empty/other reads off — the default-off posture.
pub(super) fn author_enabled() -> bool {
    matches!(
        std::env::var(GENESIS_LLM_AUTHOR_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Enrich `spec` via the LLM authoring pass, degrade-fail-safe.
///
/// - Flag off → `spec` returned unchanged (no provider call).
/// - Flag on, authoring failed (transport/parse) → `spec` unchanged.
/// - Flag on, authoring succeeded → persona/responsibilities/onboarding enriched.
pub(super) async fn run_author_pass(spec: CapabilitySpec) -> CapabilitySpec {
    if !author_enabled() {
        return spec;
    }
    let client = GateAuthorClient {
        provider_id: resolve_provider_id(),
    };
    match author_teammate(&spec, &client).await {
        Some(authored) => apply_authored(spec, authored),
        None => spec,
    }
}

/// [`AuthorClient`] over the shared provider transport — the SAME machinery as
/// the archetype classifier / safety judge / rubric grader.
struct GateAuthorClient {
    provider_id: String,
}

#[async_trait]
impl AuthorClient for GateAuthorClient {
    async fn author(&self, prompt: &str) -> ArgoResult<String> {
        complete_once(&self.provider_id, prompt).await
    }
}
