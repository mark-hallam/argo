//! Regression coverage for the audit feed/detail worker-name join.
//!
//! Closes UX-AUDIT scene-6 F7: before this brief, the audit-feed
//! serializer read `worker_name` straight from the audit payload —
//! which, on every demo-live row, was either absent or back-filled
//! with the worker's UUID, so the `/audit` page rendered raw UUID
//! strings instead of "Mariah" / "Peta" / …
//!
//! These tests pin the new behaviour: the response's `worker.name`
//! comes from the live `worker`/`persona` registry, tenant-scoped,
//! falling back to the UUID only when no worker row exists for the
//! `(tenant_id, worker_id)` pair (deletion or never-existed).

mod common;

use std::process::{Command, Stdio};
use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use serde_json::{json, Value};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";
const TENANT_A: &str = "ws-name-join-a";
const TENANT_B: &str = "ws-name-join-b";
const WKR_MARIAH: &str = "wkr-mariah-0000-0000-0000-000000000001";
const WKR_GHOST: &str = "wkr-ghost-0000-0000-0000-0000000000002";
const WKR_CROSS: &str = "wkr-cross-0000-0000-0000-0000000000003";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_feed_resolves_worker_name_from_registry_with_tenant_guard() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = Arc::new(open_store(home.path()).await);

    // Tenant-A: live worker "Mariah".
    seed_worker(&store, TENANT_A, WKR_MARIAH, "Mariah").await;
    // Tenant-B: same-name worker — must NOT leak into Tenant-A's response.
    seed_worker(&store, TENANT_B, WKR_CROSS, "OtherTenantMariah").await;

    // Audit row pointing at the live worker — expect "Mariah".
    seed_audit_row(&store, TENANT_A, WKR_MARIAH).await;
    // Audit row pointing at a worker that exists only in Tenant-B — expect UUID fallback.
    seed_audit_row(&store, TENANT_A, WKR_CROSS).await;
    // Audit row pointing at a worker that doesn't exist anywhere — expect UUID fallback.
    seed_audit_row(&store, TENANT_A, WKR_GHOST).await;

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

    let body = get_audit_feed(port, TENANT_A).await;
    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exit: {status}");

    let items = body["items"].as_array().expect("items array");
    let by_worker_id = |id: &str| -> Value {
        items
            .iter()
            .find(|it| it["worker"]["id"].as_str() == Some(id))
            .cloned()
            .unwrap_or_else(|| panic!("no item for worker_id {id}: {body}"))
    };

    let live = by_worker_id(WKR_MARIAH);
    assert_eq!(
        live["worker"]["name"], "Mariah",
        "live worker should resolve to persona name, got: {live}"
    );

    let cross = by_worker_id(WKR_CROSS);
    assert_eq!(
        cross["worker"]["name"],
        WKR_CROSS,
        "cross-tenant worker name must NOT leak; should fall back to UUID, got: {cross}"
    );

    let ghost = by_worker_id(WKR_GHOST);
    assert_eq!(
        ghost["worker"]["name"],
        WKR_GHOST,
        "missing worker should fall back to UUID, got: {ghost}"
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

/// Insert directly into the `worker` + `persona` tables — bypasses
/// the heavier [`argo_identity::IdentityStore::create_worker`] path
/// (extra invariants around role/duty/budget rows) so the test
/// stays narrow to the join semantics being exercised.
async fn seed_worker(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    worker_id: &str,
    name: &str,
) {
    let persona_id = format!("persona-{worker_id}");
    sqlx::query(
        "INSERT INTO persona \
         (id, worker_id, tenant_id, name, voice_preamble, avatar, bio, \
          builder_attribution, provenance_chain_json) \
         VALUES (?, ?, ?, ?, '', NULL, NULL, 'test', '[]')",
    )
    .bind(&persona_id)
    .bind(worker_id)
    .bind(tenant_id)
    .bind(name)
    .execute(store.pool())
    .await
    .expect("insert persona");
    sqlx::query(
        "INSERT INTO worker \
         (id, tenant_id, vertical_id, persona_id, role_id, memory_scope, \
          auto_approval_json, channels_json, created_at) \
         VALUES (?, ?, 'test-vertical', ?, ?, 'shared', '[]', '[]', ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(&persona_id)
    .bind(format!("worker:{worker_id}"))
    .bind(chrono::Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .expect("insert worker");
}

async fn seed_audit_row(store: &Arc<SessionStore>, tenant_id: &str, worker_id: &str) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let payload = json!({
        "worker_id": worker_id,
        // Deliberately omit `worker_name` — the bug being closed is
        // that the serializer used to back-fill this slot with the
        // UUID itself; the resolver must source the name from the
        // worker registry, not from the embedded payload.
        "action_type": "test.event",
        "channel": "web",
        "outcome_status": "succeeded",
        "verb": format!("event for {worker_id}"),
        "data_classification": "internal",
    });
    sink.write(None, AuditKind::ToolCall, payload, None)
        .await
        .expect("write audit row");
    sink.flush_pending().await.expect("flush audit row");
}

async fn get_audit_feed(port: u16, tenant_id: &str) -> Value {
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/api/v1/audit?limit=20"))
        .header("X-Argo-Tenant-Id", tenant_id)
        .header("X-Argo-Role", "owner")
        .send()
        .await
        .expect("send");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    assert_eq!(status, 200, "audit feed response: {text}");
    serde_json::from_str(&text).expect("json body")
}
