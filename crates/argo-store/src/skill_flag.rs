//! Per-skill operator flag store (Stream
//! `mission-forge-operator-feedback-eval-signal`, MISSION_BACKLOG
//! Section 3 Claim 4 #85).
//!
//! Generic schema — one row per `(skill_id, flag)` pair, value
//! stored as INTEGER. The Forge ingestion path consults
//! [`get_flag_or_default`] for `no-forge-feedback` before pulling
//! operator feedback as eval cases; future per-skill flags
//! piggyback on the same table without further migrations.
//!
//! `skill_id` is the skill name string for symmetry with the
//! existing `skill_quality_score` / `skill_canary` tables and the
//! operator-facing `argo skill set-flag <name>` CLI surface
//! (operators type a name, not a binary id).
use crate::StoreResult;
use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

/// Sentinel flag name — when `value = 1`, the Forge ingestion path
/// skips this skill entirely. See
/// `argo-forge::feedback_eval::ingest_operator_feedback`.
pub const FLAG_NO_FORGE_FEEDBACK: &str = "no-forge-feedback";

/// Upsert one `(skill_id, flag)` row.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
/// argo_store::skill_flag::set_flag(pool, "billing-resolver", "no-forge-feedback", 1).await?;
/// # Ok(()) }
/// ```
pub async fn set_flag(
    pool: &Pool<Sqlite>,
    skill_id: &str,
    flag: &str,
    value: i64,
) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO skill_flag (skill_id, flag, value) VALUES (?, ?, ?) \
         ON CONFLICT (skill_id, flag) DO UPDATE SET value = excluded.value",
    )
    .bind(skill_id)
    .bind(flag)
    .bind(value)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_flag set_flag: {e}")))?;
    Ok(())
}

/// Read one `(skill_id, flag)` value if present.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
/// let v = argo_store::skill_flag::get_flag(pool, "billing", "no-forge-feedback").await?;
/// assert!(v.is_none() || matches!(v, Some(0) | Some(1)));
/// # Ok(()) }
/// ```
pub async fn get_flag(
    pool: &Pool<Sqlite>,
    skill_id: &str,
    flag: &str,
) -> StoreResult<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT value FROM skill_flag WHERE skill_id = ? AND flag = ?",
    )
    .bind(skill_id)
    .bind(flag)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_flag get_flag: {e}")))?;
    Ok(row.map(|(v,)| v))
}

/// Read one `(skill_id, flag)` value, falling back to `default`
/// when the row is absent. Useful for boolean flags where "missing
/// row" is equivalent to "feature on" (default = 1) or "feature
/// off" (default = 0).
///
/// # Examples
///
/// ```no_run
/// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
/// let on = argo_store::skill_flag::get_flag_or_default(
///     pool, "billing", "no-forge-feedback", 0,
/// ).await? == 1;
/// # let _ = on; Ok(()) }
/// ```
pub async fn get_flag_or_default(
    pool: &Pool<Sqlite>,
    skill_id: &str,
    flag: &str,
    default: i64,
) -> StoreResult<i64> {
    Ok(get_flag(pool, skill_id, flag).await?.unwrap_or(default))
}

#[cfg(test)]
#[path = "skill_flag_tests.rs"]
mod tests;
