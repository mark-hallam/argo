//! Per-subtask budget tree — `set_subtask_budget`,
//! `record_subtask_spend`, `get_remaining`, and `walk_tree` for
//! Stream `mission-delegation-budget-tree` (MISSION_BACKLOG Section 3
//! Claim 2 #57).
//!
//! Sibling to [`crate::subtask`]: extends the `subtask_ledger` row
//! with declared cap (`budget_usd_cents`), accumulated spend
//! (`spent_usd_cents`), and an explicit parent pointer
//! (`parent_subtask_id`) so a delegating worker's budget caps its
//! children's total spend. All four production helpers live here;
//! the original `subtask` module is untouched.
//!
//! Semantics:
//!
//!   * NULL `budget_usd_cents` = unbounded; [`get_remaining`]
//!     returns `None` and the runtime treats it as "no
//!     enforcement". Legacy rows from before the schema bump
//!     fall through this path.
//!   * [`record_subtask_spend`] is additive and always succeeds —
//!     the brief deliberately allows the accumulator to exceed the
//!     declared cap so audit trails record what was actually spent;
//!     `get_remaining` reports a negative number in that case, which
//!     the runtime gate reads as "abort".
//!   * [`walk_tree`] uses a `WITH RECURSIVE` CTE over the
//!     `parent_subtask_id` self-reference; each returned row's
//!     `aggregated_spent` is `row.spent_usd_cents + sum over all
//!     descendants` so the operator can read a roll-up from any
//!     intermediate node.
//!
//! The [`argo_runtime::SubtaskSink`] impl on
//! [`crate::subtask::SqliteSubtaskSink`] gets a new method override
//! ([`set_subtask_budget_by_natural_key`]) so
//! `argo_runtime::record_decomposition` can plumb a parent's
//! declared budget down to each child without round-tripping
//! through the id.
use crate::subtask::{SubtaskRow, SubtaskState};
use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

/// Set (or re-set) a subtask's declared budget cap. Pass any
/// non-negative integer cents value; the runtime treats a cap of
/// zero as "any spend exceeds budget" — that's a legitimate
/// way to immediately gate further work.
pub async fn set_subtask_budget(
    pool: &SqlitePool,
    subtask_id: &str,
    budget_usd_cents: i64,
) -> ArgoResult<()> {
    let rows = sqlx::query(
        "UPDATE subtask_ledger SET budget_usd_cents = ? WHERE id = ?",
    )
    .bind(budget_usd_cents)
    .bind(subtask_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("set_subtask_budget: {e}")))?;
    if rows.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "set_subtask_budget: no row with id '{subtask_id}'"
        )));
    }
    Ok(())
}

/// Identify a row by the same `(parent_session_id, worker_id,
/// plan_node_path)` natural key the `SqliteSubtaskSink` insert path
/// uses, then set its budget. Used by `record_decomposition` to
/// plumb the divided parent budget down to each child without
/// round-tripping through the id, since the existing
/// [`argo_runtime::SubtaskSink::record_subtask`] hand-off discards
/// the post-INSERT id.
pub async fn set_subtask_budget_by_natural_key(
    pool: &SqlitePool,
    parent_session_id: Option<&str>,
    worker_id: &str,
    plan_node_path: &str,
    budget_usd_cents: i64,
) -> ArgoResult<()> {
    // The unique index `idx_subtask_ledger_node_unique` only covers
    // `(parent_session_id, worker_id, plan_node_path)` where
    // `parent_session_id IS NOT NULL`. The UPDATE clause is robust
    // for both shapes — we use `IS` (SQLite null-safe equality) so
    // a NULL session id still finds its row.
    let rows = sqlx::query(
        "UPDATE subtask_ledger \
         SET budget_usd_cents = ? \
         WHERE parent_session_id IS ? AND worker_id = ? AND plan_node_path = ?",
    )
    .bind(budget_usd_cents)
    .bind(parent_session_id)
    .bind(worker_id)
    .bind(plan_node_path)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("set_subtask_budget_by_natural_key: {e}")))?;
    if rows.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "set_subtask_budget_by_natural_key: no row for \
             (session={parent_session_id:?}, worker={worker_id}, path={plan_node_path})"
        )));
    }
    Ok(())
}

