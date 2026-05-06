//! Tenant-scoped shared memory for all workers on a team.
use crate::MemoryStore;
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Namespace for memory shared by every worker in one tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedTeamNs {
    /// Tenant boundary for shared-team memory.
    pub tenant_id: TenantId,
}

impl SharedTeamNs {
    /// Build a shared-team namespace for one tenant.
    #[must_use]
    pub fn new(tenant_id: TenantId) -> Self {
        Self { tenant_id }
    }
}

/// Build a tenant-scoped shared-team memory namespace.
#[must_use]
pub fn shared_team(tenant_id: TenantId) -> SharedTeamNs {
    SharedTeamNs::new(tenant_id)
}

/// A shared-team memory row with the worker that authored it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedEntry {
    /// Stable row id.
    pub id: Uuid,
    /// Worker that wrote the shared entry.
    pub author_worker_id: WorkerId,
    /// Entry body visible to every worker in the tenant.
    pub content: String,
    /// Time the entry was written.
    pub written_at: DateTime<Utc>,
}

pub(crate) const SHARED_TEAM_SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS shared_team_memory (
        id               BLOB PRIMARY KEY DEFAULT (randomblob(16)),
        tenant_id        BLOB NOT NULL DEFAULT (zeroblob(16)),
        author_worker_id BLOB NOT NULL DEFAULT (zeroblob(16)),
        content          TEXT NOT NULL DEFAULT '',
        written_at       INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE INDEX IF NOT EXISTS idx_shared_team_memory_tenant_written
     ON shared_team_memory(tenant_id, written_at, id)",
    "CREATE INDEX IF NOT EXISTS idx_shared_team_memory_author
     ON shared_team_memory(tenant_id, author_worker_id, written_at)",
];

impl MemoryStore {
    /// Append a tenant-wide shared-team memory row with an author stamp.
    pub async fn write_shared_team(
        &self,
        ns: &SharedTeamNs,
        author_worker_id: WorkerId,
        content: String,
    ) -> ArgoResult<SharedEntry> {
        self.ensure_shared_team_schema().await?;
        let id = Uuid::now_v7();
        let written_at = Utc::now();
        sqlx::query(
            "INSERT INTO shared_team_memory
             (id, tenant_id, author_worker_id, content, written_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(ns.tenant_id.0.as_bytes().as_slice())
        .bind(author_worker_id.0.as_bytes().as_slice())
        .bind(&content)
        .bind(written_at.timestamp_micros())
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory write_shared_team: {e}")))?;
        Ok(SharedEntry {
            id,
            author_worker_id,
            content,
            written_at,
        })
    }

    /// Read all shared-team rows visible inside one tenant.
    pub async fn read_shared_team(&self, ns: &SharedTeamNs) -> ArgoResult<Vec<SharedEntry>> {
        self.ensure_shared_team_schema().await?;
        let rows = sqlx::query_as::<_, (Vec<u8>, Vec<u8>, String, i64)>(
            "SELECT id, author_worker_id, content, written_at
             FROM shared_team_memory
             WHERE tenant_id = ?
             ORDER BY written_at ASC, id ASC",
        )
        .bind(ns.tenant_id.0.as_bytes().as_slice())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("memory read_shared_team: {e}")))?;
        rows.into_iter()
            .map(|(id_b, author_b, content, written_at)| {
                let id = uuid_from_blob(&id_b, "shared_team_memory.id")?;
                let author_worker_id = WorkerId(uuid_from_blob(
                    &author_b,
                    "shared_team_memory.author_worker_id",
                )?);
                let written_at = Utc.timestamp_micros(written_at).single().ok_or_else(|| {
                    ArgoError::Schema("shared_team_memory.written_at out of range".into())
                })?;
                Ok(SharedEntry {
                    id,
                    author_worker_id,
                    content,
                    written_at,
                })
            })
            .collect()
    }

    async fn ensure_shared_team_schema(&self) -> ArgoResult<()> {
        for sql in SHARED_TEAM_SCHEMA {
            sqlx::query(sql)
                .execute(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("memory shared_team schema: {e}")))?;
        }
        Ok(())
    }
}

fn uuid_from_blob(b: &[u8], field: &'static str) -> ArgoResult<Uuid> {
    Uuid::from_slice(b).map_err(|e| ArgoError::Schema(format!("{field}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;
    use std::sync::Arc;

    async fn fresh() -> MemoryStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        MemoryStore::new(Arc::new(store))
    }

    async fn apply_shared_team_schema(memory: &MemoryStore) {
        for sql in SHARED_TEAM_SCHEMA {
            sqlx::query(sql).execute(memory.pool()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn same_tenant_cross_worker_read_sees_writes() {
        let memory = fresh().await;
        let ns = shared_team(TenantId::new());
        let worker_a = WorkerId::new();
        let worker_b = WorkerId::new();

        memory
            .write_shared_team(&ns, worker_a, "handoff from A".into())
            .await
            .unwrap();
        memory
            .write_shared_team(&ns, worker_b, "handoff from B".into())
            .await
            .unwrap();

        let entries = memory.read_shared_team(&ns).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "handoff from A");
        assert_eq!(entries[1].content, "handoff from B");
    }

    #[tokio::test]
    async fn cross_tenant_isolation_hides_other_tenant_writes() {
        let memory = fresh().await;
        let tenant_a = shared_team(TenantId::new());
        let tenant_b = shared_team(TenantId::new());

        memory
            .write_shared_team(&tenant_a, WorkerId::new(), "tenant A only".into())
            .await
            .unwrap();

        assert!(memory.read_shared_team(&tenant_b).await.unwrap().is_empty());
        let visible = memory.read_shared_team(&tenant_a).await.unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].content, "tenant A only");
    }

    #[tokio::test]
    async fn author_stamp_survives_round_trip() {
        let memory = fresh().await;
        let ns = shared_team(TenantId::new());
        let author = WorkerId::new();

        let written = memory
            .write_shared_team(&ns, author, "signed note".into())
            .await
            .unwrap();
        let read = memory.read_shared_team(&ns).await.unwrap();

        assert_eq!(read[0].id, written.id);
        assert_eq!(read[0].author_worker_id, author);
        assert_eq!(read[0].content, "signed note");
        assert_eq!(read[0].written_at, written.written_at);
    }

    #[tokio::test]
    async fn migration_is_idempotent_on_fresh_and_layered_db() {
        let memory = fresh().await;

        apply_shared_team_schema(&memory).await;
        apply_shared_team_schema(&memory).await;

        memory.ensure_layer_schema().await.unwrap();
        apply_shared_team_schema(&memory).await;
        apply_shared_team_schema(&memory).await;

        let table: String = sqlx::query_scalar(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name = 'shared_team_memory'",
        )
        .fetch_one(memory.pool())
        .await
        .unwrap();
        assert_eq!(table, "shared_team_memory");
    }
}
