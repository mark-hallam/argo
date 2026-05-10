//! Mutation surface for the team-roster primitive.
//!
//! Split out of `team_roster.rs` to stay below the 500-LOC ceiling.
//! Read-side helpers (`list_teams`, `get_team`, `compute_level`,
//! `summarise_charter`) and the `ensure_team_roster_columns` schema
//! bootstrapper remain in `team_roster.rs`. Public symbols from
//! this module are re-exported via `team_roster.rs` so external
//! callers continue to import as
//! `argo_store::team_roster::create_team` (etc.).
//!
//! All four mutators share the same persistence model documented
//! on the [`team_roster`](crate::team_roster) module — `org_unit`
//! carries the name + `parent_id` link, `roster` carries the
//! disabled-by-default policy row keyed by the same id, and
//! `roster_member` carries the soft-deletable membership rows.
//! Soft-delete semantics:
//!   - active rows have `removed_at IS NULL`
//!   - removed rows are preserved with `removed_at = <unix_ts>` so
//!     KPI / audit queries can still see historical membership.

use crate::StoreResult;
use crate::team_roster::ensure_team_roster_columns;
use argo_types::{ArgoError, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Result of [`create_team`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTeamOutcome {
    /// Newly minted team id (uuid).
    pub id: String,
    /// Echoed back from the request.
    pub name: String,
    /// Echoed back from the request, validated against `org_unit`.
    pub parent_id: Option<String>,
}

/// Create a new team in `tenant`. Inserts BOTH an `org_unit` row
/// (carrying the name + parent link) AND a `roster` row (carrying
/// a default disabled policy) keyed by the same id. If `parent_id`
/// is supplied it must already exist as an `org_unit` in this
/// tenant, otherwise the call returns a `Provider` error.
pub async fn create_team(
    pool: &SqlitePool,
    tenant: TenantId,
    name: &str,
    parent_id: Option<&str>,
) -> StoreResult<CreateTeamOutcome> {
    ensure_team_roster_columns(pool).await?;
    if name.trim().is_empty() {
        return Err(ArgoError::Provider("team name must not be empty".into()));
    }
    let tenant_str = tenant.to_string();
    if let Some(pid) = parent_id {
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM org_unit WHERE tenant_id = ? AND id = ?",
        )
        .bind(&tenant_str)
        .bind(pid)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("validate parent: {e}")))?;
        if exists.is_none() {
            return Err(ArgoError::Provider(format!(
                "parent team '{pid}' not found in tenant '{tenant}'"
            )));
        }
    }
    let id = Uuid::now_v7().to_string();
    let now = Utc::now().timestamp();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("create_team tx: {e}")))?;
    sqlx::query(
        "INSERT INTO org_unit (id, tenant_id, name, parent_id, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&tenant_str)
    .bind(name)
    .bind(parent_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("create_team org_unit: {e}")))?;
    sqlx::query(
        "INSERT INTO roster (id, tenant_id, name, policy_json, channels_json, \
                              budget_cap_usd_micros, created_at) \
         VALUES (?, ?, ?, '{\"enabled\":false}', '[]', NULL, ?)",
    )
    .bind(&id)
    .bind(&tenant_str)
    .bind(name)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("create_team roster: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("create_team commit: {e}")))?;
    Ok(CreateTeamOutcome {
        id,
        name: name.to_string(),
        parent_id: parent_id.map(str::to_string),
    })
}

/// Rename a team. Updates BOTH `org_unit.name` and `roster.name`
/// to keep the two tables in sync. Returns `false` if the team
/// did not exist in the tenant.
pub async fn rename_team(
    pool: &SqlitePool,
    tenant: TenantId,
    team_id: &str,
    new_name: &str,
) -> StoreResult<bool> {
    if new_name.trim().is_empty() {
        return Err(ArgoError::Provider(
            "team name must not be empty".into(),
        ));
    }
    let tenant_str = tenant.to_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("rename_team tx: {e}")))?;
    let unit_rows = sqlx::query(
        "UPDATE org_unit SET name = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(new_name)
    .bind(&tenant_str)
    .bind(team_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rename_team org_unit: {e}")))?
    .rows_affected();
    if unit_rows == 0 {
        tx.rollback()
            .await
            .map_err(|e| ArgoError::Provider(format!("rename_team rollback: {e}")))?;
        return Ok(false);
    }
    sqlx::query("UPDATE roster SET name = ? WHERE tenant_id = ? AND id = ?")
        .bind(new_name)
        .bind(&tenant_str)
        .bind(team_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("rename_team roster: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("rename_team commit: {e}")))?;
    Ok(true)
}

/// Add `worker_id` to `team_id`. Idempotent: re-adding an already
/// active member updates only the role. Re-adding a previously
/// soft-deleted member clears `removed_at` and refreshes
/// `added_at`.
pub async fn add_team_member(
    pool: &SqlitePool,
    team_id: &str,
    worker_id: &str,
    role: Option<&str>,
) -> StoreResult<()> {
    ensure_team_roster_columns(pool).await?;
    let now = Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO roster_member (roster_id, worker_id, added_at, role, removed_at) \
         VALUES (?, ?, ?, ?, NULL) \
         ON CONFLICT(roster_id, worker_id) DO UPDATE SET \
             role = excluded.role, \
             added_at = CASE \
                 WHEN roster_member.removed_at IS NOT NULL THEN excluded.added_at \
                 ELSE roster_member.added_at \
             END, \
             removed_at = NULL",
    )
    .bind(team_id)
    .bind(worker_id)
    .bind(now)
    .bind(role)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("add_team_member: {e}")))?;
    Ok(())
}

/// Soft-delete an active membership. Sets `removed_at` to `now`.
/// Returns `true` if a row was updated, `false` if the worker was
/// already inactive or never a member.
pub async fn remove_team_member(
    pool: &SqlitePool,
    team_id: &str,
    worker_id: &str,
) -> StoreResult<bool> {
    ensure_team_roster_columns(pool).await?;
    let now = Utc::now().timestamp();
    let rows = sqlx::query(
        "UPDATE roster_member SET removed_at = ? \
         WHERE roster_id = ? AND worker_id = ? AND removed_at IS NULL",
    )
    .bind(now)
    .bind(team_id)
    .bind(worker_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("remove_team_member: {e}")))?
    .rows_affected();
    Ok(rows > 0)
}