/// Attach a child subtask to its parent. Lets a multi-level
/// decomposition record the tree structure that `walk_tree`
/// traverses. Top-level (root) subtasks leave this NULL.
pub async fn set_subtask_parent(
    pool: &SqlitePool,
    subtask_id: &str,
    parent_subtask_id: &str,
) -> ArgoResult<()> {
    let rows = sqlx::query(
        "UPDATE subtask_ledger SET parent_subtask_id = ? WHERE id = ?",
    )
    .bind(parent_subtask_id)
    .bind(subtask_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("set_subtask_parent: {e}")))?;
    if rows.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "set_subtask_parent: no row with id '{subtask_id}'"
        )));
    }
    Ok(())
}

/// Add `delta_usd_cents` to `spent_usd_cents`. Always succeeds for
/// known rows — the brief deliberately keeps this additive so the
/// audit trail captures what was actually charged even after the
/// cap is breached. Pass a non-negative delta; the runtime only
/// charges forward.
pub async fn record_subtask_spend(
    pool: &SqlitePool,
    subtask_id: &str,
    delta_usd_cents: i64,
) -> ArgoResult<()> {
    let rows = sqlx::query(
        "UPDATE subtask_ledger \
         SET spent_usd_cents = spent_usd_cents + ? \
         WHERE id = ?",
    )
    .bind(delta_usd_cents)
    .bind(subtask_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("record_subtask_spend: {e}")))?;
    if rows.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "record_subtask_spend: no row with id '{subtask_id}'"
        )));
    }
    Ok(())
}

/// Remaining budget for one subtask. Returns:
///
///   * `None` when the row has NULL `budget_usd_cents` (unbounded);
///     legacy rows from before the schema bump land here.
///   * `Some(budget - spent)` otherwise. Can be negative when the
///     accumulator overran the cap — the runtime gate reads any
///     `<= 0` value as "abort the next turn".
pub async fn get_remaining(pool: &SqlitePool, subtask_id: &str) -> ArgoResult<Option<i64>> {
    let row: Option<(Option<i64>, i64)> = sqlx::query_as(
        "SELECT budget_usd_cents, spent_usd_cents \
         FROM subtask_ledger WHERE id = ?",
    )
    .bind(subtask_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("get_remaining: {e}")))?;
    let (budget, spent) = match row {
        Some(r) => r,
        None => {
            return Err(ArgoError::Provider(format!(
                "get_remaining: no row with id '{subtask_id}'"
            )));
        }
    };
    Ok(budget.map(|b| b - spent))
}

/// One node in a `walk_tree` traversal — the full row plus the
/// sum of `spent_usd_cents` across this row and every descendant
/// (rooted at the originally requested id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedSubtask {
    /// The row itself (carries its own per-node `spent_usd_cents`
    /// in `SubtaskRow` via the budget columns surfaced through
    /// [`SubtaskBudgetExt`]).
    pub row: SubtaskRow,
    /// Declared cap on this row (NULL = unbounded).
    pub budget_usd_cents: Option<i64>,
    /// Per-row accumulator (matches the column value).
    pub spent_usd_cents: i64,
    /// `spent_usd_cents` summed across this row and every
    /// descendant via `parent_subtask_id`. Equals
    /// `spent_usd_cents` for a leaf.
    pub aggregated_spent_usd_cents: i64,
    /// This row's parent id (None for the originally requested
    /// root).
    pub parent_subtask_id: Option<String>,
}

