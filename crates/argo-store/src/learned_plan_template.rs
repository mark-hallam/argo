//! Persistent learned-plan-template store (Stream
//! `claude-dreams-plan-templates`).
//!
//! Holds the polished workflow YAML the Dreams API distilled from
//! a cohort of successful Planner decompositions. Lifecycle:
//!
//!   * `Pending`   — distilled but not yet operator-approved
//!   * `Promoted`  — operator approved via
//!                   `argo plan template promote`; the
//!                   `TemplatePlanner` consults these before
//!                   falling through to LlmPlanner
//!   * `Rejected`  — operator rejected; never matched
//!
//! The store is a pure data sink: it owns the schema and CRUD
//! surface but knows nothing about the Planner trait. The runtime
//! layer (`argo_runtime::dreams_plan_templates`) calls into the
//! sink trait defined here when a Dreams distillation produces a
//! new candidate; the CLI handlers (`argo plan template ...`)
//! drive promotion + listing.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use argo_types::{ArgoError, ArgoResult};

const LEARNED_PLAN_TEMPLATE_SCHEMA: &str =
    include_str!("../migrations/30__learned_plan_templates.sql");

/// Lifecycle state of a [`LearnedPlanTemplateRow`]. Mirrored as a
/// lower-case string in the `trust_grade` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustGrade {
    /// Distilled by Dreams; awaits operator approval.
    Pending,
    /// Operator-approved via `argo plan template promote`.
    Promoted,
    /// Operator rejected; never matched by TemplatePlanner.
    Rejected,
}

impl TrustGrade {
    /// Lower-case label used in the `trust_grade` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Promoted => "promoted",
            Self::Rejected => "rejected",
        }
    }

    /// Parse the column label. Unknown values fall back to
    /// [`TrustGrade::Pending`] so a forward-rolled DB doesn't
    /// crash older readers.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "promoted" => Self::Promoted,
            "rejected" => Self::Rejected,
            _ => Self::Pending,
        }
    }
}

/// One persisted learned-plan-template row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnedPlanTemplateRow {
    /// UUIDv7 id.
    pub id: String,
    /// Stable name used by the planner + CLI.
    pub name: String,
    /// One-line description shown to operators.
    pub description: String,
    /// Anchor cohort goal that seeded the template.
    pub parent_goal: String,
    /// Lower-case CSV keywords the planner matches against.
    pub keywords_csv: String,
    /// Workflow YAML body the Dreams call returned.
    pub yaml: String,
    /// Lifecycle state.
    pub trust_grade: TrustGrade,
    /// Cohort size at distillation time.
    pub cohort_size: i64,
    /// Created-at micros since epoch.
    pub created_at: i64,
    /// Promoted-at micros since epoch (None while not promoted).
    pub promoted_at: Option<i64>,
}

impl LearnedPlanTemplateRow {
    /// Comma-separated `keywords_csv` parsed back into a Vec.
    /// Empty entries are dropped.
    #[must_use]
    pub fn keywords(&self) -> Vec<String> {
        self.keywords_csv
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect()
    }
}

/// Insert payload for [`insert`]. Lets callers populate every
/// column without dragging the auto-managed `id` and
/// `created_at` through their type signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLearnedPlanTemplate<'a> {
    /// Stable name used by the planner + CLI.
    pub name: &'a str,
    /// One-line description shown to operators.
    pub description: &'a str,
    /// Anchor cohort goal that seeded the template.
    pub parent_goal: &'a str,
    /// Lower-case CSV keywords the planner matches against.
    pub keywords_csv: &'a str,
    /// Workflow YAML body the Dreams call returned.
    pub yaml: &'a str,
    /// Cohort size at distillation time.
    pub cohort_size: i64,
}

/// Filter set for [`list`]. All fields default to None (no filter).
#[derive(Debug, Default, Clone)]
pub struct LearnedPlanTemplateFilters {
    /// Restrict to one lifecycle state.
    pub trust_grade: Option<TrustGrade>,
}

/// Ensure the schema is in place. Idempotent — safe to call from
/// every entry point so callers don't have to coordinate with the
/// global migration runner.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    for stmt in LEARNED_PLAN_TEMPLATE_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("learned_plan_template schema: {e}")))?;
    }
    Ok(())
}

