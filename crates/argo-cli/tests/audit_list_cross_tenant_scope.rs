//! Regression coverage for tenant-scoped `GET /api/v1/audit`.
//!
//! The feed filters on `payload_json.tenant_id` because that is the
//! chain-authenticated source. Rows without that payload key are
//! legacy-only and must not be unioned into ordinary tenant reads.

mod common;

use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use serde_json::{json, Value};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";
const TENANT_A: &str = "ws-a";
const TENANT_B: &str = "ws-b";
const TENANT_MALICIOUS: &str = "ws-malicious";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_list_denies_legacy_union_and_cross_tenant_probe() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = Arc::new(open_store(home.path()).await);
    let a_ids = seed_audit_rows(&store, TENANT_A, "tenant-a").await;
    let b_ids = seed_audit_rows(&store, TENANT_B, "tenant-b").await;
    let legacy_id = insert_legacy_row(&store).await;
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
            "4",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let own_a = get_items(
        port,
        "/api/v1/audit?limit=200",
        &[("X-Argo-Tenant-Id", TENANT_A), ("X-Argo-Role", "guest")],
    )
    .await;
    let own_b = get_items(
        port,
        "/api/v1/audit?limit=200",
        &[("X-Argo-Tenant-Id", TENANT_B), ("X-Argo-Role", "guest")],
    )
    .await;
    let malicious = get_items(
        port,
        &format!("/api/v1/audit?as_tenant={TENANT_A}&limit=200"),
        &[
            ("X-Argo-Tenant-Id", TENANT_MALICIOUS),
            ("X-Argo-Role", "guest"),
        ],
    )
    .await;
    let legacy_admin = get_items(
        port,
        "/api/v1/audit?as_tenant=__legacy__&limit=200",
        &[("X-Argo-Tenant-Id", TENANT_A), ("X-Argo-Role", "owner")],
    )
    .await;

    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exit: {status}");

    assert!(a_ids.is_subset(&own_a), "tenant-a rows missing: {own_a:?}");
    assert!(
        b_ids.is_disjoint(&own_a),
        "tenant-a feed leaked tenant-b rows: {own_a:?}"
    );
    assert!(
        !own_a.contains(&legacy_id),
        "tenant-a feed leaked legacy row: {own_a:?}"
    );
    assert!(b_ids.is_subset(&own_b), "tenant-b rows missing: {own_b:?}");
    assert!(
        a_ids.is_disjoint(&own_b),
        "tenant-b feed leaked tenant-a rows: {own_b:?}"
    );
    assert!(
        malicious.is_empty(),
        "guest cross-tenant probe should be empty: {malicious:?}"
    );
    assert!(
        legacy_admin.contains(&legacy_id),
        "owner legacy lane did not expose legacy row: {legacy_admin:?}"
    );
    assert!(
        a_ids.is_subset(&legacy_admin) && b_ids.is_subset(&legacy_admin),
        "owner legacy lane did not expose all seeded tenant rows: {legacy_admin:?}"
    );
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

async fn seed_audit_rows(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    label: &str,
) -> HashSet<String> {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    for i in 0..2 {
        sink.write(
            None,
            AuditKind::ToolCall,
            json!({
                "action_type": label,
                "worker_id": label,
                "worker_name": label,
                "verb": format!("{label}-{i}"),
            }),
            None,
        )
        .await
        .expect("write audit row");
    }
    sink.flush_pending().await.expect("flush audit rows");
    query_ids_by_action(store, label).await
}

async fn query_ids_by_action(store: &SessionStore, action_type: &str) -> HashSet<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT lower(hex(id)) FROM audit_record \
         WHERE json_extract(payload_json, '$.action_type') = ?",
    )
    .bind(action_type)
    .fetch_all(store.pool())
    .await
    .expect("query seeded ids")
    .into_iter()
    .collect()
}

async fn insert_legacy_row(store: &SessionStore) -> String {
    let id = [0x83_u8; 16];
    let id_hex = hex::encode(id);
    let payload = json!({
        "action_type": "legacy-only",
        "worker_id": "legacy-only",
        "worker_name": "legacy-only",
        "verb": "legacy-only",
    })
    .to_string();
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_slice())
    .bind(AuditKind::ToolCall.label())
    .bind(payload)
    .bind(vec![0_u8; 32])
    .bind(vec![1_u8; 32])
    .bind(chrono::Utc::now().timestamp_micros())
    .bind("__legacy__")
    .execute(store.pool())
    .await
    .expect("insert legacy row");
    id_hex
}

async fn get_items(port: u16, path: &str, headers: &[(&str, &str)]) -> HashSet<String> {
    let client = reqwest::Client::builder().build().expect("client");
    let mut req = client.get(format!("http://127.0.0.1:{port}{path}"));
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let resp = req.send().await.expect("send");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    assert_eq!(status, 200, "GET {path}: {text}");
    let body: Value = serde_json::from_str(&text).expect("json body");
    body["items"]
        .as_array()
        .expect("items array")
        .iter()
        .map(|item| item["id"].as_str().expect("item id").to_string())
        .collect()
}
