//! Round-trip tests for the work-path audit writers
//! (`fix-work-audit-rows-tenant-roundtrip`).
//!
//! These pin the product's core auditability promise: a row written by
//! a work-path handler MUST surface when the SAME caller reads
//! `GET /api/v1/audit`. The bug was a tenant-resolution split — writers
//! resolved their tenant through `rate_limit::extract_tenant`
//! (`?tenant=` → header → config-default / `"anonymous"`), while the
//! feed's default scope resolves through the canonical
//! `resolve_tenant::resolve_tenant_for_request` (session JWT → header →
//! cwd workspace). For a caller carrying no explicit tenant signal the
//! two disagree: the writer stamped `"anonymous"`, the feed read the
//! cwd workspace UUID, and the row never round-tripped.
//!
//! The fix routes every work-path writer through
//! `resolve_tenant::resolve_audit_write_tenant`, which shares the feed's
//! resolver. These tests exercise the real handlers (pitch `ask_response`
//! end-to-end; the genesis-quiz writer directly) on a request with no
//! explicit tenant signal — the exact case that was broken — and assert
//! the row reads back, plus a cross-tenant negative.

use super::routes_audit_feed::list_response;
use super::{rate_limit, resolve_tenant, routes_genesis_quiz, routes_pitch_ask};
use argo_store::SessionStore;
use serde_json::Value;
use std::sync::Arc;

/// Fresh in-memory store. `ARGO_STORE_BATCHED_WRITES=0` forces the
/// audit sink onto its synchronous per-row insert path so a write is
/// immediately readable in the same test (the work-path writers
/// construct a short-lived `AuditSink` we cannot flush from here). The
/// value is always `"0"` wherever any test sets it, so there is no
/// torn-value race with the marketplace / message tests that set it too.
async fn mem_store() -> Arc<SessionStore> {
    std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// A read request scoped to an explicit (foreign) tenant via the
/// `X-Argo-Tenant-Id` header the feed's scope honours — used for the
/// cross-tenant negative assertions.
fn feed_get_other_tenant() -> String {
    "GET /api/v1/audit HTTP/1.1\r\nHost: x\r\n\
     X-Argo-Tenant-Id: 99999999-9999-9999-9999-999999999999\r\n\r\n"
        .to_string()
}

/// A read request with NO tenant signal — resolves to the same cwd
/// workspace tenant the un-signalled writer now stamps under.
fn feed_get_default() -> String {
    "GET /api/v1/audit HTTP/1.1\r\nHost: x\r\n\r\n".to_string()
}

fn items(body: &str) -> Vec<Value> {
    let v: Value = serde_json::from_str(body).expect("feed body is JSON");
    v["items"].as_array().cloned().unwrap_or_default()
}

fn has_action_type(items: &[Value], action_type: &str) -> bool {
    items.iter().any(|it| it["action_type"] == action_type)
}

#[tokio::test]
async fn pitch_ask_audit_row_round_trips_to_feed_for_same_caller() {
    let store = mem_store().await;

    // Off-topic question → canonical miss. With no provider key present
    // (swarm invariant) the LLM fallback is disabled, so the handler
    // emits a deterministic `PitchCanonicalQaMiss` row and makes no
    // model call. No `?tenant=` / `X-Tenant-Id` / cookie → the exact
    // un-signalled case the bug dropped.
    let payload =
        serde_json::json!({ "question": "Recite a haiku about arctic terns.", "channel": "Rehearsal" });
    let req = format!(
        "POST /api/v1/pitch/ask HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/json\r\n\r\n{payload}"
    );

    // Regression guard: the write tenant the fix resolves MUST differ
    // from the pre-fix `extract_tenant` value, else this test could not
    // distinguish fixed from broken. (cwd workspace UUID vs the
    // config-default / `"anonymous"` tail.)
    let write_tenant = resolve_tenant::resolve_audit_write_tenant(&req, &store).await;
    assert_ne!(
        write_tenant,
        rate_limit::extract_tenant(&req),
        "test premise: fixed resolver must diverge from extract_tenant on an un-signalled request"
    );

    let (status, _body) = routes_pitch_ask::ask_response(&req, Some(&store)).await;
    assert_eq!(status, "200 OK");

    // Same caller (no tenant signal) reads the feed → the pitch row is
    // present.
    let (fstatus, fbody) = list_response(&feed_get_default(), Some(&store)).await;
    assert_eq!(fstatus, "200 OK");
    let feed = items(&fbody);
    assert!(
        has_action_type(&feed, "pitch.canonical_qa_miss"),
        "pitch audit row did not round-trip into the same caller's feed: {fbody}"
    );

    // Cross-tenant negative: a different tenant's feed must NOT see it.
    let (ostatus, obody) = list_response(&feed_get_other_tenant(), Some(&store)).await;
    assert_eq!(ostatus, "200 OK");
    assert!(
        !has_action_type(&items(&obody), "pitch.canonical_qa_miss"),
        "pitch audit row leaked into a foreign tenant's feed: {obody}"
    );
}

#[tokio::test]
async fn genesis_quiz_audit_row_round_trips_to_feed_for_same_caller() {
    let store = mem_store().await;

    // The genesis-quiz writer, called the way the route does, on a
    // request with no explicit tenant signal.
    let req = "POST /api/v1/genesis/quiz HTTP/1.1\r\nHost: localhost\r\n\r\n";
    routes_genesis_quiz::emit_audit(Some(&store), req, "vertical-x", true, 2).await;

    let (fstatus, fbody) = list_response(&feed_get_default(), Some(&store)).await;
    assert_eq!(fstatus, "200 OK");
    assert!(
        has_action_type(&items(&fbody), "genesis/quiz"),
        "genesis-quiz audit row did not round-trip into the same caller's feed: {fbody}"
    );

    let (ostatus, obody) = list_response(&feed_get_other_tenant(), Some(&store)).await;
    assert_eq!(ostatus, "200 OK");
    assert!(
        !has_action_type(&items(&obody), "genesis/quiz"),
        "genesis-quiz audit row leaked into a foreign tenant's feed: {obody}"
    );
}
