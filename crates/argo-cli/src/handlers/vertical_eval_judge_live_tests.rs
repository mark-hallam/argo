//! Unit tests for the live LLM judge seam (#701). Uses a mock
//! `JudgeBackend` so the full grade-fixture path runs end-to-
//! end with no network access. CI-safe — no API key needed.
#![cfg(test)]

use super::*;
use argo_vertical::{score_fixture, FixtureScore};
use std::sync::Mutex;

/// Hand-rolled mock backend. Captures every prompt it sees so
/// tests can assert prompt determinism, and returns canned
/// responses keyed by a counter so a single backend can drive
/// multi-fixture grading.
struct MockBackend {
    /// Pre-canned responses, returned in order.
    responses: Mutex<Vec<String>>,
    /// Captured prompts for assertion.
    prompts: Mutex<Vec<String>>,
    /// Optional error to inject — returned from the next
    /// `grade()` call instead of a response.
    error: Mutex<Option<String>>,
}

impl MockBackend {
    fn with_responses(responses: Vec<&str>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().rev().map(String::from).collect()),
            prompts: Mutex::new(Vec::new()),
            error: Mutex::new(None),
        }
    }

    fn with_error(msg: &str) -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            prompts: Mutex::new(Vec::new()),
            error: Mutex::new(Some(msg.into())),
        }
    }

    fn prompts_seen(&self) -> Vec<String> {
        self.prompts.lock().unwrap().clone()
    }
}

#[async_trait]
impl JudgeBackend for MockBackend {
    fn source(&self) -> String {
        "mock:test".into()
    }

