//! Worker v3 execution-mode action ledger (#728).
//!
//! Actions are persisted as `worker_action` memory rows in the
//! originating job's vertical namespace. Each row captures a
//! complete snapshot of the action at the moment its state
//! changed; the "current" state of an action is the latest row
//! whose `action_id` matches.
//!
//! ## Safety boundary
//!
//! Per the SKYCOT mission: the worker may autonomously plan,
//! draft, queue, and verify evidence; it must require human
//! approval before any action that posts publicly, messages a
//! buyer, spends money, changes a public price, or marks an
//! item sold. `ActionType::requires_approval()` encodes that
//! boundary in the type system so #730 / #731 can rely on it
//! rather than re-deriving the rule.
//!
//! #728 scope: data model + read-side CLI (`list` / `show`).
//! Worker emits actions in #729; approval CLI in #730;
//! approval-gated action types + execution in #731.

use crate::cli::cli_worker_cmds::{WorkerActionsCmd, WorkerCmd, WorkerItemsCmd, WorkerPolicyCmd};
use argo_identity::WorkerId;
use argo_types::ArgoResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-export short() so the existing tests in `worker_actions_tests.rs`
// can keep calling it via `super::short` after the
// `claude-worker-actions-split` refactor.
#[cfg(test)]
pub(crate) use super::worker_actions_query::short;

// ───────────────────────── enums ─────────────────────────
//
// `#[allow(dead_code)]` on the impl blocks: the `parse`,
// `requires_approval`, `is_terminal`, and `can_transition_to`
// methods are exercised by the unit tests below + consumed by
// the writer + approval CLI in #729 / #730. They land in #728
// as part of the contract so the boundary is testable now.

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

// `ActionType` enum lives in `worker_actions_type.rs` — kept
// distinct so `worker_actions.rs` stays under the per-file
// 500-LOC budget. The SKYCOT safety boundary lock test in
// `worker_actions_tests.rs` continues to import it via the
// re-export below.
#[allow(unused_imports)]
pub(crate) use super::worker_actions_type::ActionType;

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

// ───────────────────────── CLI ───────────────────────────

pub async fn run(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Create { vertical, persona, charter, builder, voice_preamble, duties, json } => super::worker_create::run(&vertical, &persona, &charter, builder.as_deref(), voice_preamble.as_deref(), &duties, json).await,
        WorkerCmd::Show { worker_id, tenant, json } => super::worker_list::show(&tenant, &worker_id, json).await,
        WorkerCmd::List { tenant, persona, json } => super::worker_list::list(&tenant, persona.as_deref(), json).await,
        WorkerCmd::Policy(policy_cmd) => match policy_cmd {
            WorkerPolicyCmd::Show { worker_id, tenant, json } => super::worker_list::policy_show(&tenant, &worker_id, json).await,
        },
        WorkerCmd::Update { worker_id, auto_approval_json, json } => {
            super::worker_update::update_auto_approval(&worker_id, &auto_approval_json, json).await
        }
        WorkerCmd::Doctor { vertical, json } => {
            super::worker_doctor::run(vertical.as_deref(), json).await
        }
        WorkerCmd::Items(items_cmd) => match items_cmd {
            WorkerItemsCmd::List { vertical, state, limit, json } => {
                super::worker_actions_items::list(vertical.as_deref(), &state, limit, json).await
            }
            WorkerItemsCmd::Show { item_id, vertical, json } => {
                super::worker_actions_items::show(&item_id, vertical.as_deref(), json).await
            }
        },
        WorkerCmd::Actions(actions_cmd) => match actions_cmd {
            WorkerActionsCmd::List {
                state,
                job_id,
                vertical,
                item_id,
                action_type,
                platform,
                limit,
                json,
            } => {
                super::worker_actions_query::list(
                    &state,
                    job_id.as_deref(),
                    vertical.as_deref(),
                    item_id.as_deref(),
                    action_type.as_deref(),
                    platform.as_deref(),
                    limit,
                    json,
                )
                .await
            }
            WorkerActionsCmd::Show { action_id, json } => {
                super::worker_actions_query::show(&action_id, json).await
            }
            WorkerActionsCmd::Approve {
                action_id,
                approved_by,
                note,
                json,
            } => {
                super::worker_actions_dispatch::approve(
                    &action_id,
                    approved_by.as_deref(),
                    note.as_deref(),
                    json,
                )
                .await
            }
            WorkerActionsCmd::ApproveBatch {
                action_ids,
                approved_by,
                note,
                json,
            } => {
                super::worker_actions_dispatch::approve_batch(
                    &action_ids,
                    approved_by.as_deref(),
                    note.as_deref(),
                    json,
                )
                .await
            }
            WorkerActionsCmd::Execute { action_id, json } => {
                super::worker_actions_dispatch::execute(&action_id, json).await
            }
            WorkerActionsCmd::Complete {
                action_id,
                note,
                result_json,
                json,
            } => {
                super::worker_actions_dispatch::complete(
                    &action_id,
                    note.as_deref(),
                    result_json.as_deref(),
                    json,
                )
                .await
            }
            WorkerActionsCmd::Fail {
                action_id,
                reason,
                json,
            } => super::worker_actions_dispatch::fail(&action_id, &reason, json).await,
            WorkerActionsCmd::Cancel {
                action_id,
                cancelled_by,
                note,
                json,
            } => {
                super::worker_actions_dispatch::cancel(
                    &action_id,
                    cancelled_by.as_deref(),
                    note.as_deref(),
                    json,
                )
                .await
            }
            WorkerActionsCmd::Status { action_id, json } => {
                super::worker_actions_dispatch::status(&action_id, json).await
            }
            WorkerActionsCmd::Render { vertical } => {
                super::worker_actions_dispatch::render(vertical.as_deref()).await
            }
            WorkerActionsCmd::Retry {
                action_id,
                force,
                json,
            } => super::worker_actions_dispatch::retry(&action_id, force, json).await,
        },
    }
}

#[cfg(test)]
#[path = "worker_actions_tests.rs"]
mod tests;
