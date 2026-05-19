//! `SessionStore::set_routing_policy` writer for the
//! `session.routing_policy` column added in slot 117 (Stream
//! `mission-claim-3-runtime-wiring-v1x`). Lives in its own file so the
//! main `impl SessionStore` blocks stay under the 200-LOC ceiling and
//! the writer can grow companion helpers (validation, decoding) in
//! one place.
use crate::{SessionStore, StoreResult};
use argo_types::{ArgoError, RoutingPolicy, SessionId};

impl SessionStore {
    /// Set (or clear) the routing policy on a session row.
    ///
    /// `Some(policy)` serialises the policy as JSON and writes it to
    /// `session.routing_policy`; `None` clears the column back to
    /// SQL NULL so the agent loop falls back to `session.model`.
    ///
    /// Returns `Ok(())` even when the session id has no row — the
    /// caller is responsible for checking existence beforehand if
    /// it cares about that distinction (mirrors
    /// `set_session_worker_id`'s no-op-on-missing semantics minus
    /// the rows-affected return).
    pub async fn set_routing_policy(
        &self,
        session_id: SessionId,
        policy: Option<&RoutingPolicy>,
    ) -> StoreResult<()> {
        let json = policy
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| ArgoError::Schema(format!("routing_policy encode: {e}")))?;
        sqlx::query("UPDATE session SET routing_policy = ? WHERE id = ?")
            .bind(json)
            .bind(session_id.0.as_bytes().as_slice())
            .execute(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("set_routing_policy: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::SessionStore;
    use argo_types::{ProviderModel, RoutingPolicy, TieredRouting};

    async fn fixture() -> (SessionStore, argo_types::SessionId) {
        let store = SessionStore::open(":memory:").await.expect("open");
        store.migrate().await.expect("migrate");
        let (tenant, workspace) = store
            .ensure_workspace("t", "/tmp/x")
            .await
            .expect("workspace");
        let sid = store
            .create_session(tenant, workspace, "m", "p", None, None)
            .await
            .expect("create");
        (store, sid)
    }

    fn sample_tiered() -> RoutingPolicy {
        let model = ProviderModel {
            provider: "p".into(),
            model: "m".into(),
        };
        RoutingPolicy::Tiered(TieredRouting {
            classifier_model: model.clone(),
            simple: model.clone(),
            medium: model.clone(),
            complex: model,
            classify_threshold_ms: 250,
            fallback_to_default_on_error: true,
        })
    }

    #[tokio::test]
    async fn set_then_read_round_trips() {
        let (store, sid) = fixture().await;
        store
            .set_routing_policy(sid, Some(&sample_tiered()))
            .await
            .expect("set");
        let header = store.get_session(sid).await.expect("get").expect("row");
        let policy = header.routing_policy.expect("policy present");
        assert!(matches!(policy, RoutingPolicy::Tiered(_)));
    }

    #[tokio::test]
    async fn set_none_clears_policy() {
        let (store, sid) = fixture().await;
        store
            .set_routing_policy(sid, Some(&sample_tiered()))
            .await
            .expect("set");
        store.set_routing_policy(sid, None).await.expect("clear");
        let header = store.get_session(sid).await.expect("get").expect("row");
        assert!(header.routing_policy.is_none());
    }
}
