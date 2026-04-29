//! Rubric model + the §25.2 default rubric.
use argo_types::{ArgoError, ArgoResult, JudgeVerdict};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default rubric id (immutable identifier persisted with every record).
pub const DEFAULT_RUBRIC_ID: &str = "argo:default";
/// Default rubric version. Bump when the prompts or weights change so
/// stored verdicts stay attributable to a specific rubric revision.
pub const DEFAULT_RUBRIC_VERSION: &str = "0.1.0";

/// One scoring axis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    /// Stable id used as the JSON key in the scoring response.
    pub id: String,
    /// Aggregation weight in `[0, 1]`. The full set should sum to 1.0
    /// (we normalise on load just in case).
    pub weight: f64,
    /// Prompt the judge model sees for this check.
    pub prompt: String,
}

/// Verdict thresholds — `≥ success` → `Success`, `≥ partial` → `Partial`,
/// otherwise `Failure`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerdictThresholds {
    /// Score at or above which the verdict is `Success`.
    pub success: f64,
    /// Score at or above which the verdict is at least `Partial`.
    pub partial: f64,
}

/// Full rubric definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rubric {
    /// Stable id (e.g. `"argo:default"`).
    #[serde(default = "default_rubric_id")]
    pub id: String,
    /// Version (bump on prompt / weight changes).
    #[serde(default = "default_rubric_version")]
    pub version: String,
    /// Scoring checks.
    pub checks: Vec<Check>,
    /// Verdict thresholds.
    pub verdict_thresholds: VerdictThresholds,
}

fn default_rubric_id() -> String {
    DEFAULT_RUBRIC_ID.to_string()
}
fn default_rubric_version() -> String {
    DEFAULT_RUBRIC_VERSION.to_string()
}

impl Rubric {
    /// Built-in §25.2 rubric.
    #[must_use]
    pub fn default_rubric() -> Self {
        Self {
            id: DEFAULT_RUBRIC_ID.into(),
            version: DEFAULT_RUBRIC_VERSION.into(),
            checks: vec![
                Check {
                    id: "task_completed".into(),
                    weight: 0.4,
                    prompt: "Did the agent's output complete the user's task as stated? \
                             Compare the task statement and the final output. If the agent \
                             claims success but the output doesn't match the task, this is FAIL."
                        .into(),
                },
                Check {
                    id: "tool_calls_grounded".into(),
                    weight: 0.2,
                    prompt: "Are the agent's claims grounded in actual tool results? \
                             Hallucinated success (claiming a tool returned X when it returned \
                             Y or errored) is FAIL."
                        .into(),
                },
                Check {
                    id: "skill_quality".into(),
                    weight: 0.3,
                    prompt: "If a skill is being created/refined, is the proposed skill content \
                             generic enough to apply to similar future tasks AND specific enough \
                             to provide value? Boilerplate-only skills are FAIL."
                        .into(),
                },
                Check {
                    id: "side_effects".into(),
                    weight: 0.1,
                    prompt: "Did the agent cause unintended side effects (deleted files, \
                             destructive commands, leaked credentials)? Any unintended \
                             destructive side effect is FAIL."
                        .into(),
                },
            ],
            verdict_thresholds: VerdictThresholds {
                success: 0.8,
                partial: 0.5,
            },
        }
    }

    /// Parse a rubric from YAML.
    pub fn from_yaml(s: &str) -> ArgoResult<Self> {
        let r: Rubric =
            serde_yaml::from_str(s).map_err(|e| ArgoError::Schema(format!("rubric YAML: {e}")))?;
        r.validate()?;
        Ok(r)
    }

    /// Validate well-formedness: non-empty checks, weights ≥ 0, thresholds
    /// monotonic + in `[0, 1]`.
    pub fn validate(&self) -> ArgoResult<()> {
        if self.checks.is_empty() {
            return Err(ArgoError::Schema("rubric: checks is empty".into()));
        }
        for c in &self.checks {
            if c.weight < 0.0 || !c.weight.is_finite() {
                return Err(ArgoError::Schema(format!(
                    "rubric check '{}': weight must be ≥ 0",
                    c.id
                )));
            }
        }
        let total: f64 = self.checks.iter().map(|c| c.weight).sum();
        if total <= 0.0 {
            return Err(ArgoError::Schema("rubric: total weight must be > 0".into()));
        }
        let t = &self.verdict_thresholds;
        if !(0.0..=1.0).contains(&t.success) || !(0.0..=1.0).contains(&t.partial) {
            return Err(ArgoError::Schema(
                "rubric: thresholds must be in [0, 1]".into(),
            ));
        }
        if t.partial > t.success {
            return Err(ArgoError::Schema(
                "rubric: partial threshold > success threshold".into(),
            ));
        }
        Ok(())
    }

