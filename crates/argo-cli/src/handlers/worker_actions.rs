//! Worker v3 execution-mode action ledger (#728).
//!
//! Actions are persisted as `worker_action` memory rows in the
//! originating job's vertical namespace. Each row captures a
//! complete snapshot of the action at the moment its state
//! changed; the "current" state of an action is the latest row
//! whose `action_id` matches.
//!
//! ## State machine
//!
//! ```text
//!   pending ──→ approved ──→ executing ──→ completed
//!     │           │              │           ▲
//!     │           └──────────────┴───────────┘   (synchronous shortcut)
//!     │                                ↓
//!     │                              failed
//!     ↓
//!   blocked  (terminal)
//! ```
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

use crate::cli::cli_worker_cmds::{WorkerActionsCmd, WorkerCmd, WorkerItemsCmd};
use crate::handlers::vertical::resolve_verticals_dir;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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
            _ => None,
        }
    }

    /// Terminal states never transition further. `failed` is
    /// terminal for THIS action — operators retry by creating a
    /// new action that references the failed one in `evidence`.
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Blocked)
    }

    /// Whether `self → next` is a legal transition. Pure state-
    /// state predicate — does NOT enforce approval gating.
    /// Approval gating is enforced upstream by the `execute`
    /// transition handler against the row's `requires_approval`
    /// field (the manifest classification stamped at emission
    /// time). Keeping the two concerns separate means the state
    /// machine stays a pure invariant of the action lifecycle.
    pub(crate) fn can_transition_to(&self, next: ActionState) -> bool {
        use ActionState::*;
        matches!(
            (*self, next),
            (Pending, Approved)
                | (Pending, Blocked)
                | (Pending, Executing) // autonomous-safe path
                | (Approved, Executing)
                | (Approved, Completed)
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

fn default_risk() -> String {
    "low".to_string()
}

// ───────────────────────── CLI ───────────────────────────

pub async fn run(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Doctor { vertical, json } => {
            super::worker_doctor::run(vertical.as_deref(), json).await
        }
        WorkerCmd::Items(items_cmd) => match items_cmd {
            WorkerItemsCmd::List { vertical, state, limit, json } => {
                super::worker_actions_items::list(vertical.as_deref(), &state, limit, json).await
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
                list(
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
            WorkerActionsCmd::Show { action_id, json } => show(&action_id, json).await,
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

// ─────────────────── implementation ──────────────────────

#[allow(clippy::too_many_arguments)]
async fn list(
    state_filter: &str,
    job_id: Option<&str>,
    vertical: Option<&str>,
    item_id_filter: Option<&str>,
    action_type_filter: Option<&str>,
    platform_filter: Option<&str>,
    limit: usize,
    json_out: bool,
) -> ArgoResult<()> {
    let actions = collect_latest_actions(vertical).await?;
    let mut rows = super::worker_actions_list::apply_list_filters(
        actions,
        state_filter,
        job_id,
        item_id_filter,
        action_type_filter,
        platform_filter,
    );
    // Sort newest-first by created_at — operators want recent
    // first; ties broken by action_id for determinism.
    rows.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    rows.truncate(limit);

    if json_out {
        for a in &rows {
            println!(
                "{}",
                serde_json::to_string(&a).unwrap_or_else(|_| "{}".into())
            );
        }
    } else if rows.is_empty() {
        println!("(no actions)");
    } else {
        println!("action_id                  state        type                   platform       created_at");
        for a in &rows {
            println!(
                "{:<26} {:<12} {:<22} {:<14} {}",
                short(&a.action_id),
                a.state,
                a.action_type,
                a.platform.as_deref().unwrap_or("-"),
                a.created_at,
            );
        }
    }
    Ok(())
}

async fn show(action_id: &str, json_out: bool) -> ArgoResult<()> {
    let actions = collect_latest_actions(None).await?;
    let Some(found) = actions.into_iter().find(|a| a.action_id == action_id) else {
        return Err(ArgoError::Provider(format!(
            "no action with action_id '{action_id}'"
        )));
    };
    // #755: surface retry-chain counters alongside the rest.
    let attempt_number = super::worker_actions_dispatch::count_retry_chain(&found);
    let max_attempts = lookup_max_attempts(&found.action_type).await.unwrap_or(1);
    if json_out {
        // Augment the serialized record with the derived
        // retry counters so consumers don't have to re-walk
        // the evidence chain or reload the manifest.
        let mut v = serde_json::to_value(&found).unwrap_or(serde_json::json!({}));
        if let Some(map) = v.as_object_mut() {
            map.insert("retry_attempt".into(), serde_json::json!(attempt_number));
            map.insert("retry_max_attempts".into(), serde_json::json!(max_attempts));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("action_id     {}", found.action_id);
        println!("job_id        {}", found.job_id);
        if let Some(it) = &found.item_id {
            println!("item_id       {it}");
        }
        println!("action_type   {}", found.action_type);
        println!("state         {}", found.state);
        if let Some(p) = &found.platform {
            println!("platform      {p}");
        }
        println!("created_at    {}", found.created_at);
        println!("attempt       {attempt_number}/{max_attempts}");
        if let Some(by) = &found.approved_by {
            println!("approved_by   {by}");
        }
        if let Some(at) = &found.approved_at {
            println!("approved_at   {at}");
        }
        if let Some(at) = &found.executed_at {
            println!("executed_at   {at}");
        }
        if let Some(reason) = &found.blocked_reason {
            println!("blocked       {reason}");
        }
        if !found.evidence.is_empty() {
            println!("evidence:");
            for e in &found.evidence {
                println!("  - {e}");
            }
        }
        println!("payload:");
        println!(
            "{}",
            serde_json::to_string_pretty(&found.payload).unwrap_or_else(|_| "{}".into())
        );
        if let Some(r) = &found.result {
            println!("result:");
            println!(
                "{}",
                serde_json::to_string_pretty(r).unwrap_or_else(|_| "{}".into())
            );
        }
    }
    Ok(())
}

/// Walk every loaded vertical's memory namespace, collect every
/// `worker_action` row, and reduce to "latest snapshot per
/// `action_id`". When `vertical` is set, walk only that one.
async fn collect_latest_actions(vertical: Option<&str>) -> ArgoResult<Vec<ActionRecord>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let store_arc = Arc::new(store);
    let memory = MemoryStore::new(store_arc.clone());

    // Build the list of verticals to walk. With --vertical, just
    // that one; otherwise every loaded vertical.
    let vertical_ids: Vec<String> = match vertical {
        Some(v) => vec![v.to_string()],
        None => {
            let dir = resolve_verticals_dir();
            let (registry, _errors) = argo_vertical::load_dir(&dir)?;
            registry.list().map(|(id, _)| id.to_string()).collect()
        }
    };

    let mut latest: HashMap<String, ActionRecord> = HashMap::new();
    for vid in &vertical_ids {
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in entries {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            // Latest-wins via created_at comparison.
            match latest.get(&rec.action_id) {
                Some(prev) if prev.created_at >= rec.created_at => {}
                _ => {
                    latest.insert(rec.action_id.clone(), rec);
                }
            }
        }
    }
    Ok(latest.into_values().collect())
}

fn short(id: &str) -> String {
    if id.len() > 24 {
        format!("{}…", &id[..23])
    } else {
        id.to_string()
    }
}

/// Lookup the manifest's declared `retry.max_attempts` for an
/// action_type, walking every loaded vertical to find the
/// owning declaration. Returns `None` when no manifest declares
/// the type — caller treats that as the conservative default
/// (`1`, no retries). Used by `show` (#755) to surface the
/// retry ceiling alongside the action's current attempt count.
async fn lookup_max_attempts(action_type: &str) -> Option<u32> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errs) = argo_vertical::load_dir(&dir).ok()?;
    for (_id, loaded) in registry.list() {
        if let Some(decl) = loaded
            .manifest
            .actions
            .as_ref()
            .and_then(|a| a.actions.get(action_type))
        {
            return Some(decl.retry.clone().unwrap_or_default().max_attempts);
        }
    }
    None
}

#[cfg(test)]
#[path = "worker_actions_tests.rs"]
mod tests;
