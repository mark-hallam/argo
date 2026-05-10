//! Propose-for-review eval gate + outcome matching.
//!
//! Sibling of [`crate::eval_runner`]. Holds the glue that
//! [`crate::SkillStore::propose_for_review_judged_with_evals`] consumes:
//! a Pass/Reject signal derived from running the declared eval set
//! against a caller-supplied executor and applying the pass-rate floor.
//! The predicate matcher lives here too, since "did the actual outcome
//! match the expected predicate" is the same conceptual layer as the
//! gate itself — both decide pass-vs-fail.

use crate::eval_runner::{run_evals, ActualOutcome, EvalExecutor, DEFAULT_EVAL_PASS_RATE_THRESHOLD};
use crate::skill::{ExpectedOutcome, LifecycleState, Skill, SkillFrontmatter};
use argo_types::SkillId;
use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Pass/Reject signal that the propose-for-review path consumes when
/// applying the eval-runner gate.
#[derive(Debug, PartialEq)]
pub(crate) enum EvalGateOutcome {
    Pass,
    Reject(String),
}

/// Run the declared eval set against the supplied executor and apply
/// the pass-rate floor. Used by
/// [`crate::SkillStore::propose_for_review_judged_with_evals`] — the
/// thin glue keeps the store_vertical file under the 500-LOC ceiling.
///
/// Returns `Pass` when no executor is provided, when the frontmatter
/// declares no cases, or when the rate clears [`DEFAULT_EVAL_PASS_RATE_THRESHOLD`].
/// Returns `Reject` (with a human-readable reason) when the rate
/// strictly trails the floor or [`run_evals`] errors at the layer above
/// individual case failures.
pub(crate) async fn run_propose_eval_gate(
    frontmatter: &SkillFrontmatter,
    body: &str,
    executor: Option<&dyn EvalExecutor>,
) -> EvalGateOutcome {
    let executor = match executor {
        Some(e) => e,
        None => return EvalGateOutcome::Pass,
    };
    if frontmatter.evals.is_empty() {
        return EvalGateOutcome::Pass;
    }
    let temp = synthetic_skill_for_eval(frontmatter.clone(), body);
    match run_evals(&temp, executor, &frontmatter.evals).await {
        Ok(report) => {
            if report.passes_threshold(DEFAULT_EVAL_PASS_RATE_THRESHOLD) {
                EvalGateOutcome::Pass
            } else {
                EvalGateOutcome::Reject(format!(
                    "eval_low_confidence: pass-rate {:.2} < floor {:.2} ({}/{} cases passed)",
                    report.rate,
                    DEFAULT_EVAL_PASS_RATE_THRESHOLD,
                    report.pass,
                    report.pass + report.fail
                ))
            }
        }
        Err(e) => EvalGateOutcome::Reject(format!("eval_runner_error: {e}")),
    }
}

fn synthetic_skill_for_eval(frontmatter: SkillFrontmatter, body: &str) -> Skill {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    Skill {
        id: SkillId(uuid::Uuid::nil()),
        frontmatter,
        body: body.to_string(),
        hash: h.finalize().into(),
        learned_in: None,
        superseded_by: None,
        created_at: Utc::now(),
        vertical_id: None,
        lifecycle_state: LifecycleState::Draft,
    }
}

pub(crate) fn matches_expected(actual: &ActualOutcome, expected: &ExpectedOutcome) -> bool {
    match expected {
        ExpectedOutcome::Contains { text } => actual
            .output
            .as_str()
            .is_some_and(|s| s.contains(text.as_str())),
        ExpectedOutcome::Equals { value } => &actual.output == value,
        ExpectedOutcome::NonEmpty => match &actual.output {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Bool(_) | Value::Number(_) => true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{EvalCase, Provenance};
    use argo_types::{ArgoError, ArgoResult, CapabilitySet, Lock};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockEvalExecutor {
        outputs: Mutex<Vec<Result<Value, &'static str>>>,
    }

    impl MockEvalExecutor {
        fn new(outputs: Vec<Result<Value, &'static str>>) -> Self {
            Self {
                outputs: Mutex::new(outputs),
            }
        }
    }

    #[async_trait]
    impl EvalExecutor for MockEvalExecutor {
        async fn run_one(
            &self,
            _skill: &Skill,
            _case: &EvalCase,
        ) -> ArgoResult<ActualOutcome> {
            let mut q = self.outputs.lock().unwrap_or_else(|p| p.into_inner());
            assert!(!q.is_empty(), "MockEvalExecutor exhausted");
            match q.remove(0) {
                Ok(v) => Ok(ActualOutcome::new(v)),
                Err(why) => Err(ArgoError::Provider(why.to_string())),
            }
        }
    }

    fn dummy_skill() -> Skill {
        Skill {
            id: SkillId(uuid::Uuid::nil()),
            frontmatter: SkillFrontmatter {
                name: "test-skill".into(),
                version: "0.1.0".into(),
                description: "for tests".into(),
                capabilities: CapabilitySet::default(),
                provenance: Provenance {
                    source: "test".into(),
                    hash: "sha256-deadbeef".into(),
                },
                lock: Lock::default(),
                evals: Vec::new(),
            },
            body: String::new(),
            hash: [0u8; 32],
            learned_in: None,
            superseded_by: None,
            created_at: Utc::now(),
            vertical_id: None,
            lifecycle_state: LifecycleState::Draft,
        }
    }

    fn case(input: Value, expected: ExpectedOutcome) -> EvalCase {
        EvalCase { input, expected }
    }

    #[tokio::test]
    async fn equals_predicate_matches_json_value_exactly() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(vec![
            Ok(serde_json::json!({"status": "ok", "n": 7})),
            Ok(serde_json::json!({"status": "ok", "n": 8})),
        ]);
        let target = serde_json::json!({"status": "ok", "n": 7});
        let cases = [
            case(
                Value::Null,
                ExpectedOutcome::Equals {
                    value: target.clone(),
                },
            ),
            case(
                Value::Null,
                ExpectedOutcome::Equals {
                    value: target.clone(),
                },
            ),
        ];
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 1);
        assert_eq!(r.fail, 1);
    }

    #[tokio::test]
    async fn non_empty_predicate_distinguishes_null_and_empty() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(vec![
            Ok(Value::Null),
            Ok(Value::String(String::new())),
            Ok(Value::Array(Vec::new())),
            Ok(Value::String("non-empty".into())),
            Ok(Value::Number(0.into())),
            Ok(serde_json::json!({"k": "v"})),
        ]);
        let cases: Vec<EvalCase> = (0..6)
            .map(|_| case(Value::Null, ExpectedOutcome::NonEmpty))
            .collect();
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 3, "non-null+non-empty should pass");
        assert_eq!(r.fail, 3, "null + empty string + empty array should fail");
    }
}
