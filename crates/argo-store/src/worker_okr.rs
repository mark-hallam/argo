//! Worker OKR store — quarterly objectives + weighted key-results
//! (Stream `claude-worker-okr-cli`, mission claim 2 worker-goals layer).
//!
//! Charter storage records the *delegation contract* on the worker row;
//! `worker_kpi_value` records observed KPI values; this module records
//! the *intent*: what the worker is trying to achieve in a given quarter
//! (`2026Q2`), and how that intent is scored against the latest KPI
//! observations.
//!
//! KPI-name validation is intentionally absent — names map free-form
//! against `worker_kpi_value.kpi_name`, mirroring the recording layer
//! decision in #1062. The progress join takes the most-recent value per
//! `(worker, tenant, kpi_name)` and computes weighted achievement
//! according to the brief's formula.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use argo_types::{ArgoError, ArgoResult};

const WORKER_OBJECTIVE_SCHEMA: &str = include_str!("../migrations/43__worker_objective.sql");
const WORKER_KEY_RESULT_SCHEMA: &str = include_str!("../migrations/44__worker_key_result.sql");

/// Comparison operator for a key result. The operator-facing tokens are
/// `gte`, `lte`, `eq` to keep the `--kr` CLI parser unambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    /// `actual >= target` is the success criterion.
    Gte,
    /// `actual <= target` is the success criterion.
    Lte,
    /// `actual == target` is the success criterion (within `f64::EPSILON`).
    Eq,
}

impl Comparison {
    /// Parse the operator-facing token used in `--kr <kpi>:<op>:<target>:<weight>`.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "gte" => Ok(Self::Gte),
            "lte" => Ok(Self::Lte),
            "eq" => Ok(Self::Eq),
            _ => Err(ArgoError::Provider(format!(
                "invalid comparison '{s}': expected gte|lte|eq"
            ))),
        }
    }

    /// String form persisted into `worker_key_result.comparison`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gte => "gte",
            Self::Lte => "lte",
            Self::Eq => "eq",
        }
    }
}

/// Lifecycle of a `worker_objective` row. Persisted as the lowercase
/// string form on the `status` column so future variants land additively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    /// In-flight; the default for `set`.
    Active,
    /// Hit. Set by the operator after a successful period close.
    Achieved,
    /// Closed without success.
    Missed,
    /// Withdrawn — counted out of the period scoreboard entirely.
    Cancelled,
}

impl ObjectiveStatus {
    /// String form persisted into `worker_objective.status`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Achieved => "achieved",
            Self::Missed => "missed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Inverse of [`Self::as_str`] — used when reading rows back from
    /// SQLite. Unknown values fall through to [`Self::Active`] so a
    /// future enum variant doesn't silently break existing callers.
    pub fn parse(s: &str) -> Self {
        match s {
            "achieved" => Self::Achieved,
            "missed" => Self::Missed,
            "cancelled" => Self::Cancelled,
            _ => Self::Active,
        }
    }
}

/// One key-result spec, as parsed from `--kr kpi:op:target:weight`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResultSpec {
    /// KPI name to read out of `worker_kpi_value`.
    pub kpi_name: String,
    /// Comparison operator.
    pub comparison: Comparison,
    /// Numeric target the comparison is evaluated against.
    pub target_value: f64,
    /// Relative weight in the period score. Must be `> 0`.
    pub weight: f64,
}

impl KeyResultSpec {
    /// Parse the `kpi:op:target:weight` token used by `argo worker okr
    /// set --kr ...`. Mirrors the brief's contract exactly: four
    /// colon-separated fields, weight strictly positive.
    pub fn parse(token: &str) -> ArgoResult<Self> {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() != 4 {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': expected kpi:op:target:weight"
            )));
        }
        let kpi_name = parts[0].trim().to_string();
        if kpi_name.is_empty() {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': kpi name must not be empty"
            )));
        }
        let comparison = Comparison::parse(parts[1].trim())?;
        let target_value: f64 = parts[2].trim().parse().map_err(|e| {
            ArgoError::Provider(format!("invalid --kr '{token}': target parse: {e}"))
        })?;
        if !target_value.is_finite() {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': target must be finite"
            )));
        }
        let weight: f64 = parts[3].trim().parse().map_err(|e| {
            ArgoError::Provider(format!("invalid --kr '{token}': weight parse: {e}"))
        })?;
        if !weight.is_finite() || weight <= 0.0 {
            return Err(ArgoError::Provider(format!(
                "invalid --kr '{token}': weight must be > 0"
            )));
        }
        Ok(Self { kpi_name, comparison, target_value, weight })
    }
}

