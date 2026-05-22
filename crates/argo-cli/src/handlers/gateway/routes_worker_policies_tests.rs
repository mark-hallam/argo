//! Unit tests for `routes_worker_policies` — pure helpers only.
//!
//! The full HTTP path (registry lookup, identity-store, audit sink)
//! is exercised by the gateway smoke tests in
//! `crates/argo-cli/tests/`; these tests pin the body-parsing,
//! TOML-roundtrip, and policy-vocabulary contracts so a typo in
//! the wire schema surfaces immediately.

use super::*;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn id() -> WorkerId {
    "00000000-0000-0000-0000-000000000001"
        .parse::<WorkerId>()
        .expect("valid test worker_id")
}

#[test]
fn parse_patch_body_accepts_valid_input() {
    let body = r#"{"policy":"auto-approve","operator_note":"trust ladder hit"}"#;
    let parsed = parse_patch_body(body).expect("valid body parses");
    assert_eq!(parsed.policy, "auto-approve");
    assert_eq!(parsed.operator_note, "trust ladder hit");
}

#[test]
fn parse_patch_body_rejects_unknown_policy() {
    let body = r#"{"policy":"please","operator_note":"hi"}"#;
    let err = parse_patch_body(body).expect_err("unknown policy rejects");
    assert!(err.contains("unknown policy"), "got: {err}");
    assert!(
        err.contains("auto-approve") && err.contains("risk-gated"),
        "error should enumerate the canonical vocabulary: {err}"
    );
}

#[test]
fn parse_patch_body_requires_non_empty_operator_note() {
    let missing = r#"{"policy":"always-ask"}"#;
    let e1 = parse_patch_body(missing).expect_err("missing note rejects");
    assert!(e1.contains("operator_note"), "got: {e1}");

    let blank = r#"{"policy":"always-ask","operator_note":"   "}"#;
    let e2 = parse_patch_body(blank).expect_err("whitespace-only note rejects");
    assert!(e2.contains("non-empty"), "got: {e2}");
}

#[test]
fn read_policies_returns_empty_when_file_absent() {
    let dir = tempdir().expect("tempdir");
    let overrides = read_policies(dir.path(), id()).expect("missing file is not an error");
    assert!(overrides.is_empty());
}

#[test]
fn write_then_read_round_trips_the_policy_table() {
    let dir = tempdir().expect("tempdir");
    let worker = id();
    let mut overrides = BTreeMap::new();
    overrides.insert("draft_listing".to_string(), "auto-approve".to_string());
    overrides.insert("post_listing".to_string(), "always-ask".to_string());
    write_policies(dir.path(), worker, &overrides).expect("write");

    let path = worker_policy_path(dir.path(), worker);
    assert!(path.exists(), "file written at {}", path.display());

    let round = read_policies(dir.path(), worker).expect("read");
    assert_eq!(
        round.get("draft_listing"),
        Some(&"auto-approve".to_string())
    );
    assert_eq!(round.get("post_listing"), Some(&"always-ask".to_string()));
    assert_eq!(round.len(), 2);
}

#[test]
fn read_policies_filters_out_unknown_policy_values() {
    // Operator-edited TOML on disk may carry stale or hand-written
    // values. The reader keeps the valid arms and drops the rest so
    // the UI never surfaces a value it can't render.
    let dir = tempdir().expect("tempdir");
    let worker = id();
    let path = worker_policy_path(dir.path(), worker);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "[policies]\ngood = \"auto-approve\"\nbad = \"yolo\"\n",
    )
    .unwrap();
    let overrides = read_policies(dir.path(), worker).expect("read");
    assert_eq!(overrides.get("good"), Some(&"auto-approve".to_string()));
    assert!(!overrides.contains_key("bad"), "unknown value dropped");
}

/// `mission-cross-tenant-worker-handler-no-existence-leak`: cross-tenant
/// existence is hidden from the HTTP response. Both `list_response`
/// (GET /policies) and `update_response` (PATCH /policies/<action_type>)
/// must return byte-identical 404 for the cross-tenant and
/// nonexistent branches.
mod cross_tenant_regression {
    use super::super::{list_response, update_response};
    use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
    use argo_store::SessionStore;
    use argo_types::TenantId;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::{Mutex, MutexGuard};

