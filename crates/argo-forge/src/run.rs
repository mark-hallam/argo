//! Forge run model — the persisted shape of one optimization run
//! (§7 `forge_run` table).
use argo_types::ForgeRunId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a Forge run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeStatus {
    /// Loaded examples, pre-baseline.
    Pending,
    /// GEPA loop in flight.
    Running,
    /// Successful completion (candidate present).
    Completed,
    /// Budget cap hit (§26.5 — "hitting the budget cleanly aborts and
    /// persists partial state").
    BudgetExceeded,
    /// User aborted (`argo forge runs abort <id>`).
    Aborted,
    /// Worker failure.
    Failed,
}

impl ForgeStatus {
    /// Wire label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::BudgetExceeded => "budget_exceeded",
            Self::Aborted => "aborted",
            Self::Failed => "failed",
        }
    }

    /// Parse from a `status` column value.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "budget_exceeded" => Self::BudgetExceeded,
            "aborted" => Self::Aborted,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

/// Per-call options for a Forge optimization run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeOptions {
    /// Skill being optimized.
    pub skill_name: String,
    /// Version of the skill at run start (the base). The run produces
    /// a `candidate_version` if successful.
    pub base_version: String,
    /// Path to a JSONL file of examples (`{input, expected_output}`
    /// per line).
    pub examples_path: std::path::PathBuf,
    /// Number of GEPA iterations (default 12 per spec).
    #[serde(default = "default_iterations")]
    pub iterations: u32,
    /// Hard USD budget for the run (default $5 per spec).
    #[serde(default = "default_budget_usd_micros")]
    pub budget_usd_micros: u64,
    /// Score above which an unverified candidate auto-promotes
    /// (still needs a passing judge verdict per §26.5).
    #[serde(default = "default_auto_promote")]
    pub auto_promote_threshold: f64,
}

fn default_iterations() -> u32 {
    12
}
fn default_budget_usd_micros() -> u64 {
    5_000_000
}
fn default_auto_promote() -> f64 {
    0.85
}

impl ForgeOptions {
    /// Convenience constructor with spec defaults.
    #[must_use]
    pub fn new(skill_name: impl Into<String>, base_version: impl Into<String>) -> Self {
        Self {
            skill_name: skill_name.into(),
            base_version: base_version.into(),
            examples_path: std::path::PathBuf::new(),
            iterations: default_iterations(),
            budget_usd_micros: default_budget_usd_micros(),
            auto_promote_threshold: default_auto_promote(),
        }
    }
}

/// One row of the `forge_run` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRun {
    /// Stable id.
    pub id: ForgeRunId,
    /// Skill name.
    pub skill_name: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// When the run ended (None while running).
    pub ended_at: Option<DateTime<Utc>>,
    /// Skill version at start.
    pub base_version: String,
    /// Candidate version produced (if successful).
    pub candidate_version: Option<String>,
    /// Number of examples loaded.
    pub examples_count: u32,
    /// GEPA iterations completed.
    pub iterations: u32,
    /// Cumulative LLM spend (USD-micros).
    pub spend_usd_micros: u64,
    /// Final score (0..=1) on the held-out set.
    pub final_score: Option<f64>,
    /// Run status.
    pub status: ForgeStatus,
}

impl ForgeRun {
    /// Build a fresh `ForgeRun` row in `Pending` status.
    #[must_use]
    pub fn pending(opts: &ForgeOptions) -> Self {
        Self {
            id: ForgeRunId::new(),
            skill_name: opts.skill_name.clone(),
            started_at: Utc::now(),
            ended_at: None,
            base_version: opts.base_version.clone(),
            candidate_version: None,
            examples_count: 0,
            iterations: 0,
            spend_usd_micros: 0,
            final_score: None,
            status: ForgeStatus::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_round_trip() {
        for s in [
            ForgeStatus::Pending,
            ForgeStatus::Running,
            ForgeStatus::Completed,
            ForgeStatus::BudgetExceeded,
            ForgeStatus::Aborted,
            ForgeStatus::Failed,
        ] {
            assert_eq!(ForgeStatus::from_label(s.label()), Some(s));
        }
    }

    #[test]
    fn unknown_status_label_yields_none() {
        assert!(ForgeStatus::from_label("unknown").is_none());
    }

    #[test]
    fn options_defaults_match_spec_caps() {
        let o = ForgeOptions::new("alpha", "0.1");
        assert_eq!(o.iterations, 12); // §26.2 default
        assert_eq!(o.budget_usd_micros, 5_000_000); // $5 cap §26.5
        assert!((o.auto_promote_threshold - 0.85).abs() < 1e-9);
    }

    #[test]
    fn run_starts_in_pending() {
        let o = ForgeOptions::new("alpha", "0.1");
        let r = ForgeRun::pending(&o);
        assert_eq!(r.status, ForgeStatus::Pending);
        assert_eq!(r.skill_name, "alpha");
        assert_eq!(r.iterations, 0);
        assert_eq!(r.spend_usd_micros, 0);
        assert!(r.candidate_version.is_none());
        assert!(r.ended_at.is_none());
    }
}
