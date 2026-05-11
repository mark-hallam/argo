//! [`argo_skill::WorkerTrustReader`] adapter for the
//! [`IdentityStore`] worker row. Lives in `argo-cli` because
//! `argo-skill` deliberately does not depend on `argo-identity`
//! (the dep arrow is `argo-identity → argo-store`; inverting it
//! would create a cycle). The adapter wraps an [`IdentityStore`]
//! borrow and translates the `Uuid` the trait uses back into the
//! [`WorkerId`] newtype the store API takes.

use argo_identity::{IdentityStore, WorkerId};
use argo_skill::WorkerTrustReader;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use uuid::Uuid;

/// Adapter struct: holds a borrow of the [`IdentityStore`] so the
/// trait impl can be a free-standing newtype rather than an
/// orphan-rule-violating bare `impl WorkerTrustReader for
/// IdentityStore<'_>` from a third crate.
pub struct IdentityWorkerTrustReader<'a, 's> {
    identity: &'a IdentityStore<'s>,
}

impl<'a, 's> IdentityWorkerTrustReader<'a, 's> {
    /// Wrap a borrowed [`IdentityStore`].
    #[must_use]
    pub fn new(identity: &'a IdentityStore<'s>) -> Self {
        Self { identity }
    }
}

#[async_trait]
impl WorkerTrustReader for IdentityWorkerTrustReader<'_, '_> {
    async fn trust_grade(
        &self,
        tenant_id: TenantId,
        worker_id: Uuid,
    ) -> ArgoResult<Option<u8>> {
        let worker = self
            .identity
            .get_worker(tenant_id, WorkerId(worker_id))
            .await?;
        Ok(worker.map(|w| w.trust_grade))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::{Persona, Role, Worker};
    use argo_store::SessionStore;

    #[tokio::test]
    async fn returns_worker_trust_grade_when_present() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let identity = IdentityStore::new(&store);
        let tenant = TenantId::new();
        let worker = Worker::draft(
            tenant,
            "gtm",
            Persona::named("Maya", "op"),
            Role::new("Sell."),
        );
        identity.create_worker(&worker).await.unwrap();
        identity
            .set_worker_trust_grade(tenant, worker.id, 3)
            .await
            .unwrap();

        let reader = IdentityWorkerTrustReader::new(&identity);
        let grade = reader.trust_grade(tenant, worker.id.0).await.unwrap();
        assert_eq!(grade, Some(3));
    }

    #[tokio::test]
    async fn returns_none_for_unknown_worker() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let identity = IdentityStore::new(&store);
        identity.ensure_schema().await.unwrap();
        let reader = IdentityWorkerTrustReader::new(&identity);
        let grade = reader
            .trust_grade(TenantId::new(), WorkerId::new().0)
            .await
            .unwrap();
        assert_eq!(grade, None);
    }
}
