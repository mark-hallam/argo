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
