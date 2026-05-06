//! V1.3 — Worker rosters: group-level policy + channels + budget.
//!
//! A [`Roster`] is a named group of workers that share an
//! [`AutoApprovalPolicy`], a list of [`NotifyChannelRef`]s, and an
//! optional spend cap. Per-worker policy resolution
//! ([`crate::Worker::resolved_policy_with_rosters`]) consults the
//! rosters a worker belongs to whenever the worker's own
//! `auto_approval` is disabled — workspace defaults are the final
//! fallback below the roster layer.
//!
//! Multi-tenant: every primitive carries `tenant_id`. Two tenants
//! can have completely separate rosters in the same DB without
//! leakage.
//!
//! Conflict resolution (when a worker belongs to multiple rosters)
//! is "most restrictive wins" — see
//! [`AutoApprovalPolicy::most_restrictive`].

use crate::types::{AutoApprovalPolicy, NotifyChannelRef, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

const ENSURE_ROSTER_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/22__roster.sql");

/// Stable roster id (UUIDv7).
///
/// ```
/// use argo_identity::roster::RosterId;
/// let id = RosterId::new();
/// assert_eq!(id.0.get_version_num(), 7);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RosterId(pub Uuid);

impl RosterId {
    /// Generate a fresh id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RosterId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RosterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::str::FromStr for RosterId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Group of workers that share auto-approval policy + notification
/// channels + an optional spend cap.
///
/// ```
/// use argo_identity::roster::Roster;
/// use argo_identity::AutoApprovalPolicy;
/// use argo_types::TenantId;
///
/// let roster = Roster::new(
///     TenantId::new(),
///     "sales-ops",
///     AutoApprovalPolicy::default(),
///     Vec::new(),
///     Some(50.0),
/// );
/// assert_eq!(roster.name, "sales-ops");
/// assert_eq!(roster.budget_cap_usd, Some(50.0));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roster {
    /// Stable roster id.
    pub id: RosterId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Operator-friendly group name (unique per tenant).
    pub name: String,
    /// Workers that belong to this roster.
    pub member_worker_ids: Vec<WorkerId>,
    /// Shared auto-approval policy applied when a member's own
    /// `auto_approval` is disabled.
    pub policy: AutoApprovalPolicy,
    /// Notification channel preferences for the group. Member
    /// workers without explicit channels fall back to these.
    pub channels: Vec<NotifyChannelRef>,
    /// Optional group-level spend ceiling, expressed in whole USD
    /// (operator-facing units). Independent of the per-action /
    /// per-day micros caps inside `policy`.
    pub budget_cap_usd: Option<f64>,
}

impl Roster {
    /// Build a fresh roster with no members. Members are added via
    /// [`RosterStore::add_member`] after persistence.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        name: impl Into<String>,
        policy: AutoApprovalPolicy,
        channels: Vec<NotifyChannelRef>,
        budget_cap_usd: Option<f64>,
    ) -> Self {
        Self {
            id: RosterId::new(),
            tenant_id,
            name: name.into(),
            member_worker_ids: Vec::new(),
            policy,
            channels,
            budget_cap_usd,
        }
    }
}

impl AutoApprovalPolicy {
    /// Merge a slice of policies into the most-restrictive
    /// resolved policy.
    ///
    /// "Most restrictive" semantics:
    /// * `enabled` — the merged policy is enabled iff at least one
    ///   input policy is enabled. Callers normally pre-filter to
    ///   enabled inputs (the resolver does); the safe fallback
    ///   when the slice is empty is the default disabled policy.
    /// * `max_usd_micros_per_action` — minimum of all set values
    ///   (any unset = no constraint, so it's skipped).
    /// * `daily_budget_usd_micros` — minimum of all set values.
    /// * `time_windows` — keep the shortest non-empty list (fewest
    ///   allowances = strictest). If every input has an empty list
    ///   the merged list is empty (no time-based restriction).
    /// * `wake_action_types` — intersection (a type is wake-allowed
    ///   only if every input policy lists it).
    ///
    /// ```
    /// use argo_identity::AutoApprovalPolicy;
    /// let a = AutoApprovalPolicy {
    ///     enabled: true,
    ///     daily_budget_usd_micros: Some(10_000),
    ///     ..Default::default()
    /// };
    /// let b = AutoApprovalPolicy {
    ///     enabled: true,
    ///     daily_budget_usd_micros: Some(5_000),
    ///     ..Default::default()
    /// };
    /// let merged = AutoApprovalPolicy::most_restrictive(&[&a, &b]);
    /// assert_eq!(merged.daily_budget_usd_micros, Some(5_000));
    /// ```
    #[must_use]
    pub fn most_restrictive(policies: &[&AutoApprovalPolicy]) -> AutoApprovalPolicy {
        if policies.is_empty() {
            return AutoApprovalPolicy::default();
        }
        let enabled = policies.iter().any(|p| p.enabled);
        let max_per_action = policies
            .iter()
            .filter_map(|p| p.max_usd_micros_per_action)
            .min();
        let daily_budget = policies
            .iter()
            .filter_map(|p| p.daily_budget_usd_micros)
            .min();
        let time_windows = policies
            .iter()
            .map(|p| p.time_windows.clone())
            .filter(|w| !w.is_empty())
            .min_by_key(std::vec::Vec::len)
            .unwrap_or_default();
        let wake_action_types = intersection_of_string_lists(
            &policies
                .iter()
                .map(|p| p.wake_action_types.as_slice())
                .collect::<Vec<&[String]>>(),
        );
        AutoApprovalPolicy {
            enabled,
            max_usd_micros_per_action: max_per_action,
            daily_budget_usd_micros: daily_budget,
            time_windows,
            wake_action_types,
        }
    }
}

fn intersection_of_string_lists(lists: &[&[String]]) -> Vec<String> {
    let Some((head, tail)) = lists.split_first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in *head {
        if tail.iter().all(|other| other.contains(item)) {
            out.push(item.clone());
        }
    }
    out
}

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

    /// Insert a new roster row. Members aren't persisted by this
    /// call — caller follows up with [`RosterStore::add_member`].
    pub async fn create(&self, roster: &Roster) -> ArgoResult<()> {
        let policy_json = serde_json::to_string(&roster.policy)
            .map_err(|e| ArgoError::Provider(format!("serialize roster policy: {e}")))?;
        let channels_json = serde_json::to_string(&roster.channels)
            .map_err(|e| ArgoError::Provider(format!("serialize roster channels: {e}")))?;
        let budget_micros = roster
            .budget_cap_usd
            .map(|cap| (cap * 1_000_000.0).round() as i64);
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
    pub async fn get_by_name(
        &self,
        tenant: TenantId,
        name: &str,
    ) -> ArgoResult<Option<Roster>> {
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
    pub async fn remove_member(
        &self,
        roster_id: RosterId,
        worker_id: WorkerId,
    ) -> ArgoResult<()> {
        sqlx::query(
            "DELETE FROM roster_member WHERE roster_id = ? AND worker_id = ?",
        )
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

#[cfg(test)]
#[path = "roster_tests.rs"]
mod tests;
