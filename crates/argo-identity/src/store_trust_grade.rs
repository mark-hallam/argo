//! `IdentityStore::set_worker_trust_grade` + `touch_worker_last_used`
//! live in their own file so the parent [`store`](crate::store) module
//! stays under the 500-LOC ceiling. Mirrors `update_channels` /
//! `update_charter` shape: tenant + worker scoped UPDATE, returns
//! `ArgoError::Provider` when the row is missing.
//!
//! Stream `mvp-worker-trust-grade-column` — Tier M prereq for the
//! routing-engine candidate ranking in
//! `mission-routing-skillmatch-rolematch` and
//! `mission-skill-propagation-loop`.

use crate::{store::IdentityStore, WorkerId};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;

impl IdentityStore<'_> {
    /// Set the worker-side trust grade (`Stream
    /// mvp-worker-trust-grade-column`). The numeric scale mirrors
    /// [`argo_skill::trust::TrustGrade`] so the same value carries
    /// across skill and worker-side ranking. Returns
    /// `ArgoError::Provider` when the (`tenant_id`, `worker_id`) pair
    /// does not match a stored row.
    pub async fn set_worker_trust_grade(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        grade: u8,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let result = sqlx::query(
            "UPDATE worker SET trust_grade = ? WHERE id = ? AND tenant_id = ?",
        )
        .bind(i64::from(grade))
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("set worker trust grade: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }

    /// Stamp `worker.last_used_at = now()` (UTC, RFC-3339). Called by
    /// the routing engine each time a worker is dispatched so the
    /// skill-match ranker can prefer recently-used candidates.
    /// Returns `ArgoError::Provider` when the (`tenant_id`,
    /// `worker_id`) pair does not match a stored row.
    pub async fn touch_worker_last_used(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE worker SET last_used_at = ? WHERE id = ? AND tenant_id = ?",
        )
        .bind(now)
        .bind(worker_id.to_string())
        .bind(tenant_id.to_string())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("touch worker last used: {e}")))?;
        if result.rows_affected() == 0 {
            return Err(ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant_id}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{IdentityStore, Persona, Role, Worker, WorkerId};
    use argo_store::SessionStore;
    use argo_types::TenantId;

    async fn fresh_store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    fn draft(tenant: TenantId) -> Worker {
        Worker::draft(
            tenant,
            "gtm-engineer",
            Persona::named("Maya", "op"),
            Role::new("Sell."),
        )
    }

    #[tokio::test]
    async fn set_worker_trust_grade_persists() {
        let store = fresh_store().await;
        let identities = IdentityStore::new(&store);
        let tenant = TenantId::new();
        let worker = draft(tenant);
        identities.create_worker(&worker).await.unwrap();

        identities
            .set_worker_trust_grade(tenant, worker.id, 4)
            .await
            .unwrap();

        let loaded = identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.trust_grade, 4);
    }

    #[tokio::test]
    async fn touch_worker_last_used_sets_timestamp() {
        let store = fresh_store().await;
        let identities = IdentityStore::new(&store);
        let tenant = TenantId::new();
        let worker = draft(tenant);
        identities.create_worker(&worker).await.unwrap();
        let before = chrono::Utc::now();

        identities
            .touch_worker_last_used(tenant, worker.id)
            .await
            .unwrap();

        let loaded = identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .unwrap();
        let stamped = loaded.last_used_at.expect("timestamp set");
        assert!(stamped >= before - chrono::Duration::seconds(1));
        assert!(stamped <= chrono::Utc::now() + chrono::Duration::seconds(1));
    }

    #[tokio::test]
    async fn worker_default_trust_grade_is_zero() {
        let store = fresh_store().await;
        let identities = IdentityStore::new(&store);
        let tenant = TenantId::new();
        let worker = draft(tenant);
        identities.create_worker(&worker).await.unwrap();

        let loaded = identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.trust_grade, 0);
        assert!(loaded.last_used_at.is_none());
    }

    #[tokio::test]
    async fn list_workers_returns_trust_grade_field() {
        let store = fresh_store().await;
        let identities = IdentityStore::new(&store);
        let tenant = TenantId::new();
        let worker = draft(tenant);
        identities.create_worker(&worker).await.unwrap();
        identities
            .set_worker_trust_grade(tenant, worker.id, 3)
            .await
            .unwrap();

        let listed = identities.list_workers(tenant, false).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].trust_grade, 3);
    }

    #[tokio::test]
    async fn set_worker_trust_grade_unknown_id_returns_provider_error() {
        let store = fresh_store().await;
        let identities = IdentityStore::new(&store);
        let tenant = TenantId::new();

        let err = identities
            .set_worker_trust_grade(tenant, WorkerId::new(), 2)
            .await
            .expect_err("missing worker should error");
        assert!(err.to_string().contains("not found"), "got {err}");
    }

    #[tokio::test]
    async fn migration_v46_applies_to_v45_db() {
        // Round-trip: migrate fully, drop back to v45, verify trust
        // columns are gone, re-migrate, verify they reappear and the
        // identity API works on the re-migrated DB. Mirrors the
        // approvals v44 down-then-up test in `approvals_tests.rs`.
        use sqlx::Row;

        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        assert_eq!(store.schema_version().await.unwrap(), 46);

        store.migrate_down_to(45).await.unwrap();
        let cols: Vec<String> = sqlx::query("PRAGMA table_info(worker)")
            .fetch_all(store.pool())
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();
        assert!(
            !cols.iter().any(|c| c == "trust_grade"),
            "trust_grade should drop at v45, got {cols:?}"
        );
        assert!(
            !cols.iter().any(|c| c == "last_used_at"),
            "last_used_at should drop at v45, got {cols:?}"
        );

        store.migrate().await.unwrap();
        assert_eq!(store.schema_version().await.unwrap(), 46);

        let identities = IdentityStore::new(&store);
        let tenant = TenantId::new();
        let worker = draft(tenant);
        identities.create_worker(&worker).await.unwrap();
        identities
            .set_worker_trust_grade(tenant, worker.id, 5)
            .await
            .unwrap();
        let loaded = identities
            .get_worker(tenant, worker.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.trust_grade, 5);
    }
}
