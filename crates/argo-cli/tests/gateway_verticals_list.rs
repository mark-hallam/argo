//! Integration coverage for `GET /api/v1/verticals` discovery.
mod common;

use argo_store::workflow_definition as wd;
use argo_store::SessionStore;
use argo_types::TenantId;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, pick_free_port};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

const TOML: &str = r#"
schema_version = "1"

[workflow]
name = "seed"

[[step]]
name = "scan"
kind = "skill_invoke"
skill = "reviewer"
"#;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

#[tokio::test]
async fn gateway_verticals_list_is_tenant_scoped() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("tempdir");
    let hex = argo_store::encryption::base64_to_hex(SMOKE_DB_KEY_B64).expect("hex key");
    let store = SessionStore::open_with_hex_key(
        home.path().join("sessions.db"),
        argo_store::wal_tuning::WalTuningProfile::default(),
        &hex,
    )
    .await
    .expect("open encrypted store");
    store.migrate().await.expect("migrate");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    seed_head(&store, tenant_a, "support", "triage").await;
    seed_head(&store, tenant_a, "support", "handoff").await;
    seed_head(&store, tenant_b, "sales", "prospect").await;
    drop(store);

    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "2",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let resp_a = http_get_tenant(port, "/api/v1/verticals", tenant_a);
    assert!(resp_a.contains("HTTP/1.1 200 OK"), "tenant A resp={resp_a}");
    let body_a: Value = serde_json::from_str(body_of(&resp_a)).expect("tenant A json");
    let rows_a = body_a["verticals"].as_array().expect("tenant A rows");
    assert_eq!(rows_a.len(), 1, "tenant A body={body_a}");
    assert_eq!(rows_a[0]["id"], "support");
    assert_eq!(rows_a[0]["workflow_count"], 2);

    let resp_b = http_get_tenant(port, "/api/v1/verticals", tenant_b);
    assert!(resp_b.contains("HTTP/1.1 200 OK"), "tenant B resp={resp_b}");
    let body_b: Value = serde_json::from_str(body_of(&resp_b)).expect("tenant B json");
    let rows_b = body_b["verticals"].as_array().expect("tenant B rows");
    assert_eq!(rows_b.len(), 1, "tenant B body={body_b}");
    assert_eq!(rows_b[0]["id"], "sales");
    assert_eq!(rows_b[0]["workflow_count"], 1);
    assert_ne!(rows_b[0]["id"], "support", "tenant A leaked: {body_b}");

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}

async fn seed_head(store: &SessionStore, tenant: TenantId, vertical: &str, name: &str) {
    let id = wd::create(store.pool(), tenant, vertical, name, TOML, None, "test")
        .await
        .expect("create workflow");
    wd::set_head(store.pool(), tenant, vertical, name, &id)
        .await
        .expect("set head");
}

fn http_get_tenant(port: u16, path: &str, tenant: TenantId) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nX-Argo-Tenant-Id: {tenant}\r\n\
         Connection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}
