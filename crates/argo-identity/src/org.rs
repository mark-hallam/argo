//! V1.3 — Org chart + addressable names + manager chain.
//!
//! Workers should know the team. v1 ships Worker / Persona / Role
//! identity primitives; this module layers the *organisational*
//! shape on top: who reports to whom, what units exist, and how
//! teammates are addressed by the names humans actually use
//! ("Maria", "M.H.", "@maria" → canonical `Maria Hernandez`).
//!
//! ## Lifecycle
//!
//! 1. Operator drops a YAML file with the org tree (units +
//!    members + aliases) into the workspace and runs
//!    `argo org ingest <yaml>`. The YAML schema lives in
//!    `crate::org_ingest::OrgChartYaml`.
//! 2. The ingest call hits [`OrgStore::ingest`], which splatters
//!    the tree across four tables (see migration 21):
//!    `org_unit`, `org_member`, `org_member_alias`,
//!    `org_reports_to`.
//! 3. From there the operator runs `argo org tree`, `argo org
//!    member <alias>`, `argo org reports-to <member>` to inspect.
//! 4. Future briefs hook this into the escalation walker (the
//!    walker prefers `manager_id` chain when an `org_member`
//!    row exists for the worker).
//!
//! Multi-tenant: every primitive carries `tenant_id`. Two tenants
//! can have completely separate org charts in the same DB without
//! leakage.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::Row;

const ENSURE_ORG_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/21__org_chart.sql");

/// What kind of teammate this row represents. The audit ledger
/// uses the variant to decide whether to render the row as a
/// person ("Maria Hernandez") or a worker ("Skater bot").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    /// Human teammate. Reachable via channel addresses, not
    /// directly callable by `argo run`.
    Human,
    /// Worker (AI) teammate. Backed by an existing `worker.id`
    /// row in the identity tables; the worker handles its own
    /// actions through the v3 ledger.
    Worker,
}

impl MemberKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Worker => "worker",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "human" => Some(Self::Human),
            "worker" => Some(Self::Worker),
            _ => None,
        }
    }
}

/// One organisational unit. A `parent_id` of `None` is a root
/// (the org itself); deeper rows nest under their parent.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgUnit {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub parent_id: Option<String>,
}

