//! Operator-facing team primitive
//! (Stream `claude-worker-team-roster-cli`, V1.1).
//!
//! Bridges three existing tables into a single "team" abstraction
//! that the `argo team` CLI exposes:
//!
//! * `org_unit` (migration 21 / `src/migrations/v22.rs`) — the
//!   tree of organisational units. Carries the team's name and
//!   `parent_id` link.
//! * `roster` (migration 22 / `src/migrations/v23.rs`) — the
//!   shared policy / channels / budget cap row. A team's row is
//!   keyed by the SAME id as its `org_unit` row, so a single
//!   `team_id` resolves uniquely in both tables.
//! * `roster_member` (same migration) — many-to-many between
//!   workers and teams. This module relies on two additive
//!   columns introduced by `migrations/41__roster_role_removed_at.sql`:
//!     - `role` TEXT — operator-supplied label per (team, worker)
//!     - `removed_at` INTEGER — soft-delete timestamp; non-null
//!       rows are excluded from active counts but preserved for
//!       audit / KPI history.
//!
//! Members are stored under `roster_member` (NOT `org_member`).
//! `org_member` exists for the addressable-name + manager-chain
//! surface served by `argo org`, which has different lifecycle
//! semantics. Mixing the two would entangle CLI surfaces.

use crate::SessionStore;
use crate::StoreResult;
use argo_types::{ArgoError, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

pub use crate::team_roster_mutations::{
    CreateTeamOutcome, add_team_member, create_team, remove_team_member, rename_team,
};

/// Embedded SQL doc — the actual schema additions are applied via
/// the PRAGMA-detect-then-ALTER pattern in
/// [`ensure_team_roster_columns`] so the same call is idempotent
/// on databases that already migrated.
pub const ROSTER_ROLE_REMOVED_AT_SCHEMA_DOC: &str =
    include_str!("../migrations/41__roster_role_removed_at.sql");

/// One row in the team list — `org_unit` joined with the active
/// `roster_member` count for the same id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamRow {
    /// Stable team id (uuid). Identifies the same row in both
    /// `org_unit` and `roster`.
    pub id: String,
    /// Operator-friendly team name.
    pub name: String,
    /// Parent team id, or `None` if this is a root team.
    pub parent_id: Option<String>,
    /// Number of active members (rows in `roster_member` with
    /// `removed_at IS NULL`).
    pub member_count: i64,
}

/// Active member of a team.
///
/// Charter hydration is deliberately NOT done here — the CLI
/// handler stitches it on via [`argo_identity::IdentityStore`]
/// because the `worker.charter_md` column is added lazily by
/// that crate's `ensure_worker_charter_columns` path, not by
/// the structured migration runner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamMemberRow {
    /// Worker id (uuid as string).
    pub worker_id: String,
    /// Operator-supplied role label, if any.
    pub role: Option<String>,
    /// Seconds-since-epoch when the worker joined the team.
    pub added_at: i64,
}

/// Detailed view of a single team.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamDetail {
    /// Stable team id.
    pub id: String,
    /// Operator-friendly team name.
    pub name: String,
    /// Parent team id, or `None` if this is a root team.
    pub parent_id: Option<String>,
    /// Depth in the org tree (0 = root team, 1 = direct child of
    /// a root, etc.). Computed by walking `parent_id` to the root.
    pub level: u32,
    /// Active members (rows with `removed_at IS NULL`).
    pub members: Vec<TeamMemberRow>,
}

/// Idempotently apply the additive `role` and `removed_at`
/// columns plus the active-member partial index.
///
/// Mirrors `ensure_session_worker_id_column` in `lib.rs` —
/// PRAGMA-detect each column and ALTER TABLE if missing. SQLite
/// does not support `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`,
/// so the explicit detect-then-alter form is the canonical
/// pattern in this crate.
pub async fn ensure_team_roster_columns(pool: &SqlitePool) -> StoreResult<()> {
    let cols = existing_columns(pool, "roster_member").await?;
    if !cols.contains("role") {
        sqlx::query("ALTER TABLE roster_member ADD COLUMN role TEXT")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add roster_member.role: {e}")))?;
    }
    if !cols.contains("removed_at") {
        sqlx::query("ALTER TABLE roster_member ADD COLUMN removed_at INTEGER")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add roster_member.removed_at: {e}")))?;
    }
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_roster_member_active \
         ON roster_member(roster_id) WHERE removed_at IS NULL",
    )
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Io(format!("create idx_roster_member_active: {e}")))?;
    Ok(())
}

