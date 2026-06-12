//! WS1 acceptance for `fix-worker-read-routes-anon-tenant-override` at the
//! `GET /api/v1/workers/<id>` route level. Sibling of
//! `routes_worker_detail_tests.rs` (which sits at the 500-LOC ceiling);
//! self-contained fixtures, same env discipline.

use super::routes_worker_detail::show_response;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

#[tokio::test]
async fn anonymous_tenant_override_cannot_read_other_tenants_worker_in_production() {
    // An UNAUTHENTICATED `GET /api/v1/workers/<B-worker>?tenant=<B>` in
    // production mode must NOT return tenant B's worker config. Pre-fix
    // the resolver honored the override and this returned 200 + config.
    let _g = super::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    std::env::set_var("ARGO_HOME", tmp.path().join("home"));
    std::fs::create_dir_all(tmp.path().join("home")).expect("home dir");
    std::env::set_var("ARGO_GATEWAY_PROFILE", "production");

    let store = mem_store().await;
    // Victim tenant B is NOT the cwd workspace tenant.
    let (tenant_b, _ws) = store
        .ensure_workspace("victim-beta", "/tmp/victim-beta-root")
        .await
        .expect("tenant b");
    let worker = Worker::draft(
        tenant_b,
        "support",
        Persona::named("Sensitive Worker", "human:victim"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(&store)
        .create_worker(&worker)
        .await
        .expect("create worker");

    let id = worker.id.to_string();
    let req = format!(
        "GET /api/v1/workers/{id}?tenant={tenant_b} HTTP/1.1\r\nHost: x\r\n\r\n"
    );
    let (status, body) = show_response(&id, &req, Some(&store)).await;
    std::env::remove_var("ARGO_GATEWAY_PROFILE");

    assert!(
        !status.starts_with("200"),
        "cross-tenant anonymous read must not succeed, got {status}: {body}"
    );
    assert!(
        !body.contains("Sensitive Worker"),
        "victim worker data must not leak: {body}"
    );
}
