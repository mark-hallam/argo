//! Cross-tenant skill sharing query helpers (Stream
//! `claude-skill-share-cross-tenant-poc`, mission claim 4).
//!
//! Two surface functions:
//!
//! * [`set_shared`] — toggle the `shared` flag on a `(skill_name,
//!   version)` row and (optionally) write the `shared_with_cluster`
//!   label.
//! * [`list_shared_for_tenant`] — return [`SharedSkillRow`]s that the
//!   given viewer tenant is allowed to read. A skill is visible if
//!   `shared = 1` and `shared_with_cluster` matches the viewer
//!   tenant's *cluster name*. A tenant's cluster name is the `name`
//!   of the root `org_unit` (the row with `parent_id IS NULL`)
//!   belonging to that tenant in the `org_chart` schema (raw SQL
//!   `21__org_chart.sql`, reconciled into versioned `v22.rs`).
//!
//! ## PoC scope (deliberate)
//!
//! - No agent-runner integration — runtime lookup lands in a
//!   follow-up brief (`claude-skill-share-runner-wire`).
//! - No provenance / signing / trust gates — those land separately.
//! - The `skill` table has no `tenant_id` column today, so there is
//!   no notion of an "owning tenant" on a shared row. The cluster
//!   label on the skill *is* the cross-tenant authorization marker.
//!   `shared_with_cluster IS NULL` is treated as "no cluster
//!   declared yet" and is invisible to every viewer; callers should
//!   pass an explicit cluster when calling [`set_shared`].

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

/// One shared-skill row visible to a viewer tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharedSkillRow {
    /// Skill UUID, formatted as a string for cross-process transport.
    pub skill_id: String,
    /// Skill name (the `(name, version)` UNIQUE key in the `skill`
    /// table).
    pub name: String,
    /// Skill version string.
    pub version: String,
    /// The cluster label authorized to read the skill (matches the
    /// viewer's resolved cluster).
    pub shared_with_cluster: String,
    /// Skill creation timestamp in epoch microseconds.
    pub created_at: i64,
}

/// Toggle the `shared` flag and optionally update the
/// `shared_with_cluster` label on a single skill version row.
///
/// `cluster = None` clears the cluster label (sets it back to NULL).
/// `cluster = Some("X")` sets it to `"X"`. The flag and the cluster
/// are written in a single UPDATE so the two columns can never be
/// observed half-updated.
///
/// Returns `Ok(())` even when no row matches — callers can treat the
/// API as "best effort", consistent with the other free-function
/// helpers in this crate.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// argo_store::skill_share::set_shared(store, "my-skill", "1.0.0", true, Some("acme")).await?;
/// # Ok(()) }
/// ```
pub async fn set_shared(
    store: &SessionStore,
    skill_name: &str,
    version: &str,
    shared: bool,
    cluster: Option<&str>,
) -> ArgoResult<()> {
    sqlx::query(
        "UPDATE skill SET shared = ?, shared_with_cluster = ? \
         WHERE name = ? AND version = ?",
    )
    .bind(i64::from(shared))
    .bind(cluster)
    .bind(skill_name)
    .bind(version)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("set_shared: {e}")))?;
    Ok(())
}

/// Return every shared skill that `tenant_id` is allowed to read.
///
/// Resolution order:
///
/// 1. Look up the viewer's cluster name — the `name` of the root
///    `org_unit` (the row with `parent_id IS NULL`) for that tenant.
///    If the tenant has no root unit, the result is empty.
/// 2. Return shared skill rows whose `shared_with_cluster` exactly
///    matches that name.
///
/// Skills with `shared = 0` are never returned. Skills whose
/// `shared_with_cluster` is NULL are never returned (PoC limitation —
/// see module docs).
///
/// # Examples
///
/// ```no_run
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let rows = argo_store::skill_share::list_shared_for_tenant(store, "tenant-a").await?;
/// assert!(rows.iter().all(|r| !r.shared_with_cluster.is_empty()));
/// # Ok(()) }
/// ```
pub async fn list_shared_for_tenant(
    store: &SessionStore,
    tenant_id: &str,
) -> ArgoResult<Vec<SharedSkillRow>> {
    let Some(cluster) = cluster_for_tenant(store, tenant_id).await? else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query(
        "SELECT id, name, version, shared_with_cluster, created_at \
         FROM skill \
         WHERE shared = 1 AND shared_with_cluster = ? \
         ORDER BY created_at DESC, name ASC, version ASC",
    )
    .bind(&cluster)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_shared_for_tenant: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id_bytes: Vec<u8> = row.get("id");
        let skill_id = Uuid::from_slice(&id_bytes)
            .map(|u| u.to_string())
            .map_err(|e| ArgoError::Provider(format!("skill_share: bad uuid bytes: {e}")))?;
        let name: String = row.get("name");
        let version: String = row.get("version");
        let cluster_col: String = row.get("shared_with_cluster");
        let created_at: i64 = row.get("created_at");
        out.push(SharedSkillRow {
            skill_id,
            name,
            version,
            shared_with_cluster: cluster_col,
            created_at,
        });
    }
    Ok(out)
}

async fn cluster_for_tenant(
    store: &SessionStore,
    tenant_id: &str,
) -> ArgoResult<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM org_unit \
         WHERE tenant_id = ? AND parent_id IS NULL \
         ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("cluster_for_tenant: {e}")))?;
    Ok(row.map(|(name,)| name))
}