async fn existing_columns(pool: &SqlitePool, table: &str) -> StoreResult<HashSet<String>> {
    let sql = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("table_info {table}: {e}")))?;
    Ok(rows
        .iter()
        .map(|r| {
            let name: String = r.get("name");
            name
        })
        .collect())
}

/// List all teams in `tenant`, ordered by name. Each row carries
/// the active member count.
pub async fn list_teams(pool: &SqlitePool, tenant: TenantId) -> StoreResult<Vec<TeamRow>> {
    ensure_team_roster_columns(pool).await?;
    let tenant_str = tenant.to_string();
    let rows = sqlx::query(
        "SELECT u.id, u.name, u.parent_id, \
                COALESCE(( \
                    SELECT COUNT(*) FROM roster_member m \
                    WHERE m.roster_id = u.id AND m.removed_at IS NULL \
                ), 0) AS member_count \
         FROM org_unit u \
         WHERE u.tenant_id = ? \
         ORDER BY u.name",
    )
    .bind(&tenant_str)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("list_teams: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|r| TeamRow {
            id: r.get("id"),
            name: r.get("name"),
            parent_id: r.get("parent_id"),
            member_count: r.get("member_count"),
        })
        .collect())
}

/// Look up a single team in `tenant` and hydrate its active
/// members. Returns `None` if no `org_unit` row matches.
pub async fn get_team(
    pool: &SqlitePool,
    tenant: TenantId,
    team_id: &str,
) -> StoreResult<Option<TeamDetail>> {
    ensure_team_roster_columns(pool).await?;
    let tenant_str = tenant.to_string();
    let unit = sqlx::query(
        "SELECT id, name, parent_id FROM org_unit \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(&tenant_str)
    .bind(team_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("get_team unit: {e}")))?;
    let Some(unit) = unit else {
        return Ok(None);
    };
    let id: String = unit.get("id");
    let name: String = unit.get("name");
    let parent_id: Option<String> = unit.get("parent_id");
    let level = compute_level(pool, &tenant_str, parent_id.as_deref()).await?;
    let member_rows = sqlx::query(
        "SELECT m.worker_id, m.role, m.added_at \
         FROM roster_member m \
         WHERE m.roster_id = ? AND m.removed_at IS NULL \
         ORDER BY m.added_at, m.worker_id",
    )
    .bind(&id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("get_team members: {e}")))?;
    let members = member_rows
        .into_iter()
        .map(|r| TeamMemberRow {
            worker_id: r.get("worker_id"),
            role: r.get("role"),
            added_at: r.get("added_at"),
        })
        .collect();
    Ok(Some(TeamDetail {
        id,
        name,
        parent_id,
        level,
        members,
    }))
}

/// Walk the parent chain to compute tree depth. Cycles short-
/// circuit at 64 hops to keep a buggy tree from looping forever.
async fn compute_level(
    pool: &SqlitePool,
    tenant: &str,
    parent: Option<&str>,
) -> StoreResult<u32> {
    let mut level = 0_u32;
    let mut current: Option<String> = parent.map(str::to_string);
    while let Some(pid) = current.take() {
        if level >= 64 {
            break;
        }
        level += 1;
        current = sqlx::query_scalar(
            "SELECT parent_id FROM org_unit WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant)
        .bind(&pid)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("compute_level: {e}")))?
        .flatten();
    }
    Ok(level)
}

/// Summarise a worker's charter body for display in `team show`.
/// First 80 chars (Unicode-aware), trimmed; longer bodies are
/// ellipsised with `…`. The CLI handler is the canonical caller
/// after hydrating the worker row through `argo_identity`.
#[must_use]
pub fn summarise_charter(body: &str) -> String {
    const MAX: usize = 80;
    let trimmed = body.trim();
    if trimmed.chars().count() <= MAX {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(MAX - 1).collect();
    out.push('…');
    out
}

/// Direct accessor on [`SessionStore`] — convenience wrapper that
/// passes the underlying pool to [`ensure_team_roster_columns`].
impl SessionStore {
    /// Apply the team-roster column additions on demand.
    pub async fn ensure_team_roster_schema(&self) -> StoreResult<()> {
        ensure_team_roster_columns(&self.pool).await
    }
}

#[cfg(test)]
#[path = "team_roster_tests.rs"]
mod tests;
