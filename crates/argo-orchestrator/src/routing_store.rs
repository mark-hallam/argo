//! Persistence facade for [`crate::routing`] rules.
//!
//! Split out of `routing.rs` to keep that file under the 500-LOC
//! ceiling. The store is re-exported from `routing` so callers
//! continue to use the `argo_orchestrator::RoutingRuleStore` path.

use crate::routing::{HandoffRoutingRule, RoutingEngine, RuleId, TargetSelector, WhenClause};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

const ENSURE_ROUTING_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/23__handoff_routing.sql");

/// Persistence facade over the `handoff_routing_rule` table.
#[derive(Clone)]
pub struct RoutingRuleStore {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for RoutingRuleStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingRuleStore").finish()
    }
}

impl RoutingRuleStore {
    /// Wrap a [`SessionStore`] for rule CRUD.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Apply migration 23 in-place. Idempotent.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in ENSURE_ROUTING_SCHEMA_SQL.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("ensure routing schema: {e}")))?;
        }
        Ok(())
    }

    /// Insert a new rule row. Caller assigns the [`RuleId`] (via
    /// `HandoffRoutingRule::new`); duplicate names per tenant are
    /// rejected by the UNIQUE constraint.
    pub async fn create(
        &self,
        tenant: TenantId,
        rule: &HandoffRoutingRule,
    ) -> ArgoResult<()> {
        let condition = serde_json::to_string(&rule.when)
            .map_err(|e| ArgoError::Provider(format!("serialize rule condition: {e}")))?;
        let target = serde_json::to_string(&rule.target)
            .map_err(|e| ArgoError::Provider(format!("serialize rule target: {e}")))?;
        let now_micros = Utc::now().timestamp_micros();
        sqlx::query(
            "INSERT INTO handoff_routing_rule \
             (id, tenant_id, name, condition_json, target_json, priority, active, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rule.id.to_string())
        .bind(tenant.to_string())
        .bind(&rule.name)
        .bind(condition)
        .bind(target)
        .bind(i64::from(rule.priority))
        .bind(i32::from(rule.active))
        .bind(now_micros)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert routing rule: {e}")))?;
        Ok(())
    }

    /// Fetch a rule by id.
    pub async fn get(
        &self,
        tenant: TenantId,
        id: RuleId,
    ) -> ArgoResult<Option<HandoffRoutingRule>> {
        let row = sqlx::query(
            "SELECT id, name, condition_json, target_json, priority, active \
             FROM handoff_routing_rule WHERE id = ? AND tenant_id = ?",
        )
        .bind(id.to_string())
        .bind(tenant.to_string())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("get routing rule: {e}")))?;
        row.map(rule_from_row).transpose()
    }

    /// Fetch a rule by tenant + name.
    pub async fn get_by_name(
        &self,
        tenant: TenantId,
        name: &str,
    ) -> ArgoResult<Option<HandoffRoutingRule>> {
        let row = sqlx::query(
            "SELECT id, name, condition_json, target_json, priority, active \
             FROM handoff_routing_rule WHERE tenant_id = ? AND name = ?",
        )
        .bind(tenant.to_string())
        .bind(name)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("get routing rule by name: {e}")))?;
        row.map(rule_from_row).transpose()
    }

    /// List all rules for a tenant in priority order.
    pub async fn list(&self, tenant: TenantId) -> ArgoResult<Vec<HandoffRoutingRule>> {
        let rows = sqlx::query(
            "SELECT id, name, condition_json, target_json, priority, active \
             FROM handoff_routing_rule WHERE tenant_id = ? \
             ORDER BY priority ASC, created_at ASC",
        )
        .bind(tenant.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list routing rules: {e}")))?;
        rows.into_iter().map(rule_from_row).collect()
    }

    /// Hard-delete a rule. Caller is responsible for confirming.
    pub async fn delete(&self, tenant: TenantId, id: RuleId) -> ArgoResult<()> {
        sqlx::query(
            "DELETE FROM handoff_routing_rule WHERE id = ? AND tenant_id = ?",
        )
        .bind(id.to_string())
        .bind(tenant.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("delete routing rule: {e}")))?;
        Ok(())
    }

    /// Build a [`RoutingEngine`] from every active rule for a
    /// tenant. Inactive rules are skipped at engine construction
    /// time so [`RoutingEngine::route`] doesn't pay the filter
    /// cost on the hot path.
    pub async fn engine_for_tenant(&self, tenant: TenantId) -> ArgoResult<RoutingEngine> {
        let rules = self
            .list(tenant)
            .await?
            .into_iter()
            .filter(|r| r.active)
            .collect();
        Ok(RoutingEngine::new(rules))
    }
}

fn rule_from_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<HandoffRoutingRule> {
    let id_str: String = row.get("id");
    let name: String = row.get("name");
    let condition_json: String = row.get("condition_json");
    let target_json: String = row.get("target_json");
    let priority: i64 = row.get("priority");
    let active: i32 = row.get("active");
    let id: RuleId = id_str
        .parse()
        .map_err(|e| ArgoError::Invariant(format!("decode rule id '{id_str}': {e}")))?;
    let when: WhenClause = serde_json::from_str(&condition_json)
        .map_err(|e| ArgoError::Invariant(format!("decode rule condition: {e}")))?;
    let target: TargetSelector = serde_json::from_str(&target_json)
        .map_err(|e| ArgoError::Invariant(format!("decode rule target: {e}")))?;
    Ok(HandoffRoutingRule {
        id,
        name,
        when,
        target,
        priority: u32::try_from(priority).unwrap_or(u32::MAX),
        active: active != 0,
    })
}
