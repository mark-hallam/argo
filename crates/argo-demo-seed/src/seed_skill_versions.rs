//! Clean version-history chain for the demo catalog skills.
//!
//! ## The problem this fixes (NO THEATER — [`DEC-007`])
//!
//! The /skills detail "Version History" panel reads the `skill` table
//! rows for a name; the "Eval Re-run History" panel above it reads
//! `skill_reeval_history` (versions `v1`/`v2`/`v3`, with the seeded
//! 88/90/72% arc). The marketplace catalog seed
//! ([`crate::marketplace`]) installs each skill at a single `0.1.0`
//! version, so the two panels were INCOHERENT: a lone `0.1.0` row that
//! never joined the `v1`/`v2`/`v3` eval rows → every version read "No
//! eval runs yet".
//!
//! This seeder turns the demo tenant's single active catalog row into a
//! real three-version chain — `v1` → `v2` → `v3` — where `v3` is the
//! active head and `v1`/`v2` are superseded history. The version labels
//! now match `skill_reeval_history.skill_version`, so each row in the
//! Version History panel shows its real eval pass-rate movement and the
//! two panels tell one coherent story.
//!
//! ## Why the active id is preserved (no FK churn)
//!
//! The existing active row is relabelled to `v3` IN PLACE — its `id`
//! never changes — and the two history rows (`v1`, `v2`) are inserted
//! BEHIND it. So `worker_skill.skill_id`, `skill_ir.skill_id`, and any
//! other reference to the active catalog id stay valid; nothing points at
//! the new history rows.
//!
//! ## Tenant scoping + the cross-tenant "6× v0.1.0" dupes
//!
//! Everything here is keyed on `(name, tenant_id)` and the unique
//! constraint `UNIQUE(name, version, tenant_id)` (migration v84) means a
//! tenant can hold at most one row per version — so within the demo
//! tenant there are never dupes to clean. The "6× v0.1.0, all ACTIVE"
//! the LIVE walkthrough surfaced were rows under SIX DIFFERENT tenant_ids
//! (boot-seeds under shifting workspace tenants), pooled by the
//! formerly-unscoped version-timeline read; that leak is closed at the
//! read side by `argo_skill::list_versions_for_tenant`. This seeder owns
//! the complementary half: making the demo tenant's OWN chain coherent.
//!
//! ## Idempotency
//!
//! Re-seeding is a no-op: if the demo tenant's active row is already at
//! `v3`, the chain was built by a prior run and the skill is skipped.
//!
//! [`DEC-007`]: ../../../../docs/skycot/decisions/DEC-007-no-theater-in-demo.md

use std::sync::Arc;

use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};
use sqlx::Row;
use uuid::Uuid;

use crate::SeedResult;

/// Ordered version labels for the demo skill chain, oldest → newest.
/// Aligned with the `skill_reeval_history` arc seeded by
/// [`crate::seed_skill_reeval_history`] (`v{run_idx+1}`), so the Version
/// History panel joins the Eval Re-run History row-for-row.
const CHAIN_VERSIONS: [&str; 3] = ["v1", "v2", "v3"];

/// The active head label (last of [`CHAIN_VERSIONS`]).
const ACTIVE_VERSION: &str = "v3";

/// One catalog row, decoded for cloning into history rows.
struct HeadRow {
    id: Vec<u8>,
    name: String,
    body: String,
    frontmatter: String,
    source: String,
    hash: Vec<u8>,
    vertical_id: Option<String>,
    created_at: i64,
}

/// Turn each demo catalog skill's single active row into a coherent
/// `v1`→`v2`→`v3` version chain under `tenant_id`, idempotently. Returns
/// the number of skills that got a NEW chain built this run (`0` on a
/// re-seed where every skill is already at `v3`).
pub async fn seed_skill_version_chain(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let tenant_str = tenant_id.to_string();
    let names = demo_catalog_names(store, &tenant_str).await?;
    let mut built = 0usize;
    for name in &names {
        if build_chain_for(store, &tenant_str, name).await? {
            built += 1;
        }
    }
    Ok(built)
}

/// Active (non-superseded) catalog skill names under this tenant, sorted
/// for determinism. These are the rows /skills lists and the version
/// history reads.
async fn demo_catalog_names(
    store: &Arc<SessionStore>,
    tenant_str: &str,
) -> SeedResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT name FROM skill \
         WHERE tenant_id = ? AND lifecycle_state = 'active' AND superseded_by IS NULL \
         ORDER BY name ASC",
    )
    .bind(tenant_str)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("demo catalog names: {e}")))?;
    Ok(rows.iter().map(|r| r.get::<String, _>("name")).collect())
}

