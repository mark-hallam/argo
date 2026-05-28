//! Tenant-resolver parity between `GET /api/v1/audit` (the feed) and
//! `GET /api/v1/audit/<id>` (the detail).
//!
//! Regression for `fix-gateway-audit-detail-tenant-resolver-drift-vs-feed`
//! (UX e2e audit 2026-05-28-0252, scene-1-and-2 F02 + scene-6 F2): the
//! two routes used to pull the caller's tenant from different headers
//! with different defaults, so the feed could return a row id that
//! the detail then 404'd on for the same SPA session. Both routes now
//! share `super::rate_limit::extract_tenant` (canonical:
//! `?tenant=` → `X-Tenant-Id` → `X-Argo-Tenant-Id` legacy alias →
//! config default → `"anonymous"`).
//!
//! What this pins:
//!
//! 1. With `X-Argo-Tenant-Id: T1` set, the feed returns T1's row and
//!    detail returns 200 for that id — the same header value drives
//!    both routes via the shared resolver.
//! 2. With the same header set to T1, detail returns 404 for the row
//!    seeded under T2 — cross-tenant probe stays denied.
//! 3. With `X-Argo-Tenant-Id: T2` set, the feed returns T2's row and
//!    detail returns 200 — both routes flip with the header in lockstep.
//!
//! The "no tenant header set" branch is intentionally not asserted
//! here: the feed's filter still uses `resolve_tenant_scope`'s
//! `ws-default` fallback while detail now uses the canonical chain's
//! `argo_config.tenant.default_id` → `"anonymous"` fallback. Aligning
//! the un-headered defaults is the next brief's job (out-of-scope
//! `fix-spa-send-tenant-from-session-jwt`). The bug this brief closes
//! is the per-call drift WHEN A HEADER IS PRESENT (or the SPA settles
//! on a single canonical default upstream).

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
const TENANT_T1: &str = "ws-t1";
const TENANT_T2: &str = "ws-t2";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_list_and_detail_share_one_tenant_resolver() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = Arc::new(open_store(home.path()).await);
    let t1_id = seed_one_row(&store, TENANT_T1, "worker-t1").await;
    let t2_id = seed_one_row(&store, TENANT_T2, "worker-t2").await;
    drop(store);

    // Six requests below; the gateway exits after that count is served.
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
            "6",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    // ─── Pass 1: X-Argo-Tenant-Id: T1 ────────────────────────────────
    let (status, body) = get_json(
        port,
        "/api/v1/audit?limit=200",
        &[("X-Argo-Tenant-Id", TENANT_T1), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(status, 200, "T1 feed: status={status} body={body}");
    let feed_t1_ids = id_set(&body);
    assert!(
        feed_t1_ids.iter().any(|id| id == &t1_id),
        "T1-scoped feed missing T1 row: {feed_t1_ids:?}"
    );
    assert!(
        !feed_t1_ids.iter().any(|id| id == &t2_id),
        "T1-scoped feed leaked T2 row: {feed_t1_ids:?}"
    );

    // The feed surfaced t1_id; detail must resolve it without 404 —
    // this is the parity invariant that was broken before the fix.
    let (status, _) = get_json(
        port,
        &format!("/api/v1/audit/{t1_id}"),
        &[("X-Argo-Tenant-Id", TENANT_T1), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(
        status, 200,
        "T1-scoped detail 404'd on id {t1_id} that T1-scoped feed surfaced"
    );

    // Same caller probes T2's row — cross-tenant 404 stays denied.
    let (status, _) = get_json(
        port,
        &format!("/api/v1/audit/{t2_id}"),
        &[("X-Argo-Tenant-Id", TENANT_T1), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(
        status, 404,
        "T1-scoped detail leaked T2 row id={t2_id} — cross-tenant probe must 404"
    );

    // ─── Pass 2: X-Argo-Tenant-Id: T2 (resolver flips with header) ───
    let (status, body) = get_json(
        port,
        "/api/v1/audit?limit=200",
        &[("X-Argo-Tenant-Id", TENANT_T2), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(status, 200, "T2 feed: status={status} body={body}");
    let feed_t2_ids = id_set(&body);
    assert!(
        feed_t2_ids.iter().any(|id| id == &t2_id),
        "T2-scoped feed missing T2 row: {feed_t2_ids:?}"
    );
    assert!(
        !feed_t2_ids.iter().any(|id| id == &t1_id),
        "T2-scoped feed leaked T1 row: {feed_t2_ids:?}"
    );

    let (status, _) = get_json(
        port,
        &format!("/api/v1/audit/{t2_id}"),
        &[("X-Argo-Tenant-Id", TENANT_T2), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(
        status, 200,
        "T2-scoped detail 404'd on id {t2_id} that T2-scoped feed surfaced"
    );

    let (status, _) = get_json(
        port,
        &format!("/api/v1/audit/{t1_id}"),
        &[("X-Argo-Tenant-Id", TENANT_T2), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(
        status, 404,
        "T2-scoped detail leaked T1 row id={t1_id} — cross-tenant probe must 404"
    );

    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exit: {status}");
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

async fn seed_one_row(store: &Arc<SessionStore>, tenant_id: &str, label: &str) -> String {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    sink.write(
        None,
        AuditKind::ToolCall,
        json!({
            "action_type": label,
            "worker_id": label,
            "worker_name": label,
            "verb": format!("{label}-action"),
        }),
        None,
    )
    .await
    .expect("write audit row");
    sink.flush_pending().await.expect("flush audit row");
    let row_id: Vec<u8> = sqlx::query_scalar(
        "SELECT id FROM audit_record \
         WHERE json_extract(payload_json, '$.action_type') = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(label)
    .fetch_one(store.pool())
    .await
    .expect("read back row id");
    hex::encode(row_id)
}

fn id_set(body: &Value) -> Vec<String> {
    body["items"]
        .as_array()
        .expect("items array")
        .iter()
        .map(|item| item["id"].as_str().expect("item id").to_string())
        .collect()
}

async fn get_json(port: u16, path: &str, headers: &[(&str, &str)]) -> (u16, Value) {
    let client = reqwest::Client::builder().build().expect("client");
    let mut req = client.get(format!("http://127.0.0.1:{port}{path}"));
    for (name, value) in headers {
        req = req.header(*name, *value);
    }
    let resp = req.send().await.expect("send");
    let status = resp.status().as_u16();
    let text = resp.text().await.expect("body");
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, body)
}