/// One persisted objective row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveRow {
    /// Auto-increment id.
    pub id: i64,
    /// Worker id this objective belongs to.
    pub worker_id: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// Period token (`YYYYQ[1-4]`).
    pub period: String,
    /// Operator-facing title.
    pub title: String,
    /// Optional long-form description.
    pub description: Option<String>,
    /// Current lifecycle state.
    pub status: ObjectiveStatus,
    /// Micros since epoch when the row was inserted.
    pub created_at: i64,
}

/// One persisted key-result row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResultRow {
    /// Auto-increment id.
    pub id: i64,
    /// Owning objective.
    pub objective_id: i64,
    /// KPI name read out of `worker_kpi_value`.
    pub kpi_name: String,
    /// Numeric target.
    pub target_value: f64,
    /// Comparison operator.
    pub comparison: Comparison,
    /// Relative weight in the period score (`> 0`).
    pub weight: f64,
}

/// Per-KR slice of a [`ProgressReport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyResultProgress {
    /// The KR row.
    pub key_result: KeyResultRow,
    /// Most-recent observed value, if any. `None` ⇒ the KPI has never
    /// been recorded for the (worker, tenant) pair.
    pub actual_value: Option<f64>,
    /// `[0.0, 1.0]` achievement ratio per the brief formula. `0.0` when
    /// no observation exists.
    pub achievement: f64,
    /// `true` when the comparison itself passes against `actual_value`.
    pub achieved: bool,
}

/// One objective's full scorecard for a period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressReport {
    /// The owning objective row.
    pub objective: ObjectiveRow,
    /// Per-KR detail in insertion order.
    pub key_results: Vec<KeyResultProgress>,
    /// Weighted score in `[0.0, 1.0]`. `0.0` when the objective has no
    /// key-results.
    pub weighted_score: f64,
}

/// Apply both schemas idempotently. Safe to call from every entry point.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    for schema in [WORKER_OBJECTIVE_SCHEMA, WORKER_KEY_RESULT_SCHEMA] {
        for stmt in schema.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("worker_okr schema: {e}")))?;
        }
    }
    Ok(())
}

/// Insert one objective with its key-results in a single transaction.
/// Returns the inserted `objective_id`. The brief allows multiple
/// objectives per `(worker, tenant, period)` — this is `set`, not
/// `replace`.
pub async fn set_objective(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    period: &str,
    title: &str,
    description: Option<&str>,
    krs: &[KeyResultSpec],
) -> ArgoResult<i64> {
    if title.trim().is_empty() {
        return Err(ArgoError::Provider(
            "worker_okr: title must not be empty".into(),
        ));
    }
    if krs.is_empty() {
        return Err(ArgoError::Provider(
            "worker_okr: at least one --kr required".into(),
        ));
    }
    ensure_schema(pool).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_okr begin: {e}")))?;
    let now = Utc::now().timestamp_micros();
    let res = sqlx::query(
        "INSERT INTO worker_objective
            (worker_id, tenant_id, period, title, description, status, created_at)
         VALUES (?, ?, ?, ?, ?, 'active', ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(period)
    .bind(title)
    .bind(description)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_objective insert: {e}")))?;
    let objective_id = res.last_insert_rowid();
    for kr in krs {
        sqlx::query(
            "INSERT INTO worker_key_result
                (objective_id, kpi_name, target_value, comparison, weight)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(objective_id)
        .bind(&kr.kpi_name)
        .bind(kr.target_value)
        .bind(kr.comparison.as_str())
        .bind(kr.weight)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_key_result insert: {e}")))?;
    }
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_okr commit: {e}")))?;
    Ok(objective_id)
}

/// List objectives for one worker, newest-first, optionally filtered by
/// `period`.
pub async fn list_objectives(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    period: Option<&str>,
) -> ArgoResult<Vec<ObjectiveRow>> {
    ensure_schema(pool).await?;
    let mut sql = String::from(
        "SELECT id, worker_id, tenant_id, period, title, description, status, created_at
         FROM worker_objective
         WHERE worker_id = ? AND tenant_id = ?",
    );
    if period.is_some() {
        sql.push_str(" AND period = ?");
    }
    sql.push_str(" ORDER BY created_at DESC, id DESC");
    let mut q = sqlx::query(&sql).bind(worker_id).bind(tenant_id);
    if let Some(p) = period {
        q = q.bind(p);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_objective list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let status: String = row.get("status");
            ObjectiveRow {
                id: row.get("id"),
                worker_id: row.get("worker_id"),
                tenant_id: row.get("tenant_id"),
                period: row.get("period"),
                title: row.get("title"),
                description: row.get("description"),
                status: ObjectiveStatus::parse(&status),
                created_at: row.get("created_at"),
            }
        })
        .collect())
}

/// Fetch the key-results for one objective in insertion order.
pub async fn key_results(pool: &SqlitePool, objective_id: i64) -> ArgoResult<Vec<KeyResultRow>> {
    ensure_schema(pool).await?;
    let rows = sqlx::query(
        "SELECT id, objective_id, kpi_name, target_value, comparison, weight
         FROM worker_key_result
         WHERE objective_id = ?
         ORDER BY id ASC",
    )
    .bind(objective_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_key_result list: {e}")))?;
    rows.into_iter()
        .map(|row| {
            let comparison: String = row.get("comparison");
            Ok(KeyResultRow {
                id: row.get("id"),
                objective_id: row.get("objective_id"),
                kpi_name: row.get("kpi_name"),
                target_value: row.get("target_value"),
                comparison: Comparison::parse(&comparison)?,
                weight: row.get("weight"),
            })
        })
        .collect()
}

/// Latest observed value of one KPI for `(worker, tenant)` — the join
/// predicate the brief calls out for `progress`. Returns `None` when
/// no observation has been recorded.
pub async fn latest_kpi_value(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    kpi_name: &str,
) -> ArgoResult<Option<f64>> {
    let row = sqlx::query(
        "SELECT value FROM worker_kpi_value
         WHERE worker_id = ? AND tenant_id = ? AND kpi_name = ?
         ORDER BY observed_at DESC, id DESC
         LIMIT 1",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(kpi_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_kpi latest: {e}")))?;
    Ok(row.map(|r| r.get::<f64, _>("value")))
}

