//! Skill eval-set runner — pluggable execution + pass-rate gate.
//!
//! When a `SKILL.md` declares an `evals: [...]` array in its
//! frontmatter, the runner walks the cases and asks a caller-supplied
//! [`EvalExecutor`] to produce an [`ActualOutcome`] for each. The
//! runner matches outcomes against [`crate::skill::ExpectedOutcome`]
//! predicates and aggregates a pass-rate. Callers that wire the
//! runner into a promotion gate use the rate to decide whether the
//! candidate is fit to graduate from `pending_review`.
//!
//! Design notes:
//! - The trait is deliberately abstract: this crate ships no real
//!   executor (no LLM call, no HTTP, no runtime). Production
//!   integration lives in argo-cli / argo-runtime callers.
//! - `MockEvalExecutor` (cfg(test)) is the only sample implementation
//!   here; it lets the unit tests exercise every branch without
//!   pulling in a heavier runtime.
//! - Pass-rate threshold is hard-coded to [`DEFAULT_EVAL_PASS_RATE_THRESHOLD`]
//!   for now; configurability lands in a follow-up brief.
//! - The propose-for-review gate adapter and outcome-matching predicate
//!   live in the [`crate::eval_runner_gate`] sibling; this file owns
//!   only the trait + report types + the `run_evals` driver.

use crate::eval_runner_gate::matches_expected;
use crate::skill::{EvalCase, Skill};
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) use crate::eval_runner_gate::{run_propose_eval_gate, EvalGateOutcome};

/// Default pass-rate floor. Below this, callers should refuse to
/// promote a candidate (`<` is strict — exactly-equal still fails).
pub const DEFAULT_EVAL_PASS_RATE_THRESHOLD: f32 = 0.8;

/// What an [`EvalExecutor`] produces for one [`EvalCase`].
#[derive(Debug, Clone)]
pub struct ActualOutcome {
    /// Free-form output the executor returns. The runner's
    /// predicate matchers walk this against [`crate::skill::ExpectedOutcome`].
    pub output: Value,
}

impl ActualOutcome {
    /// Convenience constructor wrapping any JSON-serializable value.
    pub fn new(output: Value) -> Self {
        Self { output }
    }
}

/// Pluggable executor the runner asks to evaluate one case at a time.
///
/// Real implementations (LLM-driven, HTTP-driven, etc.) live in the
/// caller crate — argo-skill ships only the trait + the runner +
/// a test-only mock.
#[async_trait]
pub trait EvalExecutor: Send + Sync {
    /// Run one case against the skill. Implementations should treat
    /// `Err` as a hard executor failure (not a failed expectation —
    /// the runner counts both as "fail" but separates the wire from
    /// signal in logs).
    async fn run_one(&self, skill: &Skill, case: &EvalCase) -> ArgoResult<ActualOutcome>;
}

/// Aggregated outcome of [`run_evals`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalReport {
    /// Cases whose `ActualOutcome` matched the case's `ExpectedOutcome`.
    pub pass: u32,
    /// Cases that produced a wrong outcome OR errored at the executor.
    pub fail: u32,
    /// `pass / (pass + fail)` clamped to `[0.0, 1.0]`. Empty input
    /// returns `1.0` so a skill with no evals is treated as fully
    /// passing — callers gate on a separate "non-empty evals required"
    /// signal if that matters.
    pub rate: f32,
}

impl EvalReport {
    /// Convenience — true when the report's rate is at or above
    /// the floor. Note `< floor` is rejected, equal is accepted.
    #[must_use]
    pub fn passes_threshold(&self, floor: f32) -> bool {
        self.rate >= floor
    }
}

