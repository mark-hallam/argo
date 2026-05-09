//! Persistent subtask ledger (Stream `claude-subtask-ledger-store`).
//!
//! When the [`argo_runtime::Planner`] decomposes a coarse goal into a
//! workflow DAG, each task in the DAG is logged here as a row so the
//! decomposition is durable across restarts and inspectable via
//! `argo subtask list/show/tree`.
//!
//! The store is a pure data sink: it owns the schema and the CRUD
//! surface but knows nothing about the Planner trait. The `argo_cli`
//! layer is the one that calls [`record_decomposition`] right after
//! `Planner::decompose` returns, then mutates rows via
//! [`update_state`] as the workflow runner picks them up.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult};

/// Lifecycle states a [`SubtaskRow`] can occupy. Mirrored as a
/// lower-case string in the `state` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtaskState {
    /// Inserted by the Planner; not yet picked up.
    Pending,
    /// Workflow runner has begun executing the node.
    Running,
    /// Node completed successfully; `output` populated.
    Done,
    /// Node failed; `output` may carry the error payload.
    Failed,
    /// Node skipped (dependency failed, gated, etc).
    Skipped,
}

impl SubtaskState {
    /// Lower-case label used in the `state` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    /// Parse the column label back to a state. Unknown values fall
    /// back to [`SubtaskState::Pending`] so a forward-rolled DB
    /// doesn't crash older readers.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Pending,
        }
    }
}

/// One persisted subtask row. Matches the `subtask_ledger` table
/// 1:1 so callers can `SELECT *` and round-trip cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtaskRow {
    /// UUIDv7 id.
    pub id: String,
    /// Coarse goal the Planner decomposed.
    pub parent_goal: String,
    /// Optional Session id this subtask belongs to.
    pub parent_session_id: Option<String>,
    /// Owning worker.
    pub worker_id: String,
    /// DAG path, typically `/<workflow_id>/<task_id>`.
    pub plan_node_path: String,
    /// Free-text description of the node.
    pub description: String,
    /// Lifecycle state.
    pub state: SubtaskState,
    /// JSON-encoded output payload (only when `state == Done`).
    pub output: Option<String>,
    /// Started-at micros since epoch.
    pub started_at: Option<i64>,
    /// Completed-at micros since epoch.
    pub completed_at: Option<i64>,
    /// Created-at micros since epoch.
    pub created_at: i64,
}

/// Insert payload for [`insert`]. Lets callers populate every
/// column without dragging the auto-managed `id` and `created_at`
/// through their type signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSubtask<'a> {
    /// Coarse goal the Planner decomposed.
    pub parent_goal: &'a str,
    /// Optional Session id this subtask belongs to.
    pub parent_session_id: Option<&'a str>,
    /// Owning worker.
    pub worker_id: &'a str,
    /// DAG path, typically `/<workflow_id>/<task_id>`.
    pub plan_node_path: &'a str,
    /// Free-text description of the node.
    pub description: &'a str,
}

/// Filter set for [`list`]. All fields default to `None` (no
/// filter); the caller picks any combination it needs.
#[derive(Debug, Default, Clone)]
pub struct SubtaskFilters<'a> {
    /// Restrict to one parent session id.
    pub parent_session_id: Option<&'a str>,
    /// Restrict to one worker id.
    pub worker_id: Option<&'a str>,
    /// Restrict to one parent goal (exact match).
    pub parent_goal: Option<&'a str>,
    /// Restrict to one lifecycle state.
    pub state: Option<SubtaskState>,
}

/// Insert one row. Returns the assigned id so the caller can keep
/// it alongside the in-memory plan node for state-transition writes.
/// Schema is owned by `migrations/v39.rs`; callers must run
/// `SessionStore::migrate` before use.
pub async fn insert(pool: &SqlitePool, new: &NewSubtask<'_>) -> ArgoResult<String> {
    let id = Uuid::now_v7().to_string();
    let created_at = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO subtask_ledger \
         (id, parent_goal, parent_session_id, worker_id, plan_node_path, description, state, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&id)
    .bind(new.parent_goal)
    .bind(new.parent_session_id)
    .bind(new.worker_id)
    .bind(new.plan_node_path)
    .bind(new.description)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("subtask insert: {e}")))?;
    Ok(id)
}

/// Update a row's state, with optional `output` payload (recorded
/// on `done`/`failed`). Bumps `started_at` when transitioning into
/// `running`, and `completed_at` when transitioning into
/// `done`/`failed`/`skipped`.
pub async fn update_state(
    pool: &SqlitePool,
    id: &str,
    state: SubtaskState,
    output: Option<&str>,
) -> ArgoResult<()> {
    let now = Utc::now().timestamp_micros();
    let started = matches!(state, SubtaskState::Running).then_some(now);
    let completed = matches!(
        state,
        SubtaskState::Done | SubtaskState::Failed | SubtaskState::Skipped
    )
    .then_some(now);
    let rows = sqlx::query(
        "UPDATE subtask_ledger \
         SET state = ?, \
             output = COALESCE(?, output), \
             started_at = COALESCE(started_at, ?), \
             completed_at = COALESCE(?, completed_at) \
         WHERE id = ?",
    )
    .bind(state.as_str())
    .bind(output)
    .bind(started)
    .bind(completed)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("subtask update_state: {e}")))?;
    if rows.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "subtask update_state: no row with id '{id}'"
        )));
    }
    Ok(())
}