/// Walk the budget tree rooted at `root_subtask_id` and return every
/// row plus its descendant-aggregated spend. Uses a `WITH RECURSIVE`
/// CTE over `parent_subtask_id`; the resulting rows are returned in
/// `created_at` ascending order so the caller can render a stable
/// hierarchical view.
///
/// Returns an empty vector when the root id matches no row.
pub async fn walk_tree(pool: &SqlitePool, root_subtask_id: &str) -> ArgoResult<Vec<WalkedSubtask>> {
    // The recursive CTE collects every descendant of the root,
    // including the root itself. The aggregation pass then sums
    // `spent_usd_cents` over each row's subtree by re-walking the
    // descendant set keyed on the candidate's id — a tiny constant
    // for the row counts we expect (decomposition trees are
    // single-digit fanout, single-digit depth).
    let rows: Vec<SubtaskTreeRow> = sqlx::query_as(WALK_TREE_SQL)
        .bind(root_subtask_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("walk_tree: {e}")))?;

    // Build a child index for the in-memory aggregation pass.
    let mut by_id: std::collections::HashMap<&str, &SubtaskTreeRow> = std::collections::HashMap::new();
    let mut children_of: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for r in &rows {
        by_id.insert(r.id.as_str(), r);
        if let Some(p) = r.parent_subtask_id.as_deref() {
            children_of.entry(p).or_default().push(r.id.as_str());
        }
    }
    fn sum_subtree(
        id: &str,
        by_id: &std::collections::HashMap<&str, &SubtaskTreeRow>,
        children_of: &std::collections::HashMap<&str, Vec<&str>>,
    ) -> i64 {
        let self_spent = by_id.get(id).map_or(0, |r| r.spent_usd_cents);
        let descendants: i64 = children_of
            .get(id)
            .map(|cs| cs.iter().map(|c| sum_subtree(c, by_id, children_of)).sum())
            .unwrap_or(0);
        self_spent + descendants
    }

    let mut out: Vec<WalkedSubtask> = Vec::with_capacity(rows.len());
    for r in &rows {
        let agg = sum_subtree(&r.id, &by_id, &children_of);
        out.push(WalkedSubtask {
            row: r.to_subtask_row(),
            budget_usd_cents: r.budget_usd_cents,
            spent_usd_cents: r.spent_usd_cents,
            aggregated_spent_usd_cents: agg,
            parent_subtask_id: r.parent_subtask_id.clone(),
        });
    }
    Ok(out)
}


const WALK_TREE_SQL: &str = "\
WITH RECURSIVE tree(id, parent_goal, parent_session_id, worker_id, plan_node_path, \
                    description, state, output, started_at, completed_at, created_at, \
                    budget_usd_cents, spent_usd_cents, parent_subtask_id) AS ( \
    SELECT id, parent_goal, parent_session_id, worker_id, plan_node_path, description, \
           state, output, started_at, completed_at, created_at, budget_usd_cents, \
           spent_usd_cents, parent_subtask_id \
    FROM subtask_ledger WHERE id = ? \
    UNION ALL \
    SELECT s.id, s.parent_goal, s.parent_session_id, s.worker_id, s.plan_node_path, \
           s.description, s.state, s.output, s.started_at, s.completed_at, s.created_at, \
           s.budget_usd_cents, s.spent_usd_cents, s.parent_subtask_id \
    FROM subtask_ledger s \
    INNER JOIN tree t ON s.parent_subtask_id = t.id \
) \
SELECT * FROM tree ORDER BY created_at ASC";

#[derive(Debug, sqlx::FromRow)]
struct SubtaskTreeRow {
    id: String,
    parent_goal: String,
    parent_session_id: Option<String>,
    worker_id: String,
    plan_node_path: String,
    description: String,
    state: String,
    output: Option<String>,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    created_at: i64,
    budget_usd_cents: Option<i64>,
    spent_usd_cents: i64,
    parent_subtask_id: Option<String>,
}

impl SubtaskTreeRow {
    fn to_subtask_row(&self) -> SubtaskRow {
        SubtaskRow {
            id: self.id.clone(),
            parent_goal: self.parent_goal.clone(),
            parent_session_id: self.parent_session_id.clone(),
            worker_id: self.worker_id.clone(),
            plan_node_path: self.plan_node_path.clone(),
            description: self.description.clone(),
            state: SubtaskState::from_str_lossy(&self.state),
            output: self.output.clone(),
            started_at: self.started_at,
            completed_at: self.completed_at,
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
#[path = "subtask_budget_tests.rs"]
mod tests;
