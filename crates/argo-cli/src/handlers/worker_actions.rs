//! Worker v3 action ledger schema, read-side CLI, and dispatch.

use crate::cli::cli_worker_channels_cmds::WorkerChannelsCmd;
use crate::cli::cli_worker_cmds::{
    WorkerActionsCmd, WorkerArchiveCmd, WorkerCharterCmd, WorkerCmd, WorkerItemsCmd,
    WorkerKpiCmd, WorkerPolicyCmd,
};
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
        WorkerCmd::Create { vertical, persona, charter, builder, voice_preamble, duties, fs_root, json } => super::worker_create::run(&vertical, &persona, &charter, builder.as_deref(), voice_preamble.as_deref(), &duties, fs_root.as_deref(), json).await,
        WorkerCmd::Onboard { persona_spec, id, json } => {
            super::worker_onboard::run(&persona_spec, id.as_deref(), json).await
        }
        WorkerCmd::Show { worker_id, tenant, json } => super::worker_show::run(&tenant, &worker_id, json).await,
        WorkerCmd::List { tenant, persona, json } => super::worker_list::list(&tenant, persona.as_deref(), json).await,
        WorkerCmd::Costs(costs_cmd) => super::worker_costs::run(costs_cmd).await,
        WorkerCmd::Portrait { worker_id, tenant, json } => {
            super::worker_portrait::run(&worker_id, &tenant, json).await
        }
        WorkerCmd::Policy(policy_cmd) => match policy_cmd {
            WorkerPolicyCmd::Show { worker_id, tenant, json } => super::worker_list::policy_show(&tenant, &worker_id, json).await,
        },
        WorkerCmd::Update { worker_id, auto_approval_json, json } => {
            super::worker_update::update_auto_approval(&worker_id, &auto_approval_json, json).await
        }
        WorkerCmd::Resume { worker_id, tenant, reason, json } => {
            super::worker_resume::run(&worker_id, &tenant, reason.as_deref(), json).await
        }
        WorkerCmd::Doctor { vertical, json } => {
            super::worker_doctor::run(vertical.as_deref(), json).await
        }
        WorkerCmd::Anomalies {
            worker_id,
            since_minutes,
            limit,
            json,
        } => {
            super::worker_anomalies::run(&worker_id, since_minutes, limit, json).await
        }
        WorkerCmd::Charter(charter_cmd) => match charter_cmd {
            WorkerCharterCmd::Set { worker_id, tenant, file, kpis, json } => {
                super::worker_charter::set(&tenant, &worker_id, &file, kpis.as_deref(), json).await
            }
            WorkerCharterCmd::Get { worker_id, tenant, md, json } => {
                super::worker_charter::get(&tenant, &worker_id, md, json).await
            }
            WorkerCharterCmd::Clear { worker_id, tenant, json } => {
                super::worker_charter::clear(&tenant, &worker_id, json).await
            }
        },
        WorkerCmd::Archive(archive_cmd) => match archive_cmd {
            WorkerArchiveCmd::Add { worker_id, tenant, reason, principal, json } => {
                super::worker_archive::archive(
                    &tenant,
                    &worker_id,
                    reason.as_deref(),
                    principal.as_deref(),
                    json,
                )
                .await
            }
            WorkerArchiveCmd::List { tenant, json } => {
                super::worker_archive::list(&tenant, json).await
            }
        },
        WorkerCmd::Rehydrate { worker_id, tenant, json } => {
            super::worker_archive::rehydrate(&tenant, &worker_id, json).await
        }
        WorkerCmd::Channels(channels_cmd) => match channels_cmd {
            WorkerChannelsCmd::Add { worker_id, spec, tenant, json } => {
                super::worker_channels::add(&tenant, &worker_id, &spec, json).await
            }
            WorkerChannelsCmd::Remove { worker_id, channel_id, tenant, json } => {
                super::worker_channels::remove(&tenant, &worker_id, &channel_id, json).await
            }
            WorkerChannelsCmd::List { worker_id, tenant, json } => {
                super::worker_channels::list(&tenant, &worker_id, json).await
            }
        },
        WorkerCmd::Budget(budget_cmd) => super::worker_budget::run(budget_cmd).await,
        WorkerCmd::Kpi(kpi_cmd) => match kpi_cmd {
            WorkerKpiCmd::Record { worker_id, kpi, value, tenant, note, json } => {
                super::worker_kpi::record(&worker_id, &kpi, value, &tenant, note.as_deref(), json).await
            }
            WorkerKpiCmd::List { worker_id, tenant, kpi, since, json } => {
                super::worker_kpi::list(&worker_id, &tenant, kpi.as_deref(), since.as_deref(), json).await
            }
            WorkerKpiCmd::Series { worker_id, kpi, tenant, bucket, json } => {
                super::worker_kpi::series(&worker_id, &kpi, &tenant, &bucket, json).await
            }
        },
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
                deny,
                confirm,
                dry_run,
                json,
            } => {
                super::worker_actions_dispatch::approve_batch(
                    &action_ids,
                    approved_by.as_deref(),
                    note.as_deref(),
                    deny,
                    confirm,
                    dry_run,
                    json,
                )
                .await
            }
            WorkerActionsCmd::Execute {
                action_id,
                dry_run,
                json,
            } => {
                super::worker_actions_dispatch::execute(&action_id, dry_run, json).await
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