/// One teammate. The `manager_id` chain walks toward the org
/// root (a member with no manager is at the top of their chain).
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgMember {
    pub id: String,
    pub tenant_id: String,
    pub kind: MemberKind,
    pub canonical: String,
    pub unit_id: Option<String>,
    pub manager_id: Option<String>,
    pub worker_id: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Async wrapper over the org-chart tables. Construct one per
/// workspace SessionStore and re-use; every method is `&self` so
/// it composes cleanly with the rest of the cli handlers.
pub struct OrgStore {
    store: std::sync::Arc<SessionStore>,
}

impl OrgStore {
    /// Build over an already-open SessionStore.
    #[must_use]
    pub fn new(store: std::sync::Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Borrow the underlying SQLite pool. Used by the sibling
    /// `org_ingest` module's `impl OrgStore { ... }` block.
    pub(crate) fn pool(&self) -> &sqlx::Pool<sqlx::Sqlite> {
        self.store.pool()
    }

    /// Lazy-apply migration 21. Idempotent — safe to call on every
    /// CLI invocation.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in ENSURE_ORG_SCHEMA_SQL.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("org schema: {e}")))?;
        }
        Ok(())
    }

    /// List units for a tenant in display order.
    pub async fn list_units(&self, tenant: TenantId) -> ArgoResult<Vec<OrgUnit>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, parent_id FROM org_unit \
             WHERE tenant_id = ? ORDER BY name",
        )
        .bind(tenant.to_string())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list org units: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|row| OrgUnit {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                name: row.get("name"),
                parent_id: row.get("parent_id"),
            })
            .collect())
    }

    /// List every member of `tenant` with their aliases attached.
    pub async fn list_members(&self, tenant: TenantId) -> ArgoResult<Vec<OrgMember>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, kind, canonical, unit_id, manager_id, worker_id \
             FROM org_member WHERE tenant_id = ? ORDER BY canonical",
        )
        .bind(tenant.to_string())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list org members: {e}")))?;
        let mut members = Vec::with_capacity(rows.len());
        for row in rows {
            members.push(self.row_to_member(row).await?);
        }
        Ok(members)
    }

    /// Collect every alias for one member.
    pub async fn aliases_for(&self, member_id: &str) -> ArgoResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT alias FROM org_member_alias WHERE member_id = ? ORDER BY alias",
        )
        .bind(member_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("aliases_for: {e}")))?;
        Ok(rows.into_iter().map(|(a,)| a).collect())
    }

    /// Resolve a name-or-alias to its canonical OrgMember row.
    /// Looks up canonical first; falls back to alias table.
    pub async fn resolve(
        &self,
        tenant: TenantId,
        name_or_alias: &str,
    ) -> ArgoResult<Option<OrgMember>> {
        let row = sqlx::query(
            "SELECT id, tenant_id, kind, canonical, unit_id, manager_id, worker_id \
             FROM org_member WHERE tenant_id = ? AND canonical = ?",
        )
        .bind(tenant.to_string())
        .bind(name_or_alias)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("resolve canonical: {e}")))?;
        if let Some(row) = row {
            return Ok(Some(self.row_to_member(row).await?));
        }
        let row = sqlx::query(
            "SELECT m.id, m.tenant_id, m.kind, m.canonical, m.unit_id, \
             m.manager_id, m.worker_id \
             FROM org_member_alias a JOIN org_member m ON m.id = a.member_id \
             WHERE m.tenant_id = ? AND a.alias = ? LIMIT 1",
        )
        .bind(tenant.to_string())
        .bind(name_or_alias)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("resolve alias: {e}")))?;
        Ok(match row {
            Some(row) => Some(self.row_to_member(row).await?),
            None => None,
        })
    }

    /// Walk the manager chain from `member_id` toward the root,
    /// returning the chain in order (direct manager first). 64-hop
    /// guard catches cycles that would otherwise loop forever.
    pub async fn reports_to_chain(
        &self,
        tenant: TenantId,
        member_id: &str,
    ) -> ArgoResult<Vec<OrgMember>> {
        let mut chain = Vec::new();
        let mut cur = member_id.to_string();
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 64 {
                return Err(ArgoError::Provider(
                    "reports-to chain exceeds 64 hops; org has a cycle".into(),
                ));
            }
            let row: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT manager_id FROM org_member WHERE id = ? AND tenant_id = ?",
            )
            .bind(&cur)
            .bind(tenant.to_string())
            .fetch_optional(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("reports_to: {e}")))?;
            let manager_id = match row {
                Some((Some(m),)) => m,
                _ => break,
            };
            let row = sqlx::query(
                "SELECT id, tenant_id, kind, canonical, unit_id, manager_id, worker_id \
                 FROM org_member WHERE id = ? AND tenant_id = ?",
            )
            .bind(&manager_id)
            .bind(tenant.to_string())
            .fetch_optional(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("reports_to manager: {e}")))?;
            match row {
                Some(row) => {
                    let manager = self.row_to_member(row).await?;
                    cur = manager.id.clone();
                    chain.push(manager);
                }
                None => break,
            }
        }
        Ok(chain)
    }

    pub(crate) async fn row_to_member(
        &self,
        row: sqlx::sqlite::SqliteRow,
    ) -> ArgoResult<OrgMember> {
        let id: String = row.get("id");
        let kind_str: String = row.get("kind");
        let kind = MemberKind::parse(&kind_str).unwrap_or(MemberKind::Human);
        let aliases = self.aliases_for(&id).await?;
        Ok(OrgMember {
            id,
            tenant_id: row.get("tenant_id"),
            kind,
            canonical: row.get("canonical"),
            unit_id: row.get("unit_id"),
            manager_id: row.get("manager_id"),
            worker_id: row.get("worker_id"),
            aliases,
        })
    }
}

/// What `OrgStore::ingest` reported back.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IngestSummary {
    pub units: usize,
    pub members: usize,
    pub aliases: usize,
    pub report_links: usize,
}

#[cfg(test)]
#[path = "org_tests.rs"]
mod org_tests;