/// Per-KR achievement ratio per the brief formula:
///   - 1.0 if the comparison passes
///   - else `actual/target` (or `target/actual` for `lte`) clamped to
///     `[0.0, 1.0]`
///   - 0.0 if `actual` is `None`
///
/// Public so tests can drive the math without a SQLite round-trip.
pub fn achievement(actual: Option<f64>, target: f64, comparison: Comparison) -> (f64, bool) {
    let Some(actual) = actual else {
        return (0.0, false);
    };
    let achieved = match comparison {
        Comparison::Gte => actual >= target,
        Comparison::Lte => actual <= target,
        Comparison::Eq => (actual - target).abs() < f64::EPSILON,
    };
    if achieved {
        return (1.0, true);
    }
    let ratio = match comparison {
        Comparison::Gte => {
            if target == 0.0 {
                0.0
            } else {
                actual / target
            }
        }
        Comparison::Lte => {
            if actual == 0.0 {
                1.0
            } else {
                target / actual
            }
        }
        Comparison::Eq => {
            // Symmetric distance — closer to target is better. Handles
            // both target=0 (unreachable from non-zero actual) and
            // actual=0 (always misses unless target is also 0).
            if target == 0.0 || actual == 0.0 {
                0.0
            } else {
                let smaller = target.abs().min(actual.abs());
                let larger = target.abs().max(actual.abs());
                smaller / larger
            }
        }
    };
    (ratio.clamp(0.0, 1.0), false)
}

/// Compute the weighted score for one objective. Public so tests can
/// drive the math directly.
pub fn weighted_score(progress: &[KeyResultProgress]) -> f64 {
    let total_weight: f64 = progress.iter().map(|p| p.key_result.weight).sum();
    if total_weight <= 0.0 {
        return 0.0;
    }
    let numerator: f64 = progress
        .iter()
        .map(|p| p.key_result.weight * p.achievement)
        .sum();
    (numerator / total_weight).clamp(0.0, 1.0)
}

/// Compute progress for every objective owned by `(worker, tenant)`,
/// optionally filtered by `period`. Joins each KR against the latest
/// `worker_kpi_value` row.
pub async fn progress(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    period: Option<&str>,
) -> ArgoResult<Vec<ProgressReport>> {
    let objectives = list_objectives(pool, worker_id, tenant_id, period).await?;
    let mut out = Vec::with_capacity(objectives.len());
    for objective in objectives {
        let krs = key_results(pool, objective.id).await?;
        let mut kr_progress = Vec::with_capacity(krs.len());
        for kr in krs {
            let actual =
                latest_kpi_value(pool, worker_id, tenant_id, &kr.kpi_name).await?;
            let (ratio, achieved) = achievement(actual, kr.target_value, kr.comparison);
            kr_progress.push(KeyResultProgress {
                key_result: kr,
                actual_value: actual,
                achievement: ratio,
                achieved,
            });
        }
        let score = weighted_score(&kr_progress);
        out.push(ProgressReport {
            objective,
            key_results: kr_progress,
            weighted_score: score,
        });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "worker_okr_tests.rs"]
mod tests;
