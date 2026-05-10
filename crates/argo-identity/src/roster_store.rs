//! `RosterStore` persistence — sibling of `roster.rs`; re-exported there so the public path is unchanged.

use crate::roster::{Roster, RosterId};
use crate::types::{AutoApprovalPolicy, NotifyChannelRef, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

const ENSURE_ROSTER_SCHEMA_SQL: &str = include_str!("../../argo-store/migrations/22__roster.sql");

/// Persistence facade over the `roster` + `roster_member` tables.
#[derive(Clone)]
pub struct RosterStore {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for RosterStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RosterStore").finish()
    }
}

impl RosterStore {
    /// Wrap a [`SessionStore`] for roster CRUD.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Apply migration 22 in-place. Idempotent.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in ENSURE_ROSTER_SCHEMA_SQL.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("ensure roster schema: {e}")))?;
        }
        Ok(())
    }

    /// Insert a new roster row. Members are persisted via [`RosterStore::add_member`].
    pub async fn create(&self, roster: &Roster) -> ArgoResult<()> {
        let policy_json = serde_json::to_string(&roster.policy)
            .map_err(|e| ArgoError::Provider(format!("serialize roster policy: {e}")))?;
        let channels_json = serde_json::to_string(&roster.channels)
            .map_err(|e| ArgoError::Provider(format!("serialize roster channels: {e}")))?;
        let budget_micros = roster.budget_cap_usd.map(|cap| (cap * 1_000_000.0).round() as i64);
        let now_micros = Utc::now().timestamp_micros();
        sqlx::query(
            "INSERT INTO roster (id, tenant_id, name, policy_json, \
             channels_json, budget_cap_usd_micros, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(roster.id.to_string())
        .bind(roster.tenant_id.to_string())
        .bind(&roster.name)
        .bind(policy_json)
        .bind(channels_json)
        .bind(budget_micros)
        .bind(now_micros)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert roster: {e}")))?;
        Ok(())
    }

    /// Look up a roster by id (with members hydrated).
    pub async fn get(&self, tenant: TenantId, id: RosterId) -> ArgoResult<Option<Roster>> {
        let row = sqlx::query(
            "SELECT id, tenant_id, name, policy_json, channels_json, \
             budget_cap_usd_micros FROM roster \
             WHERE id = ? AND tenant_id = ?",
        )
        .bind(id.to_string())
        .bind(tenant.to_string())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("get roster: {e}")))?;
        let Some(row) = row else { return Ok(None) };
        let mut roster = hydrate_row(&row)?;
        roster.member_worker_ids = self.list_members(roster.id).await?;
        Ok(Some(roster))
    }

    /// Resolve a roster by tenant + name.
    pub async fn get_by_name(&self, tenant: TenantId, name: &str) -> ArgoResult<Option<Roster>> {
        let row = sqlx::query(
            "SELECT id, tenant_id, name, policy_json, channels_json, \
             budget_cap_usd_micros FROM roster \
             WHERE tenant_id = ? AND name = ?",
        )
        .bind(tenant.to_string())
        .bind(name)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("get roster by name: {e}")))?;
        let Some(row) = row else { return Ok(None) };
        let mut roster = hydrate_row(&row)?;
        roster.member_worker_ids = self.list_members(roster.id).await?;
        Ok(Some(roster))
    }

    /// List rosters for a tenant ordered by name (members hydrated).
    pub async fn list(&self, tenant: TenantId) -> ArgoResult<Vec<Roster>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, policy_json, channels_json, \
             budget_cap_usd_micros FROM roster \
             WHERE tenant_id = ? ORDER BY name",
        )
        .bind(tenant.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list rosters: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut roster = hydrate_row(&row)?;
            roster.member_worker_ids = self.list_members(roster.id).await?;
            out.push(roster);
        }
        Ok(out)
    }

    /// Delete a roster + all its membership rows.
    pub async fn delete(&self, tenant: TenantId, id: RosterId) -> ArgoResult<()> {
        sqlx::query("DELETE FROM roster_member WHERE roster_id = ?")
            .bind(id.to_string())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("delete roster members: {e}")))?;
        sqlx::query("DELETE FROM roster WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant.to_string())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("delete roster: {e}")))?;
        Ok(())
    }

    /// Add a worker to a roster. Idempotent (PK collision tolerated).
    pub async fn add_member(&self, roster_id: RosterId, worker_id: WorkerId) -> ArgoResult<()> {
        let now_micros = Utc::now().timestamp_micros();
        sqlx::query(
            "INSERT OR IGNORE INTO roster_member \
             (roster_id, worker_id, added_at) VALUES (?, ?, ?)",
        )
        .bind(roster_id.to_string())
        .bind(worker_id.to_string())
        .bind(now_micros)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("add roster member: {e}")))?;
        Ok(())
    }

    /// Drop a worker from a roster.
    pub async fn remove_member(&self, roster_id: RosterId, worker_id: WorkerId) -> ArgoResult<()> {
        sqlx::query("DELETE FROM roster_member WHERE roster_id = ? AND worker_id = ?")
            .bind(roster_id.to_string())
            .bind(worker_id.to_string())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("remove roster member: {e}")))?;
        Ok(())
    }

    /// List worker_ids that belong to a given roster.
    pub async fn list_members(&self, roster_id: RosterId) -> ArgoResult<Vec<WorkerId>> {
        let rows = sqlx::query(
            "SELECT worker_id FROM roster_member \
             WHERE roster_id = ? ORDER BY added_at",
        )
        .bind(roster_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list roster members: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let raw: String = row.get("worker_id");
            let id: WorkerId = raw.parse().map_err(|e| {
                ArgoError::Provider(format!("decode worker id '{raw}': {e}"))
            })?;
            out.push(id);
        }
        Ok(out)
    }

    /// Find every roster a given worker is a member of.
    pub async fn rosters_for_worker(
        &self,
        tenant: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<Vec<Roster>> {
        let rows = sqlx::query(
            "SELECT r.id, r.tenant_id, r.name, r.policy_json, r.channels_json, \
             r.budget_cap_usd_micros FROM roster r \
             JOIN roster_member m ON r.id = m.roster_id \
             WHERE m.worker_id = ? AND r.tenant_id = ? \
             ORDER BY r.name",
        )
        .bind(worker_id.to_string())
        .bind(tenant.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("rosters for worker: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut roster = hydrate_row(&row)?;
            roster.member_worker_ids = self.list_members(roster.id).await?;
            out.push(roster);
        }
        Ok(out)
    }
}

fn hydrate_row(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<Roster> {
    let id_str: String = row.get("id");
    let tenant_str: String = row.get("tenant_id");
    let name: String = row.get("name");
    let policy_json: String = row.get("policy_json");
    let channels_json: String = row.get("channels_json");
    let budget_micros: Option<i64> = row.get("budget_cap_usd_micros");
    let id: RosterId = id_str
        .parse()
        .map_err(|e| ArgoError::Provider(format!("decode roster id '{id_str}': {e}")))?;
    let tenant_id: TenantId = tenant_str
        .parse::<Uuid>()
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("decode tenant id '{tenant_str}': {e}")))?;
    let policy: AutoApprovalPolicy = serde_json::from_str(&policy_json)
        .map_err(|e| ArgoError::Provider(format!("decode roster policy: {e}")))?;
    let channels: Vec<NotifyChannelRef> = serde_json::from_str(&channels_json)
        .map_err(|e| ArgoError::Provider(format!("decode roster channels: {e}")))?;
    Ok(Roster {
        id,
        tenant_id,
        name,
        member_worker_ids: Vec::new(),
        policy,
        channels,
        budget_cap_usd: budget_micros.map(|m| (m as f64) / 1_000_000.0),
    })
}
