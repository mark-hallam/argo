//! Worker v3 action ledger wire schema — state enum and record.
//!
//! Extracted from `worker_actions.rs` to keep both files under the
//! per-file 500-LOC budget. `worker_actions.rs` re-exports
//! [`ActionState`] and [`ActionRecord`] so existing callers
//! (`super::worker_actions::ActionRecord`, etc.) keep working
//! unchanged.

use argo_identity::WorkerId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ───────────────────────── enums ─────────────────────────
//
// `#[allow(dead_code)]` on the impl block: the `parse`,
// `is_terminal`, and `can_transition_to` methods are exercised by
// the unit tests in `worker_actions_tests.rs` + consumed by the
// writer + approval CLI in #729 / #730. They land in #728 as part
// of the contract so the boundary is testable now.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionState {
    Pending,
    Approved,
    Executing,
    Completed,
    Failed,
    Blocked,
    /// Operator cancelled before execution. Terminal. Reachable
    /// from `Pending` or `Approved` only — once an action is
    /// `Executing`, only `Completed` / `Failed` are valid (Stream
    /// `claude-action-cancel-flow`).
    Cancelled,
}

#[allow(dead_code)]
impl ActionState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ActionState::Pending => "pending",
            ActionState::Approved => "approved",
            ActionState::Executing => "executing",
            ActionState::Completed => "completed",
            ActionState::Failed => "failed",
            ActionState::Blocked => "blocked",
            ActionState::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "executing" => Some(Self::Executing),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "blocked" => Some(Self::Blocked),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Terminal states never transition further.
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Blocked | Self::Cancelled
        )
    }

    /// Whether `self → next` is a legal transition.
    pub(crate) fn can_transition_to(&self, next: ActionState) -> bool {
        use ActionState::*;
        matches!(
            (*self, next),
            (Pending, Approved)
                | (Pending, Blocked)
                | (Pending, Cancelled)
                | (Pending, Executing) // autonomous-safe path
                | (Approved, Executing)
                | (Approved, Completed)
                | (Approved, Cancelled)
                | (Executing, Completed)
                | (Executing, Failed)
        )
    }
}

// ──────────────────────── record ─────────────────────────

/// Wire shape persisted into a memory row's body. Each state
/// transition appends a NEW row carrying the full snapshot at
/// that state; the action's "current state" is determined by
/// taking the latest row matching the `action_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ActionRecord {
    pub schema_key: String,
    pub action_id: String,
    pub job_id: String,
    #[serde(default = "WorkerId::nil")]
    pub worker_id: WorkerId,
    #[serde(default)]
    pub item_id: Option<String>,
    pub action_type: String,
    /// `low` / `medium` / `high` — stamped from the bound
    /// vertical's `[actions.<name>].risk` declaration at
    /// emission time (#729). The model can't override this.
    /// Default is `low` for compatibility with action rows
    /// written before this field existed.
    #[serde(default = "default_risk")]
    pub risk_level: String,
    /// Stamped from the bound vertical's
    /// `[actions.<name>].requires_approval` at emission time.
    /// The model can't override this. Default `false` for
    /// pre-#729 rows.
    #[serde(default)]
    pub requires_approval: bool,
    pub state: String,
    pub payload: Value,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub approved_at: Option<String>,
    #[serde(default)]
    pub executed_at: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    pub created_at: String,
}

impl ActionRecord {
    pub(crate) const SCHEMA_KEY: &'static str = "worker_action";
}

fn default_risk() -> String { "low".to_string() }