    async fn env_lock() -> MutexGuard<'static, ()> {
        static M: Mutex<()> = Mutex::const_new(());
        M.lock().await
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .to_path_buf()
    }

    fn set_test_env(tmp: &std::path::Path) {
        std::env::set_var("ARGO_HOME", tmp.join("home"));
        std::fs::create_dir_all(tmp.join("home")).expect("home dir");
        std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
    }

    async fn mem_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.expect("open mem db");
        store.migrate().await.expect("migrate");
        Arc::new(store)
    }

    async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
        let cwd = std::env::current_dir().expect("cwd");
        let name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace");
        let (tenant, _) = store
            .ensure_workspace(name, &cwd.to_string_lossy())
            .await
            .expect("ensure workspace");
        tenant
    }

    async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
        let worker = Worker::draft(
            tenant,
            "support",
            Persona::named(name, "human:mark"),
            Role::new("test."),
        );
        IdentityStore::new(store)
            .create_worker(&worker)
            .await
            .expect("create worker");
        worker
    }

    fn get_req(id: &WorkerId) -> String {
        format!("GET /api/v1/workers/{id}/policies HTTP/1.1\r\nHost: x\r\n\r\n")
    }

    fn patch_req(id: &WorkerId, action: &str, body: &str) -> String {
        let len = body.len();
        format!(
            "PATCH /api/v1/workers/{id}/policies/{action} HTTP/1.1\r\nHost: x\r\nContent-Length: {len}\r\n\r\n{body}"
        )
    }

    #[tokio::test]
    async fn cross_tenant_worker_policies_returns_same_404_as_unknown_worker() {
        let _g = env_lock().await;
        let tmp = tempfile::tempdir().expect("tempdir");
        set_test_env(tmp.path());
        let store = mem_store().await;
        let _ = workspace_tenant(&store).await;
        let other = TenantId::new();
        let alien = add_worker(&store, other, "Alien").await;
        let ghost = WorkerId::new();
        let req_alien = get_req(&alien.id);
        let req_ghost = get_req(&ghost);
        let (status_alien, body_alien) =
            list_response(&alien.id.to_string(), &req_alien, Some(&store)).await;
        let (status_ghost, body_ghost) =
            list_response(&ghost.to_string(), &req_ghost, Some(&store)).await;
        assert_eq!(status_alien, "404 Not Found", "body: {body_alien}");
        assert_eq!(status_alien, status_ghost);
        assert_eq!(
            body_alien, body_ghost,
            "GET /policies: cross-tenant body must equal unknown-worker body"
        );
        assert!(!body_alien.contains(&alien.id.to_string()));
    }

    #[tokio::test]
    async fn cross_tenant_worker_policy_update_returns_same_404_as_unknown_worker() {
        let _g = env_lock().await;
        let tmp = tempfile::tempdir().expect("tempdir");
        set_test_env(tmp.path());
        let store = mem_store().await;
        let _ = workspace_tenant(&store).await;
        let other = TenantId::new();
        let alien = add_worker(&store, other, "Alien").await;
        let ghost = WorkerId::new();
        let body = r#"{"policy":"auto-approve","operator_note":"why"}"#;
        let req_alien = patch_req(&alien.id, "draft_listing", body);
        let req_ghost = patch_req(&ghost, "draft_listing", body);
        let (status_alien, body_alien) = update_response(
            &alien.id.to_string(),
            "draft_listing",
            &req_alien,
            Some(&store),
        )
        .await;
        let (status_ghost, body_ghost) =
            update_response(&ghost.to_string(), "draft_listing", &req_ghost, Some(&store)).await;
        assert_eq!(status_alien, "404 Not Found", "body: {body_alien}");
        assert_eq!(status_alien, status_ghost);
        assert_eq!(
            body_alien, body_ghost,
            "PATCH /policies/<action>: cross-tenant body must equal unknown-worker body"
        );
        assert!(!body_alien.contains(&alien.id.to_string()));
    }
}
