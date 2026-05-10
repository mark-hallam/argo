//! V2.0 — A/B testing across two variants of the same skill.
//!
//! ## Run lifecycle
//!
//! 1. Operator runs `argo skill ab-test <skill> --variants vN,vM
//!    --runs 100` → inserts a `skill_ab_test_run` row in `running`
//!    state. The two variants must already exist as separate rows
//!    in the `skill` table (UNIQUE name+version from migration v1).
//! 2. Each invocation that targets that skill calls
//!    [`select_variant`] with the run id + invocation id; a
//!    deterministic SHA-256 hash of the invocation id picks A or B
//!    50/50, then [`record_outcome`] writes the result row.
//! 3. After every recorded outcome, [`maybe_complete`] checks
//!    whether (a) at least `min_samples_per_variant` outcomes have
//!    landed for *each* variant, and (b) the chi-square statistic
//!    rejects the null at α=0.05. If both, the run flips to
//!    `completed` with the higher-success-rate variant as winner.
//! 4. Operator runs `argo skill ab-promote <run-id>` to mark the
//!    losing version `superseded_by` the winner.
//!
//! Pure statistical helpers (`select_variant`, `significance`,
//! `Variant`, `VariantCounts`, `Significance`, the chi-square
//! constant, the default min-sample constant) live in
//! [`crate::ab_test_stats`] to keep this file under the per-file
//! 500-LOC budget; they are re-exported here so the public surface
//! is unchanged.

use crate::version::find_version;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

pub use crate::ab_test_stats::{
    select_variant, significance, Significance, Variant, VariantCounts,
    CHI_SQUARE_CRIT_DF1_ALPHA_05, DEFAULT_MIN_SAMPLES_PER_VARIANT,
};

const ENSURE_AB_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/20__skill_ab_tests.sql");

/// Lifecycle status for a run.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Cancelled,
}

#[allow(missing_docs)]
impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Run row + computed status snapshot. Returned by
/// `argo skill ab-status`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTestRun {
    pub id: String,
    pub skill_name: String,
    pub variant_a: String,
    pub variant_b: String,
    pub target_runs: i64,
    pub started_at_micros: i64,
    pub completed_at_micros: Option<i64>,
    pub status: String,
    pub winner: Option<String>,
}

/// Apply migration 20 idempotently. Mirrors the
/// `worker_create::ensure_*` pattern.
pub async fn ensure_ab_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in ENSURE_AB_SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ab schema: {e}")))?;
    }
    Ok(())
}

/// Insert a new `skill_ab_test_run` row. Both variants are
/// version strings (e.g. `"v2"` / `"v3"`). Returns the run id.
pub async fn create_run(
    store: &SessionStore,
    skill_name: &str,
    variant_a_version: &str,
    variant_b_version: &str,
    target_runs: i64,
) -> ArgoResult<String> {
    if variant_a_version == variant_b_version {
        return Err(ArgoError::Provider(
            "ab-test: variants A and B must differ".into(),
        ));
    }
    if find_version(store, skill_name, variant_a_version)
        .await?
        .is_none()
    {
        return Err(ArgoError::Provider(format!(
            "ab-test: skill '{skill_name}' has no version '{variant_a_version}'"
        )));
    }
    if find_version(store, skill_name, variant_b_version)
        .await?
        .is_none()
    {
        return Err(ArgoError::Provider(format!(
            "ab-test: skill '{skill_name}' has no version '{variant_b_version}'"
        )));
    }
    let id = Uuid::now_v7().to_string();
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO skill_ab_test_run \
         (id, skill_name, variant_a, variant_b, target_runs, started_at, status) \
         VALUES (?, ?, ?, ?, ?, ?, 'running')",
    )
    .bind(&id)
    .bind(skill_name)
    .bind(variant_a_version)
    .bind(variant_b_version)
    .bind(target_runs)
    .bind(now)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("create ab run: {e}")))?;
    Ok(id)
}

/// Record one invocation outcome. Idempotent on `(run_id,
/// invocation_id)` — re-recording the same invocation is a no-op
/// so callers don't have to dedupe.
pub async fn record_outcome(
    store: &SessionStore,
    run_id: &str,
    invocation_id: &str,
    variant: Variant,
    succeeded: bool,
    latency_ms: Option<i64>,
) -> ArgoResult<()> {
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT OR IGNORE INTO skill_ab_test_outcome \
         (run_id, invocation_id, variant_chosen, succeeded, latency_ms, recorded_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(run_id)
    .bind(invocation_id)
    .bind(variant.as_str())
    .bind(if succeeded { 1_i64 } else { 0 })
    .bind(latency_ms)
    .bind(now)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("record outcome: {e}")))?;
    Ok(())
}

