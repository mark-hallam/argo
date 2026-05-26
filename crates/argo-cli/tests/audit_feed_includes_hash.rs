//! Regression coverage for hash fields in `GET /api/v1/audit`.

mod common;

use std::process::{Command, Stdio};

use argo_audit::{compute_chain_hash, AuditKind};
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use serde_json::{json, Value};
use uuid::Uuid;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";
const TENANT_ID: &str = "ws-default";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_feed_projects_persisted_self_and_parent_hashes() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = open_store(home.path()).await;
    let seeded = insert_hash_row(&store).await;
    drop(store);

    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env("ANTHROPIC_API_KEY", "sk-ant-test-only-not-real")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let body = get_audit_feed(port).await;
    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exit: {status}");

    let item = body["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|item| item["id"].as_str() == Some(seeded.id.as_str()))
        .unwrap_or_else(|| panic!("seeded row missing from feed: {body}"));
    assert_eq!(item["self_hash"], seeded.self_hash);
    assert_eq!(item["parent_hash"], seeded.parent_hash);
    assert_eq!(item["self_hash"].as_str().expect("self_hash").len(), 64);
    assert_eq!(item["parent_hash"].as_str().expect("parent_hash").len(), 64);
}

async fn open_store(home: &std::path::Path) -> SessionStore {
    let hex = argo_store::encryption::base64_to_hex(SMOKE_DB_KEY_B64).expect("hex key");
    let store = SessionStore::open_with_hex_key(
        home.join("sessions.db"),
        WalTuningProfile::default(),
        &hex,
    )
    .await
    .expect("open encrypted store");
    store.migrate().await.expect("migrate store");
    store
}

struct SeededHashRow {
    id: String,
    parent_hash: String,
    self_hash: String,
}

async fn insert_hash_row(store: &SessionStore) -> SeededHashRow {
    let id = Uuid::now_v7();
    let id_hex = hex::encode(id.as_bytes());
    let kind = AuditKind::ToolCall.label();
    let parent_hash = [0x4d_u8; 32];
    let payload = json!({
        "tenant_id": TENANT_ID,
        "worker_id": "wkr_hash_regression",
        "worker_name": "Hash Regression",
        "action_type": "hash_regression",
        "verb": "persisted self hash was projected",
        "outcome_status": "succeeded",
        "channel": "web",
        "data_classification": "internal",
    });
    let canonical = serde_json::to_string(&payload).expect("canonical payload");
    let self_hash = compute_chain_hash(&parent_hash, kind, &canonical);
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().to_vec())
    .bind(kind)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(chrono::Utc::now().timestamp_micros())
    .bind(TENANT_ID)
    .execute(store.pool())
    .await
    .expect("insert hash row");

    SeededHashRow {
        id: id_hex,
        parent_hash: hex::encode(parent_hash),
        self_hash: hex::encode(self_hash),
    }
}

async fn get_audit_feed(port: u16) -> Value {
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/api/v1/audit?limit=10"))
        .header("X-Argo-Tenant-Id", TENANT_ID)
        .header("X-Argo-Role", "guest")
        .send()
        .await
        .expect("send");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    assert_eq!(status, 200, "audit feed response: {text}");
    serde_json::from_str(&text).expect("json body")
}
