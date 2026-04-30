//! Eval runner — feeds tasks into an `EvalSubject` and aggregates
//! per-task scores into a final dataset summary.
use crate::task::EvalTask;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// What's under test. The runner gives the subject the task input;
/// the subject returns a string the metric scores. Production
/// callers bind this to an `Agent` (with a fixed model + system
/// prompt) or directly to a routing classifier.
#[async_trait]
pub trait EvalSubject: Send + Sync {
    /// Run one task, return the output to score.
    async fn run(&self, task: &EvalTask) -> ArgoResult<String>;
}

/// Per-task outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalRecord {
    /// Task name.
    pub task: String,
    /// Score in `[0.0, 1.0]`.
    pub score: f64,
    /// `true` when `score >= 0.5` (binary view used by `forge bench`).
    pub passed: bool,
    /// Output the subject produced.
    pub output: String,
}

/// Aggregate summary of an eval run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalSummary {
    /// Number of tasks attempted.
    pub n: usize,
    /// Mean score across attempted tasks.
    pub mean_score: f64,
    /// Pass rate (count where `passed == true` / `n`).
    pub pass_rate: f64,
    /// Number of subject errors (failed to produce output at all).
    pub errors: usize,
}

/// Run `tasks` through `subject`, returning the per-task records
/// and the aggregate summary. Errors from the subject become a
/// zero-score record with the `errors` counter incremented.
pub async fn run_eval(
    subject: &dyn EvalSubject,
    tasks: &[EvalTask],
) -> (Vec<EvalRecord>, EvalSummary) {
    let mut records = Vec::with_capacity(tasks.len());
    let mut errors = 0_usize;
    for t in tasks {
        match subject.run(t).await {
            Ok(out) => {
                let score = t.score(&out);
                records.push(EvalRecord {
                    task: t.name.clone(),
                    score,
                    passed: score >= 0.5,
                    output: out,
                });
            }
            Err(_) => {
                errors += 1;
                records.push(EvalRecord {
                    task: t.name.clone(),
                    score: 0.0,
                    passed: false,
                    output: String::new(),
                });
            }
        }
    }
    let n = records.len();
    let mean_score = if n == 0 {
        0.0
    } else {
        records.iter().map(|r| r.score).sum::<f64>() / n as f64
    };
    let passes = records.iter().filter(|r| r.passed).count();
    let pass_rate = if n == 0 {
        0.0
    } else {
        passes as f64 / n as f64
    };
    let summary = EvalSummary {
        n,
        mean_score,
        pass_rate,
        errors,
    };
    (records, summary)
}
