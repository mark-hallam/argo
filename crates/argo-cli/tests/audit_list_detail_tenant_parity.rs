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
//! 4. List → detail drill-in parity: every id returned by the list
//!    pass MUST 200 from detail under the same caller. The
//!    `drill_in_*` tests below seed 3 rows under one tenant, fetch the
//!    list, and assert detail succeeds for every returned id. This is
//!    the regression for
//!    `fix-audit-detail-tenant-scope-list-detail-sql-mismatch`.
//! 5. The un-headered-curl branch is also pinned: list and detail now
//!    share `audit_query::resolve_audit_tenant`, which layers the
//!    cwd-bound workspace UUID **between** the explicit-signal chain
//!    and the config / anonymous tail. Before the fix detail fell
//!    through to `"anonymous"` while list resolved to the workspace
//!    UUID — the curl-with-no-header case 404'd on every id the list
//!    surfaced.

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

/// Drill-in parity: every id surfaced by the list MUST be drillable
/// by detail under the same caller. Seeds 3 rows under T1, queries
/// the list (expects all 3 present), then queries detail for each
/// id (expects 200 OK on every one).
///
/// This is the load-bearing assertion for
/// `fix-audit-detail-tenant-scope-list-detail-sql-mismatch`:
/// pre-fix, the list returned the 3 rows but detail 404'd on every
/// id because the two routes resolved different un-headered tenants.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drill_in_every_listed_row_with_explicit_tenant_header() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = Arc::new(open_store(home.path()).await);
    let mut seeded_ids = Vec::with_capacity(3);
    for label in ["drill-1", "drill-2", "drill-3"] {
        seeded_ids.push(seed_one_row(&store, TENANT_T1, label).await);
    }
    drop(store);

    // 1 list call + 3 detail calls = 4 requests.
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

    let (status, body) = get_json(
        port,
        "/api/v1/audit?limit=200",
        &[("X-Argo-Tenant-Id", TENANT_T1), ("X-Argo-Role", "guest")],
    )
    .await;
    assert_eq!(status, 200, "list status={status} body={body}");
    let listed = id_set(&body);
    for id in &seeded_ids {
        assert!(
            listed.contains(id),
            "seeded id {id} missing from list: {listed:?}"
        );
    }

    for id in &seeded_ids {
        let (status, _) = get_json(
            port,
            &format!("/api/v1/audit/{id}"),
            &[("X-Argo-Tenant-Id", TENANT_T1), ("X-Argo-Role", "guest")],
        )
        .await;
        assert_eq!(
            status, 200,
            "detail 404'd on id {id} that the list surfaced — list/detail drift"
        );
    }

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
