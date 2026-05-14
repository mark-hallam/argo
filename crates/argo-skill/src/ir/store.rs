//! SQLite-backed store for the canonical Skill IR (L2 brief).
//!
//! Every write to `skill_ir` records the graph's canonical JSON, its
//! `sha256` hash, and a `prev_hash` pointer to the previous version
//! in the same `(tenant_id, skill_id)` family. The chain lets a
//! verifier detect tampering: if anyone edits a row's `ir_json` in
//! place, the recomputed canonical hash no longer matches the
//! `prev_hash` recorded in the next row.
//!
//! The schema lives in `crates/argo-skill/migrations/0078_skill_ir.sql`
//! and is shipped into the binary via [`include_str!`]; callers run
//! [`ensure_schema`] once after opening a [`SessionStore`] to apply
//! it. Every statement is `IF NOT EXISTS`-guarded so the call is
//! idempotent against a populated database.
//!
//! Track L brief: `streams/L2-skill-ir-storage.md`.
use crate::ir::{SkillGraph, canonical_hash, canonical_json};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;

/// SQL applied by [`ensure_schema`]. Bundled into the binary so the
/// migration travels with the crate.
const SCHEMA_SQL: &str = include_str!("../../migrations/0078_skill_ir.sql");

/// Tenant-scoped, hash-chained store for skill IR documents.
///
/// All methods are tenant-scoped: passing a `tenant_id` that doesn't
/// own the row returns `Ok(None)` (for reads) or inserts a separate
/// row (for writes). Cross-tenant reads cannot collide because the
/// table's primary key prefixes on `tenant_id`.
#[async_trait]
pub trait SkillIrStore: Send + Sync {
    /// Persist `graph` as the `version`-th revision of `skill_id`
    /// under `tenant_id`. Returns the canonical hash of the stored
    /// graph (= `sha256(canonical_json(graph))`).
    ///
    /// Hash-chain: if any prior row exists for `(tenant_id,
    /// skill_id)`, the new row's `prev_hash` is set to the latest
    /// existing row's `ir_hash`. The first row's `prev_hash` is
    /// `NULL`.
    ///
    /// Re-putting the same `(tenant_id, skill_id, version)` triple is
    /// rejected with [`ArgoError::Invariant`] — versions are
    /// immutable; supersede by writing a new `version` string.
    async fn put(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
        version: &str,
        graph: &SkillGraph,
    ) -> ArgoResult<[u8; 32]>;

    /// Fetch the IR for `(tenant_id, skill_id, version)`, or `None`
    /// if no such row exists for this tenant.
    async fn get(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
        version: &str,
    ) -> ArgoResult<Option<SkillGraph>>;

    /// Fetch the latest-by-`created_at` IR for `(tenant_id,
    /// skill_id)`, paired with its version string. Returns `None`
    /// when the skill has no IR rows under this tenant.
    async fn latest(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
    ) -> ArgoResult<Option<(String, SkillGraph)>>;

    /// Every version string recorded for `(tenant_id, skill_id)`,
    /// newest first. Returns an empty vector when no rows exist.
    async fn list_versions(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
    ) -> ArgoResult<Vec<String>>;
}

/// SQLite-backed [`SkillIrStore`] implementation built on top of
/// [`SessionStore`]'s pool.
#[derive(Clone)]
pub struct SqliteSkillIrStore<'a> {
    store: &'a SessionStore,
}

impl<'a> SqliteSkillIrStore<'a> {
    /// Wrap a [`SessionStore`]. The caller is expected to have run
    /// [`ensure_schema`] at some point before issuing any reads or
    /// writes; constructing the wrapper is otherwise free.
    pub fn new(store: &'a SessionStore) -> Self {
        Self { store }
    }

    async fn latest_hash(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
    ) -> ArgoResult<Option<[u8; 32]>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT ir_hash FROM skill_ir \
             WHERE tenant_id = ? AND skill_id = ? \
             ORDER BY created_at DESC, version DESC LIMIT 1",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_ir latest_hash: {e}")))?;
        match row {
            Some((blob,)) => Ok(Some(decode_hash(&blob)?)),
            None => Ok(None),
        }
    }
}

