//! Provider-touching half of the Genesis LLM-authoring pass.
//!
//! The pure half ([`argo_genesis::author`]) builds the authoring prompt, parses
//! the model's JSON, and folds it into the spec. THIS half dials the model over
//! the SAME shared transport the archetype classifier + safety judge + rubric
//! use (`complete_once` → `argo_provider::Transport`), gated by
//! [`GENESIS_LLM_AUTHOR_ENV`] which now **defaults on when a provider key is
//! present** (DEC-025 full-synthesis intent; the env var stays an explicit
//! override) and degrades fail-safe.
//!
//! ## Degrade fail-safe — never a regression (the load-bearing invariant)
//! - **Pass off** (no provider key, or the env override forced off) → spec
//!   returned UNCHANGED, NO provider call. The existing templated
//!   `system_prompt_from_spec` / `persona_prompt_from_spec` path runs exactly as
//!   today (byte-identical key-less tests, zero demo impact).
//! - **Pass on, transport OR parse failed** → spec returned UNCHANGED (templated
//!   path) — `author_teammate` returns `None` on any failure.
//! - **Pass on, model produced a usable authoring** → the spec's
//!   `persona_description` / `responsibilities` / `onboarding_script` are
//!   enriched (the capability risk/approval floor is never touched).
//!
//! Adds NO new provider edge — one more narrow client over the existing
//! dispatcher + the SAME [`resolve_provider_id`] resolution.

use super::routes_genesis_stream_llm::{complete_once, resolve_provider_id};
use argo_genesis::{apply_authored, author_teammate, AuthorClient, CapabilitySpec};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Explicit override for the LLM-authoring pass. When set it wins outright
/// (`1`/`true`/`on`/`yes` forces on, `0`/`false`/`off`/`no` forces off). When
/// **unset**, the pass now defaults **on whenever a provider key is present**
/// (DEC-025 full-synthesis intent) and off otherwise — so the live gateway
/// gets the richer author pass without an extra export, while key-less test
/// runs stay on the deterministic templated path. The live gateway can still
/// pin it via `ARGO_GENESIS_LLM_AUTHOR=1` in `~/.argo/env.local`.
pub(super) const GENESIS_LLM_AUTHOR_ENV: &str = "ARGO_GENESIS_LLM_AUTHOR";

/// Whether the LLM-authoring pass should run.
///
/// Three-state, with the explicit env override always winning:
/// - [`GENESIS_LLM_AUTHOR_ENV`] truthy (`1`/`true`/`on`/`yes`) → **on**.
/// - [`GENESIS_LLM_AUTHOR_ENV`] falsy (`0`/`false`/`off`/`no`) → **off**.
/// - unset / empty / unrecognised → **default**: on when a provider key is
///   present (reuses the canonical `startup_check::provider_key_present` —
///   `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`), off when none is.
///
/// The default-on-with-key posture is still fully fail-safe: with no usable
/// provider [`run_author_pass`] degrades to the templated spec unchanged (see
/// [`author_teammate`] returning `None` on any transport/parse failure).
pub(super) fn author_enabled() -> bool {
    match std::env::var(GENESIS_LLM_AUTHOR_ENV)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "1" | "true" | "on" | "yes" => true,
        "0" | "false" | "off" | "no" => false,
        // Unset / empty / unrecognised: default on when a provider key is
        // present, off otherwise.
        _ => super::startup_check::provider_key_present(),
    }
}

/// Enrich `spec` via the LLM authoring pass, degrade-fail-safe.
///
/// - Flag off → `spec` returned unchanged (no provider call).
/// - Flag on, authoring failed (transport/parse) → `spec` unchanged.
/// - Flag on, authoring succeeded → persona/responsibilities/onboarding enriched.
pub(super) async fn run_author_pass(
    spec: CapabilitySpec,
    store: Option<&Arc<SessionStore>>,
) -> CapabilitySpec {
    if !author_enabled() {
        return spec;
    }
    let client = GateAuthorClient {
        provider_id: resolve_provider_id(),
        store: store.cloned(),
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
    /// Live store for the anonymous spend gate (`None` off the prod path).
    store: Option<Arc<SessionStore>>,
}

#[async_trait]
impl AuthorClient for GateAuthorClient {
    async fn author(&self, prompt: &str) -> ArgoResult<String> {
        // 4096 max_tokens (not the classifier's 64): authoring emits the full role
        // content (charter / prompts / skills), far larger than a slug; the HTTP
        // transport respects the cap, so 64 truncated it.
        complete_once(&self.provider_id, prompt, self.store.as_ref(), 4096).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII guard restoring the prior value of `ARGO_GENESIS_LLM_AUTHOR`.
    /// `_guard` holds the process-global env lock for the test's lifetime.
    struct AuthorEnvGuard {
        prev: Option<String>,
        _guard: tokio::sync::OwnedMutexGuard<()>,
    }

    impl AuthorEnvGuard {
        fn set(value: &str) -> Self {
            let guard = crate::handlers::gateway::shared_env_mutex().blocking_lock_owned();
            let prev = std::env::var(GENESIS_LLM_AUTHOR_ENV).ok();
            std::env::set_var(GENESIS_LLM_AUTHOR_ENV, value);
            Self { prev, _guard: guard }
        }
    }

    impl Drop for AuthorEnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(GENESIS_LLM_AUTHOR_ENV, v),
                None => std::env::remove_var(GENESIS_LLM_AUTHOR_ENV),
            }
        }
    }

    /// An explicit truthy override forces the pass ON regardless of whether a
    /// provider key happens to be present in the environment.
    #[test]
    fn explicit_truthy_override_forces_on() {
        for v in ["1", "true", "ON", "Yes", "  yes  "] {
            let _g = AuthorEnvGuard::set(v);
            assert!(author_enabled(), "`{v}` must force the author pass on");
        }
    }

    /// An explicit falsy override forces the pass OFF — this is the escape
    /// hatch the live gateway uses to opt out even when a key is present, so
    /// it must win over the default-on-with-key behaviour.
    #[test]
    fn explicit_falsy_override_forces_off() {
        for v in ["0", "false", "OFF", "No", "  no  "] {
            let _g = AuthorEnvGuard::set(v);
            assert!(!author_enabled(), "`{v}` must force the author pass off");
        }
    }
}