/// Build the chain for one `(name, tenant)`. Returns `true` when a new
/// chain was built, `false` when the chain already exists (idempotent
/// skip).
async fn build_chain_for(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    name: &str,
) -> SeedResult<bool> {
    let Some(head) = active_head(store, tenant_str, name).await? else {
        return Ok(false); // no active catalog row for this name — nothing to chain
    };
    // Idempotency: a head already at the active label means a prior run
    // built the chain. (The unique constraint guarantees one v3 max.)
    if version_exists(store, tenant_str, name, ACTIVE_VERSION).await? {
        return Ok(false);
    }

    // 1. Relabel the existing active row to the head version IN PLACE so
    //    its id (and every FK that points at it) is preserved.
    set_version(store, &head.id, ACTIVE_VERSION).await?;

    // 2. Insert the two history rows (v1, v2) BEHIND the head, each
    //    stamped slightly older so the newest-first timeline orders them
    //    v3, v2, v1. Link the supersede chain forward: v1 → v2 → head.
    let history_labels = &CHAIN_VERSIONS[..CHAIN_VERSIONS.len() - 1]; // ["v1", "v2"]
    let mut prev_id: Option<Vec<u8>> = None;
    let mut newest_history_id: Option<Vec<u8>> = None;
    for (offset, version) in history_labels.iter().enumerate() {
        // Older than the head by a stable, decreasing amount (v1 oldest).
        let age_days = (history_labels.len() - offset) as i64 * 14;
        let created_at = head.created_at - chrono::Duration::days(age_days).num_microseconds().unwrap_or(0);
        let new_id = insert_history_row(store, tenant_str, &head, version, created_at).await?;
        if let Some(p) = prev_id.take() {
            // Link the previous (older) history row forward to this one.
            set_superseded_by(store, &p, &new_id).await?;
        }
        prev_id = Some(new_id.clone());
        newest_history_id = Some(new_id);
    }
    // Link the newest history row (v2) forward to the active head (v3).
    if let Some(v2_id) = newest_history_id {
        set_superseded_by(store, &v2_id, &head.id).await?;
    }
    Ok(true)
}

/// The active (non-superseded) catalog row for `(name, tenant)`.
async fn active_head(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    name: &str,
) -> SeedResult<Option<HeadRow>> {
    let row = sqlx::query(
        "SELECT id, name, body, frontmatter, source, hash, vertical_id, created_at \
         FROM skill \
         WHERE name = ? AND tenant_id = ? AND superseded_by IS NULL AND lifecycle_state = 'active' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(name)
    .bind(tenant_str)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("active head lookup: {e}")))?;
    Ok(row.map(|r| HeadRow {
        id: r.get("id"),
        name: r.get("name"),
        body: r.get("body"),
        frontmatter: r.get("frontmatter"),
        source: r.get("source"),
        hash: r.get("hash"),
        vertical_id: r.get("vertical_id"),
        created_at: r.get("created_at"),
    }))
}

/// True when a `(name, version, tenant)` row already exists.
async fn version_exists(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    name: &str,
    version: &str,
) -> SeedResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM skill WHERE name = ? AND version = ? AND tenant_id = ?",
    )
    .bind(name)
    .bind(version)
    .bind(tenant_str)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("version exists probe: {e}")))?;
    Ok(count > 0)
}

/// Relabel an existing row's `version` in place (id preserved).
async fn set_version(store: &Arc<SessionStore>, id: &[u8], version: &str) -> SeedResult<()> {
    sqlx::query("UPDATE skill SET version = ? WHERE id = ?")
        .bind(version)
        .bind(id)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("set version: {e}")))?;
    Ok(())
}

/// Point one row's `superseded_by` at another (mark the first superseded).
async fn set_superseded_by(
    store: &Arc<SessionStore>,
    id: &[u8],
    superseded_by: &[u8],
) -> SeedResult<()> {
    sqlx::query("UPDATE skill SET superseded_by = ? WHERE id = ?")
        .bind(superseded_by)
        .bind(id)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("set superseded_by: {e}")))?;
    Ok(())
}

/// Insert a superseded history row that clones the head's body /
/// frontmatter / source / hash but carries the history `version` +
/// `created_at`. Marked `lifecycle_state = 'deprecated'` (a superseded
/// older version is not the active row). Returns the new row's id blob.
async fn insert_history_row(
    store: &Arc<SessionStore>,
    tenant_str: &str,
    head: &HeadRow,
    version: &str,
    created_at: i64,
) -> SeedResult<Vec<u8>> {
    let id = Uuid::now_v7().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO skill \
         (id, name, version, body, frontmatter, source, hash, learned_in, \
          superseded_by, created_at, vertical_id, lifecycle_state, tenant_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?, ?, 'deprecated', ?)",
    )
    .bind(&id)
    .bind(&head.name)
    .bind(version)
    .bind(&head.body)
    .bind(&head.frontmatter)
    .bind(&head.source)
    .bind(&head.hash)
    .bind(created_at)
    .bind(&head.vertical_id)
    .bind(tenant_str)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("insert history row: {e}")))?;
    Ok(id)
}

#[cfg(test)]
#[path = "seed_skill_versions_tests.rs"]
mod tests;
