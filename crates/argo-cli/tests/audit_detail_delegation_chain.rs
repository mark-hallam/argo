//! Regression coverage for `GET /api/v1/audit/<id>` delegation chains.
//!
//! Seeds three audit rows with the same `current_handoff_id`, boots the
//! gateway, and asserts the detail endpoint returns the originator →
//! delegated-to → executor chain in timestamp order.

mod common;

use argo_audit::AuditKind;
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use serde_json::{json, Value};
use std::process::{Command, Stdio};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";
const TENANT_ID: &str = "tenant-delegation-chain";
const HANDOFF_ID: &str = "handoff-scene-2";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_detail_returns_ordered_delegation_chain() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = open_store(home.path()).await;
    let executor_id = seed_chain(&store).await;
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

    let body = get_json(port, &format!("/api/v1/audit/{executor_id}")).await;
    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exit: {status}");

    let chain = body["detail"]["delegation_chain"]
        .as_array()
        .expect("delegation_chain array");
    assert_eq!(chain.len(), 3, "body: {body}");
    assert_eq!(chain[0]["worker_id"], "wkr-mariah");
    assert_eq!(chain[0]["role"], "originator");
    assert_eq!(chain[1]["worker_id"], "wkr-peta");
    assert_eq!(chain[1]["role"], "delegated_to");
    assert_eq!(chain[2]["worker_id"], "wkr-nori");
    assert_eq!(chain[2]["role"], "executed");
    assert_eq!(chain[2]["action_id"], executor_id);
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

async fn seed_chain(store: &SessionStore) -> String {
    let base = chrono::Utc::now().timestamp_micros();
    let steps = [
        (
            [0x31_u8; 16],
            "wkr-mariah",
            "Mariah",
            "planned customer escalation",
        ),
        (
            [0x32_u8; 16],
            "wkr-peta",
            "Peta",
            "delegated compliance check",
        ),
        (
            [0x33_u8; 16],
            "wkr-nori",
            "Nori",
            "executed compliance check",
        ),
    ];
    for (idx, (id, worker_id, worker_name, verb)) in steps.iter().enumerate() {
        insert_audit_row(
            store,
            id,
            json!({
                "current_handoff_id": HANDOFF_ID,
                "worker_id": worker_id,
                "worker_name": worker_name,
                "action_type": "delegate_task",
                "outcome_status": "succeeded",
                "verb": verb,
            }),
            base + idx as i64,
        )
        .await;
    }
    hex::encode(steps[2].0)
}

async fn insert_audit_row(store: &SessionStore, id: &[u8; 16], payload: Value, created_at: i64) {
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_slice())
    .bind(AuditKind::ToolCall.label())
    .bind(payload.to_string())
    .bind(vec![0_u8; 32])
    .bind(vec![1_u8; 32])
    .bind(created_at)
    .bind(TENANT_ID)
    .execute(store.pool())
    .await
    .expect("insert audit row");
}

async fn get_json(port: u16, path: &str) -> Value {
    let client = reqwest::Client::builder().build().expect("client");
    let resp = client
        .get(format!("http://127.0.0.1:{port}{path}"))
        .header("X-Argo-Tenant-Id", TENANT_ID)
        .send()
        .await
        .expect("send");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    assert_eq!(status, 200, "GET {path}: {text}");
    serde_json::from_str(&text).expect("json body")
}
