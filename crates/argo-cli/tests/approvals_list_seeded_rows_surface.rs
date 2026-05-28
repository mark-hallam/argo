//! Regression for `fix-approvals-list-tenant-scope-empty-despite-seeded-rows`.
//!
//! Background: `argo demo seed` writes its 8 approval rows to the
//! first-class `approval_request` table (see
//! `crates/argo-demo-seed/src/seed.rs::seed_approval_requests`). The
//! gateway endpoint `GET /api/v1/approvals` historically read only
//! from `worker_action` memory rows, so the seeded inbox rendered an
//! empty state regardless of the seed's success. This test pins the
//! bridge introduced in `routes_approvals_json_seeded.rs`: rows
//! written into `approval_request` for tenant T surface on the
//! gateway endpoint when the caller scopes to T.
//!
//! It also pins that the cross-tenant invisibility property held by
//! `claim_6_cross_tenant_approval_select_no_existence_leak` (#2113)
//! stays closed — a different tenant's rows do NOT leak in.

mod common;

use std::process::{Command, Stdio};
use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_store::approvals::{create_request, DefaultDecision, NewApprovalRequest};
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use argo_types::TenantId;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, gateway_lock_guard, pick_free_port};
use serde_json::{json, Value};
use uuid::Uuid;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_surfaces_seeded_approval_request_rows() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = Arc::new(open_store(home.path()).await);

    let tenant_t1 = TenantId(Uuid::now_v7());
    let tenant_t2 = TenantId(Uuid::now_v7());
    seed_approval_rows(&store, tenant_t1, 5, "ws-team@t1").await;
    seed_approval_rows(&store, tenant_t2, 2, "ws-team@t2").await;
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
            "2",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    // Pass 1: T1 caller sees their 5 seeded rows.
    let (status, body) = get_json(
        port,
        "/api/v1/approvals?limit=100",
        &[
            ("X-Tenant-Id", &tenant_t1.to_string()),
            ("X-Argo-Role", "user"),
        ],
    )
    .await;
    assert_eq!(status, 200, "T1 approvals list: status={status} body={body}");
    let items = body["items"].as_array().expect("items array");
    assert_eq!(
        items.len(),
        5,
        "T1 caller expected 5 seeded rows, got {}: {body}",
        items.len()
    );

    // Pass 2: T2 caller sees only T2's 2 rows — cross-tenant rows MUST
    // NOT leak (regression for claim_6 cross-tenant invisibility).
    let (status, body) = get_json(
        port,
        "/api/v1/approvals?limit=100",
        &[
            ("X-Tenant-Id", &tenant_t2.to_string()),
            ("X-Argo-Role", "user"),
        ],
    )
    .await;
    assert_eq!(status, 200, "T2 approvals list: status={status} body={body}");
    let items = body["items"].as_array().expect("items array");
    assert_eq!(
        items.len(),
        2,
        "T2 caller expected 2 seeded rows, got {} — cross-tenant leak? body={body}",
        items.len()
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

async fn seed_approval_rows(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    count: usize,
    requested_by: &str,
) {
    let deadline_at = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros();
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    for i in 0..count {
        let action_id = Uuid::now_v7().as_bytes().to_vec();
        let summary = format!("seed row {i}");
        let new_req = NewApprovalRequest {
            action_id,
            tenant_id,
            requested_by,
            deadline_at,
            default_decision: DefaultDecision::Deny,
            draft_content: Some(&summary),
        };
        let approval_id = create_request(store.pool(), &new_req)
            .await
            .expect("create approval_request");
        // Match the seed's pattern: also write the matching ApprovalAsked
        // audit row so the gateway can derive channel + worker context.
        sink.write(
            None,
            AuditKind::ApprovalAsked,
            json!({
                "approval_id": approval_id,
                "worker_id": Uuid::now_v7().to_string(),
                "label": "test action",
                "summary": summary,
                "channel": "slack",
            }),
            None,
        )
        .await
        .expect("write approval.asked audit row");
    }
    sink.flush_pending().await.expect("flush audit rows");
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