/// Walk every eval case, invoke the executor, and return aggregate
/// pass-rate. Empty `cases` short-circuits to `EvalReport { 0, 0, 1.0 }`
/// — no work to do.
///
/// Executor errors do NOT abort the run; they count as fails for
/// that case and the loop continues. This lets a flapping executor
/// surface a clear rate (e.g. 50% executor errors → rate < threshold)
/// rather than masking the problem behind a transport panic.
pub async fn run_evals(
    skill: &Skill,
    executor: &dyn EvalExecutor,
    cases: &[EvalCase],
) -> ArgoResult<EvalReport> {
    if cases.is_empty() {
        return Ok(EvalReport {
            pass: 0,
            fail: 0,
            rate: 1.0,
        });
    }
    let mut pass = 0u32;
    let mut fail = 0u32;
    for case in cases {
        match executor.run_one(skill, case).await {
            Ok(actual) if matches_expected(&actual, &case.expected) => pass += 1,
            Ok(_) | Err(_) => fail += 1,
        }
    }
    let total = pass + fail;
    let rate = if total == 0 {
        1.0
    } else {
        f32::from(u16::try_from(pass).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(total).unwrap_or(u16::MAX))
    };
    Ok(EvalReport { pass, fail, rate })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{ExpectedOutcome, LifecycleState, Provenance, SkillFrontmatter};
    use argo_types::{ArgoError, CapabilitySet, Lock, SkillId};
    use chrono::Utc;
    use std::sync::Mutex;

    /// Test-only executor: maps each case input to a stub output.
    /// `outputs[i]` is consumed in order; an `Err` slot triggers
    /// the executor-error branch.
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
    async fn empty_cases_short_circuits_to_perfect_rate() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(Vec::new());
        let r = run_evals(&skill, &exec, &[]).await.unwrap();
        assert_eq!(r.pass, 0);
        assert_eq!(r.fail, 0);
        assert!((r.rate - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn all_pass_yields_rate_1_0() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(vec![
            Ok(Value::String("hello world".into())),
            Ok(Value::String("greetings world!".into())),
        ]);
        let cases = [
            case(
                Value::Null,
                ExpectedOutcome::Contains {
                    text: "world".into(),
                },
            ),
            case(
                Value::Null,
                ExpectedOutcome::Contains {
                    text: "world".into(),
                },
            ),
        ];
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 2);
        assert_eq!(r.fail, 0);
        assert!((r.rate - 1.0).abs() < f32::EPSILON);
        assert!(r.passes_threshold(DEFAULT_EVAL_PASS_RATE_THRESHOLD));
    }

    #[tokio::test]
    async fn all_fail_yields_rate_0_0() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(vec![
            Ok(Value::String("nope".into())),
            Ok(Value::String("nada".into())),
        ]);
        let cases = [
            case(
                Value::Null,
                ExpectedOutcome::Contains {
                    text: "yes".into(),
                },
            ),
            case(
                Value::Null,
                ExpectedOutcome::Contains {
                    text: "yes".into(),
                },
            ),
        ];
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 0);
        assert_eq!(r.fail, 2);
        assert!(r.rate.abs() < f32::EPSILON);
        assert!(!r.passes_threshold(DEFAULT_EVAL_PASS_RATE_THRESHOLD));
    }

    #[tokio::test]
    async fn mixed_pass_fail_yields_correct_rate() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(vec![
            Ok(Value::String("yes".into())),
            Ok(Value::String("no".into())),
            Ok(Value::String("yes".into())),
            Ok(Value::String("no".into())),
        ]);
        let cases: Vec<EvalCase> = (0..4)
            .map(|_| {
                case(
                    Value::Null,
                    ExpectedOutcome::Contains {
                        text: "yes".into(),
                    },
                )
            })
            .collect();
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 2);
        assert_eq!(r.fail, 2);
        assert!((r.rate - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn threshold_edge_4_of_5_passes_default_floor() {
        let skill = dummy_skill();
        // 4 pass + 1 fail = 0.8 rate, equal to default floor → passes.
        let exec = MockEvalExecutor::new(vec![
            Ok(Value::String("ok".into())),
            Ok(Value::String("ok".into())),
            Ok(Value::String("ok".into())),
            Ok(Value::String("ok".into())),
            Ok(Value::String("nope".into())),
        ]);
        let cases: Vec<EvalCase> = (0..5)
            .map(|_| {
                case(
                    Value::Null,
                    ExpectedOutcome::Contains { text: "ok".into() },
                )
            })
            .collect();
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 4);
        assert_eq!(r.fail, 1);
        assert!((r.rate - 0.8).abs() < f32::EPSILON);
        assert!(r.passes_threshold(DEFAULT_EVAL_PASS_RATE_THRESHOLD));
    }

    #[tokio::test]
    async fn executor_error_counts_as_fail_not_abort() {
        let skill = dummy_skill();
        let exec = MockEvalExecutor::new(vec![
            Ok(Value::String("ok".into())),
            Err("simulated transport error"),
            Ok(Value::String("ok".into())),
        ]);
        let cases: Vec<EvalCase> = (0..3)
            .map(|_| {
                case(
                    Value::Null,
                    ExpectedOutcome::Contains { text: "ok".into() },
                )
            })
            .collect();
        let r = run_evals(&skill, &exec, &cases).await.unwrap();
        assert_eq!(r.pass, 2);
        assert_eq!(r.fail, 1);
        assert!((r.rate - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn passes_threshold_inclusive_at_floor() {
        let r = EvalReport {
            pass: 4,
            fail: 1,
            rate: 0.8,
        };
        assert!(r.passes_threshold(0.8));
        assert!(!r.passes_threshold(0.81));
    }
}