#[async_trait]
impl SkillIrStore for SqliteSkillIrStore<'_> {
    async fn put(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
        version: &str,
        graph: &SkillGraph,
    ) -> ArgoResult<[u8; 32]> {
        let ir_json = canonical_json(graph)?;
        let ir_hash = canonical_hash(graph)?;
        let prev_hash = self.latest_hash(tenant_id, skill_id).await?;
        let now = Utc::now().timestamp_micros();
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO skill_ir \
             (tenant_id, skill_id, version, ir_json, ir_hash, prev_hash, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .bind(version)
        .bind(&ir_json)
        .bind(ir_hash.as_slice())
        .bind(prev_hash.as_ref().map(|h| h.as_slice()))
        .bind(now)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_ir put: {e}")))?;
        if inserted.rows_affected() == 0 {
            return Err(ArgoError::Invariant(format!(
                "skill_ir: version {version} already recorded for skill {skill_id} \
                 under this tenant — versions are immutable"
            )));
        }
        Ok(ir_hash)
    }

    async fn get(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
        version: &str,
    ) -> ArgoResult<Option<SkillGraph>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT ir_json FROM skill_ir \
             WHERE tenant_id = ? AND skill_id = ? AND version = ?",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .bind(version)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_ir get: {e}")))?;
        match row {
            Some((json,)) => Ok(Some(decode_graph(&json)?)),
            None => Ok(None),
        }
    }

    async fn latest(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
    ) -> ArgoResult<Option<(String, SkillGraph)>> {
        let row = sqlx::query(
            "SELECT version, ir_json FROM skill_ir \
             WHERE tenant_id = ? AND skill_id = ? \
             ORDER BY created_at DESC, version DESC LIMIT 1",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_ir latest: {e}")))?;
        match row {
            Some(r) => {
                let version: String = r.get("version");
                let json: String = r.get("ir_json");
                Ok(Some((version, decode_graph(&json)?)))
            }
            None => Ok(None),
        }
    }

    async fn list_versions(
        &self,
        tenant_id: TenantId,
        skill_id: SkillId,
    ) -> ArgoResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT version FROM skill_ir \
             WHERE tenant_id = ? AND skill_id = ? \
             ORDER BY created_at DESC, version DESC",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(skill_id.0.as_bytes().as_slice())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_ir list_versions: {e}")))?;
        Ok(rows.into_iter().map(|(v,)| v).collect())
    }
}

/// One row of the hash-chain walk produced by [`verify_chain`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainRow {
    /// Version string of the row.
    pub version: String,
    /// `prev_hash` recorded on the row at insert time (None for the
    /// first row in the chain).
    pub stored_prev_hash: Option<[u8; 32]>,
    /// Canonical hash of `ir_json` recomputed at verification time.
    pub recomputed_hash: [u8; 32],
}

/// Walk every IR row for `(tenant_id, skill_id)` in insert order and
/// verify each row's `prev_hash` matches the previous row's
/// recomputed canonical hash. Returns `Ok(Vec<ChainRow>)` if the
/// chain is intact; otherwise [`ArgoError::Invariant`] naming the
/// first row whose `prev_hash` mismatches.
///
/// The recomputation step rehashes `ir_json` (not the stored
/// `ir_hash` blob) so in-place tampering with `ir_json` is detected
/// even when the stored `ir_hash` is updated to match — what cannot
/// be forged without rewriting the *next* row is the `prev_hash`
/// pointer.
pub async fn verify_chain(
    store: &SessionStore,
    tenant_id: TenantId,
    skill_id: SkillId,
) -> ArgoResult<Vec<ChainRow>> {
    let rows = sqlx::query(
        "SELECT version, ir_json, prev_hash FROM skill_ir \
         WHERE tenant_id = ? AND skill_id = ? \
         ORDER BY created_at ASC, version ASC",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(skill_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_ir verify_chain: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    let mut expected_prev: Option<[u8; 32]> = None;
    for row in rows {
        let version: String = row.get("version");
        let json: String = row.get("ir_json");
        let prev_blob: Option<Vec<u8>> = row.get("prev_hash");
        let stored_prev = prev_blob.as_deref().map(decode_hash).transpose()?;
        if stored_prev != expected_prev {
            return Err(ArgoError::Invariant(format!(
                "skill_ir chain broken at version {version}: \
                 stored prev_hash does not match previous row's recomputed hash"
            )));
        }
        let graph = decode_graph(&json)?;
        let recomputed = canonical_hash(&graph)?;
        expected_prev = Some(recomputed);
        out.push(ChainRow {
            version,
            stored_prev_hash: stored_prev,
            recomputed_hash: recomputed,
        });
    }
    Ok(out)
}

/// Apply the `skill_ir` migration to `store`'s pool. Idempotent.
///
/// SQLite's `sqlx::Executor::execute` only runs a single statement
/// per call, so the embedded SQL has line comments stripped (so a
/// `--` line's semicolon doesn't fracture statements) and is then
/// split on `;` with each non-empty statement run separately. The
/// schema has no string literals that contain `--`, so the
/// simple-minded comment stripper is sufficient.
pub async fn ensure_schema(store: &SessionStore) -> ArgoResult<()> {
    let stripped: String = SCHEMA_SQL
        .lines()
        .map(|line| match line.find("--") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';') {
        let trimmed = stmt.trim();
        if trimmed.is_empty() {
            continue;
        }
        sqlx::query(trimmed)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("skill_ir ensure_schema: {e}")))?;
    }
    Ok(())
}

fn decode_graph(json: &str) -> ArgoResult<SkillGraph> {
    serde_json::from_str(json)
        .map_err(|e| ArgoError::Provider(format!("skill_ir decode graph: {e}")))
}

fn decode_hash(blob: &[u8]) -> ArgoResult<[u8; 32]> {
    blob.try_into()
        .map_err(|_| ArgoError::Provider("skill_ir: hash blob is not 32 bytes".to_string()))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