/// Fetch one row by id.
pub async fn get(pool: &SqlitePool, id: &str) -> ArgoResult<Option<SubtaskRow>> {
    let row: Option<RowTuple> = sqlx::query_as(SELECT_COLUMNS_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("subtask get: {e}")))?;
    Ok(row.map(into_subtask))
}

/// List rows under `filters`, ordered newest-first, capped at
/// `limit`.
pub async fn list(
    pool: &SqlitePool,
    filters: &SubtaskFilters<'_>,
    limit: u32,
) -> ArgoResult<Vec<SubtaskRow>> {
    let mut sql = String::from(SELECT_COLUMNS);
    let mut clauses: Vec<&str> = Vec::new();
    if filters.parent_session_id.is_some() {
        clauses.push("parent_session_id = ?");
    }
    if filters.worker_id.is_some() {
        clauses.push("worker_id = ?");
    }
    if filters.parent_goal.is_some() {
        clauses.push("parent_goal = ?");
    }
    if filters.state.is_some() {
        clauses.push("state = ?");
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");

    let mut q = sqlx::query_as::<_, RowTuple>(&sql);
    if let Some(s) = filters.parent_session_id {
        q = q.bind(s);
    }
    if let Some(w) = filters.worker_id {
        q = q.bind(w);
    }
    if let Some(g) = filters.parent_goal {
        q = q.bind(g);
    }
    if let Some(state) = filters.state {
        q = q.bind(state.as_str());
    }
    q = q.bind(i64::from(limit));
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("subtask list: {e}")))?;
    Ok(rows.into_iter().map(into_subtask).collect())
}

/// SQLite-backed implementation of [`argo_runtime::SubtaskSink`].
/// Wraps the pool so the [`argo_runtime::record_decomposition`]
/// helper can write rows directly into the ledger immediately
/// after `Planner::decompose` returns.
pub struct SqliteSubtaskSink {
    pool: SqlitePool,
}

impl SqliteSubtaskSink {
    /// Wrap an existing pool. Cheap (clones the pool handle).
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl argo_runtime::SubtaskSink for SqliteSubtaskSink {
    async fn record_subtask(
        &self,
        parent_goal: &str,
        parent_session_id: Option<&str>,
        subtask: &argo_runtime::PlannedSubtask,
    ) -> Result<(), argo_runtime::PlannerError> {
        let new = NewSubtask {
            parent_goal,
            parent_session_id,
            worker_id: &subtask.worker_id,
            plan_node_path: &subtask.plan_node_path,
            description: &subtask.description,
        };
        match insert(&self.pool, &new).await {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = format!("{e}").to_lowercase();
                // Re-decompose of an unchanged plan must be
                // idempotent — swallow the unique-index hit.
                if msg.contains("unique") {
                    Ok(())
                } else {
                    Err(argo_runtime::PlannerError::Llm(format!("subtask sink: {e}")))
                }
            }
        }
    }
}

/// Convenience: every row whose `parent_session_id` matches,
/// ordered by `created_at` ascending so callers can render the
/// plan in insert order. Used by `argo subtask tree`.
pub async fn list_by_session(pool: &SqlitePool, session_id: &str) -> ArgoResult<Vec<SubtaskRow>> {
    let rows: Vec<RowTuple> = sqlx::query_as(SELECT_COLUMNS_BY_SESSION)
        .bind(session_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("subtask list_by_session: {e}")))?;
    Ok(rows.into_iter().map(into_subtask).collect())
}

const SELECT_COLUMNS: &str = "SELECT id, parent_goal, parent_session_id, worker_id, \
                              plan_node_path, description, state, output, started_at, \
                              completed_at, created_at FROM subtask_ledger";

const SELECT_COLUMNS_BY_ID: &str = "SELECT id, parent_goal, parent_session_id, worker_id, \
                                    plan_node_path, description, state, output, started_at, \
                                    completed_at, created_at FROM subtask_ledger WHERE id = ?";

const SELECT_COLUMNS_BY_SESSION: &str = "SELECT id, parent_goal, parent_session_id, worker_id, \
                                         plan_node_path, description, state, output, started_at, \
                                         completed_at, created_at FROM subtask_ledger \
                                         WHERE parent_session_id = ? ORDER BY created_at ASC";

type RowTuple = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<i64>,
    i64,
);

fn into_subtask(t: RowTuple) -> SubtaskRow {
    SubtaskRow {
        id: t.0,
        parent_goal: t.1,
        parent_session_id: t.2,
        worker_id: t.3,
        plan_node_path: t.4,
        description: t.5,
        state: SubtaskState::from_str_lossy(&t.6),
        output: t.7,
        started_at: t.8,
        completed_at: t.9,
        created_at: t.10,
    }
}

#[cfg(test)]
#[path = "subtask_tests.rs"]
mod tests;
