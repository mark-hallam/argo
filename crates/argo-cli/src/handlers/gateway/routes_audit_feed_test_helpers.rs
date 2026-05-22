//! Shared helpers for the `routes_audit_feed_*_tests.rs` siblings.
//!
//! Each `routes_audit_feed_*_tests.rs` is its own `#[cfg(test)] mod`
//! under `routes_audit_feed.rs`. The helpers live here so the query,
//! pagination, and tenant sibling files can reach them via
//! `use super::routes_audit_feed_test_helpers::*;`.

#![allow(dead_code)]
#![allow(deprecated)]

use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

pub(super) async fn seed_row(store: &Arc<SessionStore>, payload: Value) {
    AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::HttpRequest, payload, None)
        .await
        .expect("write audit row");
}

/// Standard row shape carrying every dimension the feed filters on.
#[allow(clippy::too_many_arguments)]
pub(super) fn row(
    worker_id: &str,
    channel: &str,
    action_type: &str,
    status: &str,
    approver_id: Option<&str>,
    cost_usd: Option<f64>,
    classification: &str,
    verb: &str,
) -> Value {
    let mut v = json!({
        "endpoint": "test.action",
        "worker_id": worker_id,
        "worker_name": format!("Worker-{worker_id}"),
        "channel": channel,
        "action_type": action_type,
        "outcome_status": status,
        "data_classification": classification,
        "verb": verb,
    });
    if let Some(a) = approver_id {
        v["approver_id"] = json!(a);
        v["approver_name"] = json!(format!("Approver-{a}"));
    }
    if let Some(c) = cost_usd {
        v["cost_usd"] = json!(c);
    }
    v
}

pub(super) fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

pub(super) async fn parse_body(resp: (&str, String)) -> (&str, Value) {
    let v: Value =
        serde_json::from_str(&resp.1).unwrap_or_else(|_| panic!("invalid JSON body: {}", resp.1));
    (resp.0, v)
}

pub(super) async fn seed_minimal_two(store: &Arc<SessionStore>) {
    seed_row(
        store,
        row(
            "w1",
            "web",
            "approval.decided",
            "succeeded",
            Some("a1"),
            Some(0.5),
            "internal",
            "approved transfer",
        ),
    )
    .await;
    seed_row(
        store,
        row(
            "w2",
            "slack",
            "message.sent",
            "failed",
            None,
            Some(0.001),
            "public",
            "sent message to channel",
        ),
    )
    .await;
}

pub(super) async fn seed_row_for_tenant(store: &Arc<SessionStore>, tenant_id: &str, worker: &str) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    sink.write(
        None,
        AuditKind::HttpRequest,
        row(
            worker,
            "web",
            "msg",
            "succeeded",
            None,
            None,
            "internal",
            "did x",
        ),
        None,
    )
    .await
    .expect("write tenant-scoped row");
    // Drop the batched-writer latency-window before the test reads back. The
    // sink-per-helper-call pattern queues into a per-sink batch that would
    // otherwise be lost when the sink drops at end-of-scope.
    sink.flush_pending().await.expect("flush tenant-scoped row");
}

pub(super) fn get_with_tenant(path: &str, tenant: &str) -> String {
    // `X-Argo-Role: user` proves the role-gate denies cross-tenant
    // for non-owners. The previous shape relied on header absence
    // implying "no cap"; the role-gate's dev-default is Owner, so
    // the explicit non-owner role is what now signals intent.
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\nX-Argo-Role: user\r\n\r\n"
    )
}

pub(super) fn get_with_tenant_and_cap(path: &str, tenant: &str) -> String {
    // `X-Argo-Role: owner` mirrors the in-prod path where a JWT
    // role-claim of owner unlocks `Action::AuditCrossTenant`.
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\nX-Argo-Role: owner\r\n\r\n"
    )
}

/// Seed one platform-event audit row (e.g. `WorkerHired`,
/// `WorkflowHeadChanged`, `WorkflowRolledBack`) under `tenant_id` with
/// the payload shape the writer emits in
/// `routes_workers_hire.rs` / `routes_workflows_handlers.rs`.
///
/// `extra` is merged into the base payload after `tenant_id` is
/// stamped, so callers can attach `worker_id`, `vertical_id`,
/// `workflow_name`, etc. without having to duplicate the shared keys.
pub(super) async fn seed_platform_event(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    kind: AuditKind,
    extra: Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let mut payload = json!({
        "actor": "human:platform-event-test",
        "tenant_id": tenant_id,
    });
    if let Value::Object(map) = extra {
        for (k, v) in map {
            payload[k] = v;
        }
    }
    sink.write(None, kind, payload, Some("human:platform-event-test".into()))
        .await
        .expect("write platform-event audit row");
    sink.flush_pending().await.expect("flush platform-event row");
}