/// Insert one row. Returns the assigned id so the caller can
/// promote it later.
pub async fn insert(
    pool: &SqlitePool,
    new: &NewLearnedPlanTemplate<'_>,
) -> ArgoResult<String> {
    ensure_schema(pool).await?;
    let id = Uuid::now_v7().to_string();
    let created_at = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO learned_plan_template \
         (id, name, description, parent_goal, keywords_csv, yaml, trust_grade, cohort_size, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&id)
    .bind(new.name)
    .bind(new.description)
    .bind(new.parent_goal)
    .bind(new.keywords_csv)
    .bind(new.yaml)
    .bind(new.cohort_size)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("learned_plan_template insert: {e}")))?;
    Ok(id)
}

/// Promote a row — marks it as `promoted` and stamps `promoted_at`.
/// Errors on unknown id or on a row that is already `promoted`
/// (callers should guard via [`get`] first).
pub async fn promote(pool: &SqlitePool, id: &str) -> ArgoResult<()> {
    set_grade(pool, id, TrustGrade::Promoted).await
}

/// Reject a row — marks it as `rejected` so the planner never
/// picks it up. Errors on unknown id.
pub async fn reject(pool: &SqlitePool, id: &str) -> ArgoResult<()> {
    set_grade(pool, id, TrustGrade::Rejected).await
}

async fn set_grade(pool: &SqlitePool, id: &str, grade: TrustGrade) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    let now = Utc::now().timestamp_micros();
    let promoted_stamp =
        matches!(grade, TrustGrade::Promoted).then_some(now);
    let rows = sqlx::query(
        "UPDATE learned_plan_template \
         SET trust_grade = ?, \
             promoted_at = COALESCE(?, promoted_at) \
         WHERE id = ?",
    )
    .bind(grade.as_str())
    .bind(promoted_stamp)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("learned_plan_template set_grade: {e}")))?;
    if rows.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "learned_plan_template set_grade: no row with id '{id}'"
        )));
    }
    Ok(())
}

/// Fetch one row by id.
pub async fn get(pool: &SqlitePool, id: &str) -> ArgoResult<Option<LearnedPlanTemplateRow>> {
    ensure_schema(pool).await?;
    let row: Option<RowTuple> = sqlx::query_as(SELECT_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("learned_plan_template get: {e}")))?;
    Ok(row.map(into_row))
}

/// List rows under `filters`, newest-first, capped at `limit`.
pub async fn list(
    pool: &SqlitePool,
    filters: &LearnedPlanTemplateFilters,
    limit: u32,
) -> ArgoResult<Vec<LearnedPlanTemplateRow>> {
    ensure_schema(pool).await?;
    let mut sql = String::from(SELECT_COLUMNS);
    if filters.trust_grade.is_some() {
        sql.push_str(" WHERE trust_grade = ?");
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");

    let mut q = sqlx::query_as::<_, RowTuple>(&sql);
    if let Some(g) = filters.trust_grade {
        q = q.bind(g.as_str());
    }
    q = q.bind(i64::from(limit));
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("learned_plan_template list: {e}")))?;
    Ok(rows.into_iter().map(into_row).collect())
}

/// Convenience: every promoted row, ordered newest-first. Used by
/// the CLI's plan-generate path to seed the TemplatePlanner with
/// learned templates.
pub async fn list_promoted(pool: &SqlitePool) -> ArgoResult<Vec<LearnedPlanTemplateRow>> {
    list(
        pool,
        &LearnedPlanTemplateFilters {
            trust_grade: Some(TrustGrade::Promoted),
        },
        u32::MAX,
    )
    .await
}

const SELECT_COLUMNS: &str =
    "SELECT id, name, description, parent_goal, keywords_csv, yaml, trust_grade, \
     cohort_size, created_at, promoted_at FROM learned_plan_template";

const SELECT_BY_ID: &str =
    "SELECT id, name, description, parent_goal, keywords_csv, yaml, trust_grade, \
     cohort_size, created_at, promoted_at FROM learned_plan_template WHERE id = ?";

type RowTuple = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    Option<i64>,
);

fn into_row(t: RowTuple) -> LearnedPlanTemplateRow {
    LearnedPlanTemplateRow {
        id: t.0,
        name: t.1,
        description: t.2,
        parent_goal: t.3,
        keywords_csv: t.4,
        yaml: t.5,
        trust_grade: TrustGrade::from_str_lossy(&t.6),
        cohort_size: t.7,
        created_at: t.8,
        promoted_at: t.9,
    }
}

#[cfg(test)]
#[path = "learned_plan_template_tests.rs"]
mod tests;
