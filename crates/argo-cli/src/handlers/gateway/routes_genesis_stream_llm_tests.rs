//! Unit tests for the Genesis real-LLM classifier adapter.
//!
//! These cover the pure, provider-free surface: env-var precedence for
//! [`resolve_provider_id`], the `fired()` honesty flag, and the
//! keyword-fallback path of [`resolve_plan`] when no provider manifest
//! resolves. The live provider round-trip is exercised end-to-end by
//! the gateway smoke test against `:7400` (documented in the brief),
//! not here — dialing a real model from a unit test would be flaky.
use super::*;

struct EnvGuard {
    keys: Vec<(&'static str, Option<String>)>,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl EnvGuard {
    async fn apply(set: &[(&'static str, &str)], unset: &[&'static str]) -> Self {
        let guard = crate::handlers::gateway::shared_env_lock().await;
        let mut keys = Vec::with_capacity(set.len() + unset.len());
        for (k, v) in set {
            keys.push((*k, std::env::var(k).ok()));
            std::env::set_var(k, v);
        }
        for k in unset {
            keys.push((*k, std::env::var(k).ok()));
            std::env::remove_var(k);
        }
        Self { keys, _guard: guard }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, old) in &self.keys {
            match old {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

#[tokio::test]
async fn provider_id_defaults_to_claude_code_cli() {
    let _env = EnvGuard::apply(&[], &[GENESIS_LLM_PROVIDER_ENV, "ARGO_PITCH_LLM_PROVIDER"]).await;
    assert_eq!(resolve_provider_id(), "claude-code-cli");
}

#[tokio::test]
async fn provider_id_prefers_genesis_specific_env() {
    let _env = EnvGuard::apply(
        &[
            (GENESIS_LLM_PROVIDER_ENV, "anthropic"),
            ("ARGO_PITCH_LLM_PROVIDER", "claude-code-cli"),
        ],
        &[],
    ).await;
    assert_eq!(resolve_provider_id(), "anthropic");
}

#[tokio::test]
async fn provider_id_falls_back_to_pitch_env() {
    // No Genesis-specific override → reuse the pitch path's provider so
    // one dev/demo export drives both LLM surfaces (DEC-012).
    let _env = EnvGuard::apply(
        &[("ARGO_PITCH_LLM_PROVIDER", "claude-code-cli")],
        &[GENESIS_LLM_PROVIDER_ENV],
    ).await;
    assert_eq!(resolve_provider_id(), "claude-code-cli");
}

#[tokio::test]
async fn provider_id_ignores_blank_env() {
    let _env = EnvGuard::apply(
        &[
            (GENESIS_LLM_PROVIDER_ENV, "   "),
            ("ARGO_PITCH_LLM_PROVIDER", ""),
        ],
        &[],
    ).await;
    assert_eq!(resolve_provider_id(), "claude-code-cli");
}

#[test]
fn classifier_starts_not_fired() {
    let c = GenesisLlmAdapter::new("claude-code-cli".to_string(), None);
    assert!(!c.fired(), "fired() must be false before any provider call");
}

#[test]
fn extract_slug_handles_clean_and_prose_wrapped_answers() {
    // Clean single-slug answer (the common case).
    assert_eq!(
        extract_archetype_slug("calendar_coordinator").as_deref(),
        Some("calendar_coordinator")
    );
    // Trailing punctuation.
    assert_eq!(
        extract_archetype_slug("research_analyst.").as_deref(),
        Some("research_analyst")
    );
    // Real prose-wrapped shape observed from claude-code-cli on a
    // developer-relations charter — must skip the non-slug tokens and
    // land on the first recognised archetype.
    assert_eq!(
        extract_archetype_slug(
            "developer_advocate\n\nWait, that's not in the list.\n\nresearch_analyst"
        )
        .as_deref(),
        Some("research_analyst")
    );
    // Dash form normalises (argo_genesis accepts `-` for `_`).
    assert_eq!(extract_archetype_slug("Sales-SDR").as_deref(), Some("Sales-SDR"));
    // No recognised slug → None (caller keeps raw → keyword fallback).
    assert_eq!(extract_archetype_slug("i have no idea honestly"), None);
}

#[tokio::test]
async fn llm_classifier_off_by_default() {
    let _env = EnvGuard::apply(&[], &[GENESIS_LLM_CLASSIFIER_ENV]).await;
    assert!(
        !llm_classifier_enabled(),
        "classifier must default OFF so tests stay on the deterministic path"
    );
}

#[tokio::test]
async fn llm_classifier_truthy_values_enable() {
    for v in ["1", "true", "ON", "Yes"] {
        let _env = EnvGuard::apply(&[(GENESIS_LLM_CLASSIFIER_ENV, v)], &[]).await;
        assert!(llm_classifier_enabled(), "`{v}` should enable the classifier");
    }
    for v in ["0", "false", "off", "", "  "] {
        let _env = EnvGuard::apply(&[(GENESIS_LLM_CLASSIFIER_ENV, v)], &[]).await;
        assert!(!llm_classifier_enabled(), "`{v}` should NOT enable");
    }
}

#[tokio::test]
async fn resolve_plan_uses_keyword_when_classifier_disabled() {
    // Default (flag off): no provider call, deterministic archetype,
    // mode label "keyword". This is the test + safety default.
    let _env = EnvGuard::apply(&[], &[GENESIS_LLM_CLASSIFIER_ENV]).await;
    let (plan, mode) = resolve_plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        None,
    )
    .await
    .expect("keyword plan builds");
    assert_eq!(mode, "keyword");
    assert_eq!(plan.archetype, Archetype::MarketingIntern);
    assert_eq!(plan.persona, "maya");
}

#[tokio::test]
async fn resolve_plan_falls_back_to_keyword_when_provider_manifest_missing() {
    // Flag ON but no resolvable manifest → `load_manifest` fails, the
    // classifier swallows the error, the keyword scorer produces the
    // archetype, and the mode label MUST report "keyword" (no theater:
    // the LLM did not actually fire).
    let tmp = tempfile::tempdir().expect("tempdir");
    let _env = EnvGuard::apply(
        &[
            (GENESIS_LLM_CLASSIFIER_ENV, "1"),
            ("ARGO_MANIFESTS_DIR", tmp.path().to_str().unwrap()),
            (GENESIS_LLM_PROVIDER_ENV, "no-such-provider"),
        ],
        &["ARGO_PITCH_LLM_PROVIDER"],
    ).await;
    let (plan, mode) = resolve_plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        None,
    )
    .await
    .expect("plan builds via keyword fallback");
    assert_eq!(mode, "keyword", "missing manifest must report keyword mode");
    assert_eq!(plan.archetype, Archetype::MarketingIntern);
}