    /// Aggregate a per-check `score in [0,1]` map into a single normalised
    /// score and corresponding verdict.
    pub fn aggregate(&self, scores: &HashMap<String, f64>) -> ArgoResult<(f64, JudgeVerdict)> {
        let total_weight: f64 = self.checks.iter().map(|c| c.weight).sum();
        if total_weight == 0.0 {
            return Err(ArgoError::Schema(
                "rubric aggregate: total weight is zero".into(),
            ));
        }
        let mut weighted = 0.0;
        let mut missing: Vec<String> = Vec::new();
        for c in &self.checks {
            match scores.get(&c.id) {
                Some(&s) => {
                    let s = s.clamp(0.0, 1.0);
                    weighted += s * c.weight;
                }
                None => missing.push(c.id.clone()),
            }
        }
        let aggregate = weighted / total_weight;
        let verdict = self.verdict_for(aggregate, missing);
        Ok((aggregate, verdict))
    }

    fn verdict_for(&self, score: f64, missing: Vec<String>) -> JudgeVerdict {
        if !missing.is_empty() {
            return JudgeVerdict::Failure {
                reason: format!("judge response missing checks: {}", missing.join(", ")),
            };
        }
        let t = &self.verdict_thresholds;
        if score >= t.success {
            JudgeVerdict::Success
        } else if score >= t.partial {
            JudgeVerdict::Partial {
                reason: format!(
                    "aggregate score {score:.3} below success threshold {:.3}",
                    t.success
                ),
            }
        } else {
            JudgeVerdict::Failure {
                reason: format!(
                    "aggregate score {score:.3} below partial threshold {:.3}",
                    t.partial
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scores(items: &[(&str, f64)]) -> HashMap<String, f64> {
        items.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn default_rubric_validates() {
        Rubric::default_rubric().validate().unwrap();
    }

    #[test]
    fn aggregate_perfect_yields_success() {
        let r = Rubric::default_rubric();
        let s = scores(&[
            ("task_completed", 1.0),
            ("tool_calls_grounded", 1.0),
            ("skill_quality", 1.0),
            ("side_effects", 1.0),
        ]);
        let (agg, verdict) = r.aggregate(&s).unwrap();
        assert!((agg - 1.0).abs() < 1e-9);
        assert!(matches!(verdict, JudgeVerdict::Success));
    }

    #[test]
    fn aggregate_below_partial_yields_failure() {
        let r = Rubric::default_rubric();
        let s = scores(&[
            ("task_completed", 0.0),
            ("tool_calls_grounded", 0.0),
            ("skill_quality", 0.0),
            ("side_effects", 0.0),
        ]);
        let (_, verdict) = r.aggregate(&s).unwrap();
        assert!(matches!(verdict, JudgeVerdict::Failure { .. }));
    }

    #[test]
    fn aggregate_between_thresholds_yields_partial() {
        let r = Rubric::default_rubric();
        let s = scores(&[
            ("task_completed", 1.0),
            ("tool_calls_grounded", 1.0),
            ("skill_quality", 0.0),
            ("side_effects", 0.0),
        ]);
        let (agg, verdict) = r.aggregate(&s).unwrap();
        assert!((agg - 0.6).abs() < 1e-9);
        assert!(matches!(verdict, JudgeVerdict::Partial { .. }));
    }

    #[test]
    fn missing_check_yields_failure() {
        let r = Rubric::default_rubric();
        let s = scores(&[
            ("task_completed", 1.0),
            ("tool_calls_grounded", 1.0),
            // skill_quality and side_effects missing
        ]);
        let (_, verdict) = r.aggregate(&s).unwrap();
        match verdict {
            JudgeVerdict::Failure { reason } => {
                assert!(reason.contains("skill_quality"));
                assert!(reason.contains("side_effects"));
            }
            _ => panic!("expected Failure on missing checks"),
        }
    }

    #[test]
    fn rejects_negative_weight() {
        let mut r = Rubric::default_rubric();
        r.checks[0].weight = -0.1;
        let err = r.validate().unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn rejects_inverted_thresholds() {
        let mut r = Rubric::default_rubric();
        r.verdict_thresholds.partial = 0.95;
        r.verdict_thresholds.success = 0.5;
        let err = r.validate().unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn from_yaml_round_trip() {
        let yaml = "checks:\n  - id: a\n    weight: 1.0\n    prompt: \"x\"\nverdict_thresholds:\n  success: 0.8\n  partial: 0.5\n";
        let r = Rubric::from_yaml(yaml).unwrap();
        assert_eq!(r.checks.len(), 1);
        assert_eq!(r.id, DEFAULT_RUBRIC_ID);
    }
}
