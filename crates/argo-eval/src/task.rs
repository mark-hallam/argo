//! Eval task — one input/expected pair plus the scoring metric.
//!
//! Tasks live on disk under `eval/tasks/<category>/*.jsonl`, one
//! task per line. The shape mirrors what `argo forge optimize` and
//! `argo forge bench` consume.
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Scoring metric. `Score` returns a continuous `[0.0, 1.0]`;
/// `Pass` is a binary 0/1 derived from string equality (used for
/// the routing-correctness suite where a tier label must match).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvalMetric {
    /// Score is `1.0` when output equals expected, else `0.0`.
    ExactMatch,
    /// Score is `1.0` when output contains every entry in `must_contain`.
    SubstringContains {
        /// Required substrings (all must appear, AND semantics).
        must_contain: Vec<String>,
    },
    /// Score = `Jaccard` over whitespace tokens.
    JaccardTokens,
}

/// A single eval example.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    /// Task name (e.g. `"core/file-read-001"`).
    pub name: String,
    /// Input fed to the agent / skill / router.
    pub input: String,
    /// Expected output (what the metric scores against).
    pub expected: String,
    /// Scoring metric.
    pub metric: EvalMetric,
}

impl EvalTask {
    /// Score `actual` against `self.expected` per `self.metric`.
    /// Returns a value in `[0.0, 1.0]`.
    #[must_use]
    pub fn score(&self, actual: &str) -> f64 {
        match &self.metric {
            EvalMetric::ExactMatch => f64::from(u8::from(actual == self.expected)),
            EvalMetric::SubstringContains { must_contain } => {
                if must_contain.iter().all(|s| actual.contains(s.as_str())) {
                    1.0
                } else {
                    0.0
                }
            }
            EvalMetric::JaccardTokens => jaccard(actual, &self.expected),
        }
    }
}

fn jaccard(a: &str, b: &str) -> f64 {
    let a_set: std::collections::BTreeSet<&str> = a.split_whitespace().collect();
    let b_set: std::collections::BTreeSet<&str> = b.split_whitespace().collect();
    if a_set.is_empty() && b_set.is_empty() {
        return 1.0;
    }
    let inter = a_set.intersection(&b_set).count();
    let union = a_set.union(&b_set).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Load a JSONL file into a `Vec<EvalTask>`.
///
/// One task per line; blank lines are skipped. Bad lines cause a
/// `Provider` error rather than silent skipping — eval datasets are
/// the source of truth for "did the change improve things?", so a
/// half-loaded set is dangerous.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if the file can't be read or any
/// line fails JSON parsing.
pub async fn load_jsonl(path: &Path) -> ArgoResult<Vec<EvalTask>> {
    let body = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let mut tasks = Vec::new();
    for (lineno, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let task: EvalTask = serde_json::from_str(trimmed).map_err(|e| {
            ArgoError::Provider(format!(
                "parse line {} of {}: {e}",
                lineno + 1,
                path.display()
            ))
        })?;
        tasks.push(task);
    }
    Ok(tasks)
}