    async fn grade(&self, prompt: &str) -> ArgoResult<String> {
        self.prompts.lock().unwrap().push(prompt.to_string());
        if let Some(msg) = self.error.lock().unwrap().take() {
            return Err(ArgoError::Provider(msg));
        }
        let mut q = self.responses.lock().unwrap();
        Ok(q.pop().unwrap_or_else(|| {
            r#"```json
{"verdict":"uncertain","score":0.5,"rationale":"no canned response","failed_criteria":[]}
```"#
                .to_string()
        }))
    }
}

fn fx_from_row(row: &serde_json::Value) -> FixtureScore {
    score_fixture(row)
}

#[tokio::test]
async fn grade_fixture_dispatches_prompt_and_parses_pass() {
    let backend = MockBackend::with_responses(vec![
        r#"```json
{"verdict":"pass","score":0.9,"rationale":"semantically aligned","failed_criteria":[]}
```"#,
    ]);
    let v = grade_fixture(
        &backend,
        "Define an ICP for SKYCOT",
        &serde_json::json!({"must_include": ["role-based agents"]}),
        "ICP centered on role-based AI agents.",
    )
    .await;
    assert!(v.passed);
    assert_eq!(v.score, Some(0.9));
    assert_eq!(v.source, "mock:test");
    let prompts = backend.prompts_seen();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("Define an ICP for SKYCOT"));
    assert!(prompts[0].contains("ICP centered on role-based AI agents."));
}

#[tokio::test]
async fn grade_fixture_returns_uncertain_when_backend_errors() {
    let backend = MockBackend::with_error("transport blew up");
    let v = grade_fixture(
        &backend,
        "x",
        &serde_json::json!({}),
        "y",
    )
    .await;
    assert!(!v.passed);
    assert_eq!(v.score, Some(0.5));
    assert!(v.reason.starts_with("uncertain:"), "{}", v.reason);
    assert!(v.reason.contains("transport blew up"), "{}", v.reason);
    assert!(v.failed_criteria.contains(&"judge_call_failed".into()));
}

#[tokio::test]
async fn judge_all_fixtures_skips_no_actual_and_malformed_rows() {
    // Suite of three: one Scored, one NoActual, one Malformed
    // (forced via missing `expected`). Backend should be
    // invoked exactly once.
    let scored_row = serde_json::json!({
        "id": "s",
        "input": "in",
        "expected": {"must_include": ["x"]},
        "actual": "x"
    });
    let no_actual_row = serde_json::json!({
        "id": "n",
        "input": "in",
        "expected": {}
    });
    let malformed_row = serde_json::json!({"id": "m", "input": "in"});
    let scores = vec![
        fx_from_row(&scored_row),
        fx_from_row(&no_actual_row),
        fx_from_row(&malformed_row),
    ];
    let inputs = vec![(
        "s".into(),
        "in".into(),
        serde_json::json!({"must_include": ["x"]}),
        "x".into(),
    )];
    let backend = MockBackend::with_responses(vec![
        r#"```json
{"verdict":"pass","score":1.0,"rationale":"ok","failed_criteria":[]}
```"#,
    ]);
    let verdicts = judge_all_fixtures(&backend, &scores, &inputs).await;
    assert_eq!(verdicts.len(), 3);
    assert!(verdicts[0].is_some(), "scored fixture should be judged");
    assert!(verdicts[1].is_none(), "no_actual fixture should NOT be judged");
    assert!(verdicts[2].is_none(), "malformed fixture should NOT be judged");
    assert_eq!(backend.prompts_seen().len(), 1, "backend invoked once");
}

#[tokio::test]
async fn judge_all_fixtures_handles_input_id_mismatch_as_no_judge() {
    // If a Scored fixture's id doesn't appear in
    // fixture_inputs (caller bug or data drift), the helper
    // skips it rather than calling the backend with empty
    // strings. Defensive — surfaces drift instead of grading
    // garbage.
    let scored_row = serde_json::json!({
        "id": "scored_id",
        "input": "in",
        "expected": {"must_include": ["x"]},
        "actual": "x"
    });
    let scores = vec![fx_from_row(&scored_row)];
    let inputs = vec![(
        "different_id".into(),
        "in".into(),
        serde_json::json!({}),
        "x".into(),
    )];
    let backend = MockBackend::with_responses(vec![]);
    let verdicts = judge_all_fixtures(&backend, &scores, &inputs).await;
    assert_eq!(verdicts.len(), 1);
    assert!(verdicts[0].is_none());
    assert_eq!(backend.prompts_seen().len(), 0);
}

// ---- temperature policy regression (#701 hotfix) ---------------

#[test]
fn judge_temperature_for_thinking_mode_is_one_not_zero() {
    // Anthropic rejects requests with `temperature != 1` when
    // `thinking` is enabled (HTTP 400: "temperature may only
    // be set to 1 when thinking is enabled"). The shipped
    // anthropic.toml manifest enables thinking via the
    // send-thinking-blocks quirk, so the live judge MUST send
    // 1.0 — regression-locked here so a future edit can't
    // silently re-introduce the 0.0 default.
    assert_eq!(
        judge_temperature_for(true),
        1.0,
        "thinking-mode requires temperature=1.0 to satisfy Anthropic's API contract"
    );
    // Symmetric: when thinking is OFF, deterministic 0.0 is
    // the right pick so verdicts are stable across runs.
    assert_eq!(judge_temperature_for(false), 0.0);
    // Belt-and-braces: never accidentally re-introduce 0.0 in
    // the thinking branch.
    assert_ne!(
        judge_temperature_for(true),
        0.0,
        "regression guard: temperature=0.0 in thinking-mode triggers HTTP 400"
    );
}

// ---- max_tokens policy regression (#701c hotfix) ---------------

#[test]
fn judge_max_tokens_for_thinking_mode_strictly_exceeds_thinking_budget() {
    // Anthropic returns HTTP 400 when `max_tokens <=
    // thinking.budget_tokens`. The transport hardcodes
    // budget_tokens=1024, so the live judge MUST request
    // strictly more than that. This regression lock prevents
    // a future tweak (e.g. "let's save tokens by setting
    // max_tokens=1024") from re-introducing the 400.
    let max = judge_max_tokens_for(true);
    assert!(
        max > ANTHROPIC_THINKING_BUDGET_TOKENS,
        "max_tokens ({max}) MUST be strictly greater than thinking.budget_tokens ({ANTHROPIC_THINKING_BUDGET_TOKENS}) — Anthropic returns HTTP 400 otherwise"
    );
    // Symmetric: thinking off → smaller cap is fine.
    let off = judge_max_tokens_for(false);
    assert!(off < max, "thinking-off mode can use a tighter cap");
    assert!(
        off > 0,
        "even compact JSON needs a non-zero cap to render"
    );
}

#[test]
fn anthropic_thinking_budget_constant_matches_transport_hardcoded_value() {
    // The transport's `apply_quirks` hardcodes 1024 in
    // `argo_provider::anthropic::wire`. We mirror that value
    // here so `judge_max_tokens_for` can compute the headroom
    // arithmetically. If the transport ever changes its
    // budget, this test fails — which is exactly the signal
    // we want.
    assert_eq!(
        ANTHROPIC_THINKING_BUDGET_TOKENS, 1024,
        "if the transport's hardcoded budget changes, update ANTHROPIC_THINKING_BUDGET_TOKENS to match"
    );
}

// ---- existing tests below ---------------------------------------

#[test]
fn shipped_anthropic_manifest_enables_thinking_so_temperature_must_be_one() {
    // End-to-end regression lock: the manifest the live
    // judge ACTUALLY loads in production
    // (`manifests/providers/anthropic.toml`) has
    // send-thinking-blocks=true, and our temperature policy
    // must recognise that and pick 1.0. If a future edit
    // changes either side, this test fails fast with a
    // pointer to the constraint.
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root");
    let body = std::fs::read_to_string(
        workspace_root.join("manifests/providers/anthropic.toml"),
    )
    .expect("shipped anthropic manifest readable from workspace root");
    let m = argo_provider::ProviderManifest::from_toml_str(&body)
        .expect("shipped manifest parses");
    let thinking = manifest_enables_thinking(&m);
    assert!(
        thinking,
        "shipped anthropic.toml must enable thinking — without it the constraint flips and \
         this test no longer guards anything; if the manifest changes intentionally, update \
         this test to mirror the new policy."
    );
    assert_eq!(
        judge_temperature_for(thinking),
        1.0,
        "temperature picked from shipped manifest must be 1.0 — anything else 400s in production"
    );
}

#[test]
fn build_anthropic_backend_errors_clearly_when_manifest_missing() {
    // When the cwd doesn't have `manifests/providers/<id>.toml`,
    // operators should get a precise message naming the path
    // they need to create. We can't easily test the IO failure
    // without picking a known-missing id; choose one that
    // definitely doesn't exist + assert the error mentions
    // both the provider id and the expected path.
    let result = build_anthropic_backend(
        "definitely-not-a-real-provider-id-12345",
        "claude-x",
    );
    assert!(result.is_err(), "missing manifest must fail");
    let msg = format!("{}", result.err().unwrap());
    assert!(
        msg.contains("definitely-not-a-real-provider-id-12345"),
        "{msg}"
    );
    assert!(
        msg.contains("manifests/providers"),
        "{msg}"
    );
}