/// Fetch a run by id. Returns `None` if missing.
pub async fn get_run(store: &SessionStore, run_id: &str) -> ArgoResult<Option<AbTestRun>> {
    let row = sqlx::query(
        "SELECT id, skill_name, variant_a, variant_b, target_runs, \
         started_at, completed_at, status, winner \
         FROM skill_ab_test_run WHERE id = ?",
    )
    .bind(run_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("get ab run: {e}")))?;
    Ok(row.map(|row| AbTestRun {
        id: row.get("id"),
        skill_name: row.get("skill_name"),
        variant_a: row.get("variant_a"),
        variant_b: row.get("variant_b"),
        target_runs: row.get("target_runs"),
        started_at_micros: row.get("started_at"),
        completed_at_micros: row.get("completed_at"),
        status: row.get("status"),
        winner: row.get("winner"),
    }))
}

/// Compute per-variant counts from the outcome table.
pub async fn variant_counts(
    store: &SessionStore,
    run_id: &str,
) -> ArgoResult<(VariantCounts, VariantCounts)> {
    let rows = sqlx::query(
        "SELECT variant_chosen, COUNT(*) AS trials, \
         SUM(succeeded) AS successes \
         FROM skill_ab_test_outcome WHERE run_id = ? GROUP BY variant_chosen",
    )
    .bind(run_id)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("variant counts: {e}")))?;
    let mut a = VariantCounts::default();
    let mut b = VariantCounts::default();
    for row in rows {
        let chosen: String = row.get("variant_chosen");
        let trials: i64 = row.get("trials");
        let successes: i64 = row.get::<Option<i64>, _>("successes").unwrap_or(0);
        match chosen.as_str() {
            "a" => a = VariantCounts { trials, successes },
            "b" => b = VariantCounts { trials, successes },
            _ => {}
        }
    }
    Ok((a, b))
}

/// If the run is significant + min-sample, flip its status to
/// `completed` and stamp the higher-rate variant as winner. Idempotent
/// — re-calling on a `completed` run is a no-op.
pub async fn maybe_complete(
    store: &SessionStore,
    run_id: &str,
    min_samples_per_variant: i64,
) -> ArgoResult<Option<Significance>> {
    let Some(run) = get_run(store, run_id).await? else {
        return Ok(None);
    };
    if run.status != "running" {
        return Ok(None);
    }
    let (a, b) = variant_counts(store, run_id).await?;
    let sig = significance(a, b, min_samples_per_variant);
    if !sig.significant {
        return Ok(Some(sig));
    }
    let rate_a = if a.trials == 0 {
        0.0
    } else {
        a.successes as f64 / a.trials as f64
    };
    let rate_b = if b.trials == 0 {
        0.0
    } else {
        b.successes as f64 / b.trials as f64
    };
    let winner = if rate_a >= rate_b {
        run.variant_a.clone()
    } else {
        run.variant_b.clone()
    };
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "UPDATE skill_ab_test_run SET status = 'completed', \
         completed_at = ?, winner = ? WHERE id = ?",
    )
    .bind(now)
    .bind(&winner)
    .bind(run_id)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("complete ab run: {e}")))?;
    Ok(Some(sig))
}

/// Apply the run's winner to the skill table — mark the loser
/// `superseded_by` the winner. Returns the winning version that
/// was applied.
pub async fn promote(
    store: &SessionStore,
    run_id: &str,
    explicit_winner: Option<&str>,
) -> ArgoResult<String> {
    let run = get_run(store, run_id).await?.ok_or_else(|| {
        ArgoError::Provider(format!("promote: no ab run with id '{run_id}'"))
    })?;
    let winner = explicit_winner
        .map(str::to_string)
        .or(run.winner.clone())
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "promote: run '{run_id}' has no winner; pass --winner <version>"
            ))
        })?;
    if winner != run.variant_a && winner != run.variant_b {
        return Err(ArgoError::Provider(format!(
            "promote: '{winner}' is not a variant of run '{run_id}'"
        )));
    }
    let loser = if winner == run.variant_a {
        run.variant_b.clone()
    } else {
        run.variant_a.clone()
    };
    let winner_id = find_version(store, &run.skill_name, &winner)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "promote: skill '{}' has no version '{winner}'",
                run.skill_name
            ))
        })?;
    sqlx::query(
        "UPDATE skill SET superseded_by = ? WHERE name = ? AND version = ?",
    )
    .bind(blob_from_hex(&winner_id))
    .bind(&run.skill_name)
    .bind(&loser)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("promote: {e}")))?;
    Ok(winner)
}

fn blob_from_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
#[path = "ab_test_tests.rs"]
mod ab_test_tests;
