//! Tests for the live provider-backed distillation caller.
//!
//! These drive [`ProviderDistillationCaller::distill`] through a STUB
//! transport (no network) and assert the live path actually consumes the
//! transport's body + model + token counts — i.e. it is NOT the
//! deterministic placeholder. The eval-gated promotion behaviour itself
//! is covered by `argo_skill`'s `self_improve_cron_tests` (Hold below
//! floor, Promote above floor, deterministic default end-to-end).

use super::*;
use argo_provider::{ProviderResponse, ProviderResponseDelta, StopReason};
use argo_skill::dreams_distillation::TrajectoryInput;
use argo_types::CapabilitySet;
use futures::stream::{self, BoxStream};

/// Stub transport returning a fixed body + model echo + token counts.
struct StubTransport {
    body: Option<String>,
    model: Option<String>,
    tokens_in: Option<u32>,
    tokens_out: Option<u32>,
    caps: CapabilitySet,
}

impl StubTransport {
    fn returning(body: &str) -> Self {
        Self {
            body: Some(body.to_string()),
            model: Some("claude-sonnet-4-6-20260101".to_string()),
            tokens_in: Some(1_000),
            tokens_out: Some(200),
            caps: CapabilitySet(Vec::new()),
        }
    }

    fn empty() -> Self {
        Self {
            body: None,
            model: None,
            tokens_in: None,
            tokens_out: None,
            caps: CapabilitySet(Vec::new()),
        }
    }
}

#[async_trait]
impl Transport for StubTransport {
    fn id(&self) -> &str {
        "stub-distill"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let provider_meta = match &self.model {
            Some(m) => serde_json::json!({ "model": m }),
            None => serde_json::json!({}),
        };
        Ok(ProviderResponse {
            text: self.body.clone(),
            reasoning: None,
            tool_calls: Vec::new(),
            stop_reason: StopReason::EndTurn,
            tokens_in: self.tokens_in,
            tokens_out: self.tokens_out,
            provider_meta,
        })
    }

    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        Ok(Box::pin(stream::empty()))
    }
}

fn cohort() -> Cohort {
    let member = |sid: &str| TrajectoryInput {
        session_id: sid.to_string(),
        action_type: "refund".to_string(),
        trajectory: vec!["lookup".into(), "verify".into(), "refund".into()],
        final_text: Some("refund confirmed".into()),
        success_metric: 0.95,
        vertical_id: "support".to_string(),
    };
    Cohort {
        action_type: "refund".to_string(),
        vertical_id: "support".to_string(),
        members: vec![member("sess-a"), member("sess-b"), member("sess-c")],
    }
}

/// Acceptance #1: a stub provider returning a KNOWN body → `distill`
/// yields a `DistillationResult` carrying that body + a NON-deterministic
/// model id (proves the live path called the transport, not the
/// placeholder).
#[tokio::test]
async fn distill_returns_the_transport_body_and_real_model() {
    let known = "# Refund\n\nProcess a refund.\n\n## Steps\n1. lookup\n2. verify\n3. refund\n";
    let caller = ProviderDistillationCaller::with_transport(
        "anthropic",
        Arc::new(StubTransport::returning(known)),
    );

    let result = caller.distill(&cohort()).await.expect("distill ok");

    assert_eq!(result.body, known, "body comes from the transport, verbatim");
    assert_ne!(
        result.model, "deterministic",
        "live path reports a real model id, not the placeholder marker"
    );
    assert_eq!(result.model, "claude-sonnet-4-6-20260101", "model echoed from provider_meta");
    // cost = 1000 * 3 (in) + 200 * 15 (out) = 6000 micros.
    assert_eq!(result.cost_micros, 6_000, "cost estimated from token counts");
}

/// An empty model response is a clean `ArgoError::Provider`, not a panic —
/// a no-key / degraded run degrades gracefully.
#[tokio::test]
async fn empty_response_is_a_provider_error_not_a_panic() {
    let caller =
        ProviderDistillationCaller::with_transport("anthropic", Arc::new(StubTransport::empty()));
    let err = caller.distill(&cohort()).await.expect_err("empty body errors");
    assert!(matches!(err, ArgoError::Provider(_)), "got {err:?}");
}

/// The synthesis prompt generalises the cohort: it names the action,
/// the vertical, the run count, and includes each run's steps — proving
/// the model is asked to synthesise, not to echo one transcript.
#[test]
fn synthesis_prompt_carries_the_whole_cohort() {
    let prompt = render_synthesis_prompt(&cohort());
    assert!(prompt.contains("refund"), "names the action type");
    assert!(prompt.contains("support"), "names the vertical");
    assert!(prompt.contains("3 successful"), "names the cohort size");
    assert!(prompt.contains("Run 1"), "includes per-run sections");
    assert!(prompt.contains("Run 3"), "includes every run");
    assert!(prompt.contains("- lookup"), "includes trajectory steps");
    assert!(prompt.contains("refund confirmed"), "includes final output");
}

/// Cost estimate is 0 when the provider reports no token counts (still a
/// valid result — the daily cap just charges nothing).
#[test]
fn cost_is_zero_without_token_counts() {
    assert_eq!(estimate_cost_micros(None, None), 0);
    assert_eq!(estimate_cost_micros(Some(100), None), 300);
    assert_eq!(estimate_cost_micros(None, Some(100)), 1_500);
}

/// `from_env` defaults to the anthropic manifest when the env var is unset.
#[test]
fn from_env_defaults_to_anthropic() {
    // Don't mutate process env (parallel tests); assert the default const
    // is what `new` stores and that the env name is the documented one.
    let caller = ProviderDistillationCaller::new(DEFAULT_DISTILL_PROVIDER);
    assert_eq!(caller.provider_id, "anthropic");
    assert_eq!(ENV_DISTILL_PROVIDER, "ARGO_SKILL_DISTILL_PROVIDER");
}
