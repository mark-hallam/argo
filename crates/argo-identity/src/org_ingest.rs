//! YAML ingest path for the org chart. Moved out of `org.rs` to
//! keep the main module under the 500-LOC budget.

use crate::org::{IngestSummary, MemberKind, OrgStore};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Wire shape for `argo org ingest <yaml>`. Round-trips via
/// `serde_yaml` so operators can hand-edit + re-ingest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgChartYaml {
    /// Tenant the org chart belongs to.
    pub tenant: String,
    /// Root org name (informational; the unit tree below carries
    /// the structure).
    pub org: String,
    /// All units flat, with `parent` referring to another unit
    /// by name. Resolved to ids at ingest.
    #[serde(default)]
    pub units: Vec<OrgUnitYaml>,
    /// All members flat, with `unit` and `manager` referring to
    /// names. Resolved to ids at ingest.
    #[serde(default)]
    pub members: Vec<OrgMemberYaml>,
}

/// One unit row in the YAML view.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgUnitYaml {
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
}

/// One member row in the YAML view.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrgMemberYaml {
    pub canonical: String,
    pub kind: MemberKind,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl OrgStore {
    /// Bulk-import an org chart from a parsed YAML doc. Existing
    /// rows for the same `(tenant, canonical)` pair are upserted;
    /// fresh names create new rows. Aliases that already exist are
    /// preserved.
    pub async fn ingest(
        &self,
        tenant: TenantId,
        chart: &OrgChartYaml,
    ) -> ArgoResult<IngestSummary> {
        let now = Utc::now().timestamp_micros();
        let mut unit_ids: BTreeMap<String, String> = BTreeMap::new();
        let mut summary = IngestSummary::default();
        // Pass 1: ensure unit rows exist.
        for unit in &chart.units {
            let id = self.upsert_unit(tenant, &unit.name, now).await?;
            unit_ids.insert(unit.name.clone(), id);
            summary.units += 1;
        }
        // Pass 2: hook up parent references now that all unit ids
        // exist. Splitting the passes lets the YAML reference
        // parents in any order.
        for unit in &chart.units {
            if let Some(parent_name) = &unit.parent {
                let child_id = unit_ids.get(&unit.name).cloned().ok_or_else(|| {
                    ArgoError::Provider(format!(
                        "ingest: unit '{}' missing from id map",
                        unit.name
                    ))
                })?;
                let parent_id = unit_ids.get(parent_name).cloned().ok_or_else(|| {
                    ArgoError::Provider(format!(
                        "ingest: unit '{}' references unknown parent '{parent_name}'",
                        unit.name
                    ))
                })?;
                sqlx::query("UPDATE org_unit SET parent_id = ? WHERE id = ?")
                    .bind(&parent_id)
                    .bind(&child_id)
                    .execute(self.pool())
                    .await
                    .map_err(|e| ArgoError::Provider(format!("ingest unit parent: {e}")))?;
            }
        }
        // Pass 3: members + aliases.
        let mut member_ids: BTreeMap<String, String> = BTreeMap::new();
        for m in &chart.members {
            let unit_id = m.unit.as_deref().and_then(|u| unit_ids.get(u).cloned());
            let id = self
                .upsert_member(tenant, m, unit_id.as_deref(), now)
                .await?;
            member_ids.insert(m.canonical.clone(), id.clone());
            for alias in &m.aliases {
                self.upsert_alias(&id, alias).await?;
                summary.aliases += 1;
            }
            summary.members += 1;
        }
        // Pass 4: manager chains, now that all member ids exist.
        for m in &chart.members {
            if let Some(manager_name) = &m.manager {
                let child_id = member_ids.get(&m.canonical).cloned().ok_or_else(|| {
                    ArgoError::Provider(format!(
                        "ingest: member '{}' missing from id map",
                        m.canonical
                    ))
                })?;
                let manager_id = member_ids.get(manager_name).cloned().ok_or_else(|| {
                    ArgoError::Provider(format!(
                        "ingest: member '{}' references unknown manager '{manager_name}'",
                        m.canonical
                    ))
                })?;
                sqlx::query("UPDATE org_member SET manager_id = ? WHERE id = ?")
                    .bind(&manager_id)
                    .bind(&child_id)
                    .execute(self.pool())
                    .await
                    .map_err(|e| ArgoError::Provider(format!("ingest member manager: {e}")))?;
                sqlx::query(
                    "INSERT OR IGNORE INTO org_reports_to (member_id, manager_id, depth) \
                     VALUES (?, ?, 1)",
                )
                .bind(&child_id)
                .bind(&manager_id)
                .execute(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("ingest reports_to: {e}")))?;
                summary.report_links += 1;
            }
        }
        Ok(summary)
    }

    async fn upsert_unit(
        &self,
        tenant: TenantId,
        name: &str,
        now: i64,
    ) -> ArgoResult<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM org_unit WHERE tenant_id = ? AND name = ?",
        )
        .bind(tenant.to_string())
        .bind(name)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("upsert_unit lookup: {e}")))?;
        if let Some((id,)) = row {
            return Ok(id);
        }
        let id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO org_unit (id, tenant_id, name, parent_id, created_at) \
             VALUES (?, ?, ?, NULL, ?)",
        )
        .bind(&id)
        .bind(tenant.to_string())
        .bind(name)
        .bind(now)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert org_unit: {e}")))?;
        Ok(id)
    }

    async fn upsert_member(
        &self,
        tenant: TenantId,
        m: &OrgMemberYaml,
        unit_id: Option<&str>,
        now: i64,
    ) -> ArgoResult<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM org_member WHERE tenant_id = ? AND canonical = ?",
        )
        .bind(tenant.to_string())
        .bind(&m.canonical)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("upsert_member lookup: {e}")))?;
        if let Some((id,)) = row {
            return Ok(id);
        }
        let id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO org_member \
             (id, tenant_id, kind, canonical, unit_id, manager_id, worker_id, created_at) \
             VALUES (?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(&id)
        .bind(tenant.to_string())
        .bind(m.kind.as_str())
        .bind(&m.canonical)
        .bind(unit_id)
        .bind(m.worker_id.as_deref())
        .bind(now)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert org_member: {e}")))?;
        Ok(id)
    }

    async fn upsert_alias(&self, member_id: &str, alias: &str) -> ArgoResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO org_member_alias (member_id, alias) VALUES (?, ?)",
        )
        .bind(member_id)
        .bind(alias)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert org_member_alias: {e}")))?;
        Ok(())
    }
}
