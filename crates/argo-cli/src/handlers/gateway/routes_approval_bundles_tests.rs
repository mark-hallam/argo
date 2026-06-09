//! Pure-function tests for [`routes_approval_bundles`] — covers the
//! JSON parsing, query-string parsing, and member-array validation
//! paths. The end-to-end "create → decide" round-trip needs a
//! workspace + memory store + gateway listener stack, exercised by
//! the broader gateway smoke suite (`tests/gateway_*`).
#![cfg(test)]
use serde_json::json;

use super::{
    bad_request_for_test, conflict_for_test, not_found_for_test, ok_decide_for_test,
    parse_limit_for_test, parse_members_for_test, parse_query_for_test,
};

#[test]
fn parse_query_extracts_known_key() {
    let tail = "/api/v1/approvals/bundles?status=pending&limit=10 HTTP/1.1";
    assert_eq!(
        parse_query_for_test(tail, "status"),
        Some("pending".to_string())
    );
    assert_eq!(parse_query_for_test(tail, "limit"), Some("10".to_string()));
    assert_eq!(parse_query_for_test(tail, "missing"), None);
}

#[test]
fn parse_limit_defaults_and_validates() {
    assert_eq!(
        parse_limit_for_test("/api/v1/approvals/bundles HTTP/1.1").unwrap(),
        50
    );
    assert_eq!(
        parse_limit_for_test("/api/v1/approvals/bundles?limit=7 HTTP/1.1").unwrap(),
        7
    );
    let err = parse_limit_for_test("/api/v1/approvals/bundles?limit=abc HTTP/1.1")
        .expect_err("non-number must reject");
    assert_eq!(err.0, "400 Bad Request");
    let err = parse_limit_for_test("/api/v1/approvals/bundles?limit=0 HTTP/1.1")
        .expect_err("0 must reject");
    assert!(err.1.contains("1..=100"));
    let err = parse_limit_for_test("/api/v1/approvals/bundles?limit=999 HTTP/1.1")
        .expect_err("over-cap must reject");
    assert!(err.1.contains("1..=100"));
}

#[test]
fn parse_members_accepts_valid_array() {
    let v = json!(["a", "b", "c"]);
    let out = parse_members_for_test(Some(&v)).unwrap();
    assert_eq!(out, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

#[test]
fn parse_members_rejects_missing_or_empty() {
    let err = parse_members_for_test(None).expect_err("missing must reject");
    assert!(err.contains("array"));
    let empty = json!([]);
    let err = parse_members_for_test(Some(&empty)).expect_err("empty must reject");
    assert!(err.contains("at least one"));
    let mixed = json!(["a", 7]);
    let err = parse_members_for_test(Some(&mixed)).expect_err("non-string must reject");
    assert!(err.contains("strings"));
    let blank = json!(["a", "   "]);
    let err = parse_members_for_test(Some(&blank)).expect_err("blank must reject");
    assert!(err.contains("non-empty"));
}

#[test]
fn error_helpers_carry_envelope_shape() {
    let (status, body) = bad_request_for_test("nope");
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("\"bad_request\""));
    assert!(body.contains("nope"));

    let (status, body) = not_found_for_test("xyz");
    assert_eq!(status, "404 Not Found");
    assert!(body.contains("bundle_not_found"));
    assert!(body.contains("xyz"));

    let (status, body) = conflict_for_test("xyz", "approve");
    assert_eq!(status, "409 Conflict");
    assert!(body.contains("already_decided"));
    assert!(body.contains("approve"));
}

#[test]
fn ok_decide_envelope_has_counts() {
    let (status, body) = ok_decide_for_test("b1", "approve", 5, 2);
    assert_eq!(status, "200 OK");
    assert!(body.contains("\"bundle_id\":\"b1\""));
    assert!(body.contains("\"decision\":\"approve\""));
    assert!(body.contains("\"members_decided\":5"));
    assert!(body.contains("\"members_skipped\":2"));
}

// ---------------------------------------------------------------------
// Cross-tenant decide guard (WS1 — mirrors WS1-4 #2696).
//
// `decide_response` loads the bundle by its GLOBAL uuid, so before this
// fix a signed-in user could approve/reject ANY tenant's bundle by
// holding its id. The guard resolves the caller's tenant and rejects a
// mismatch as `not_found` (no existence leak, no transition). These
// tests drive the resolver via the `X-Tenant-Id` override header.
// ---------------------------------------------------------------------
use std::sync::Arc;

use argo_store::approval_bundle::{create_bundle, get_bundle, BundleStatus, NewBundle};
use argo_store::SessionStore;
use argo_types::TenantId;

use super::decide_response;

async fn store_with_two_tenants() -> (Arc<SessionStore>, TenantId, TenantId) {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    // `ensure_workspace` creates the workspace row the tenant->workspace
    // lookup needs and hands back the generated tenant id.
    let (tenant_a, _) = store
        .ensure_workspace("ws-a", "/root/a")
        .await
        .expect("workspace a");
    let (tenant_b, _) = store
        .ensure_workspace("ws-b", "/root/b")
        .await
        .expect("workspace b");
    (Arc::new(store), tenant_a, tenant_b)
}

fn decide_req(bundle_id: &str, tenant: TenantId) -> String {
    format!(
        "POST /api/v1/approvals/bundles/{bundle_id}/decide HTTP/1.1\r\n\
         X-Tenant-Id: {tenant}\r\n\r\n{{\"decision\":\"approve\"}}"
    )
}

#[tokio::test]
async fn decide_rejects_cross_tenant_bundle_with_not_found_and_no_transition() {
    let (store, tenant_a, tenant_b) = store_with_two_tenants().await;
    // Bundle owned by tenant B.
    let bundle_id = create_bundle(
        store.pool(),
        &NewBundle {
            tenant_id: tenant_b,
            label: "tenant-b work",
            member_approval_ids: vec!["act-b-1".to_string()],
        },
    )
    .await
    .expect("create bundle b");

    // Tenant A's session tries to decide tenant B's bundle.
    let req = decide_req(&bundle_id, tenant_a);
    let (status, body) = decide_response(&bundle_id, &req, Some(&store)).await;

    assert_eq!(status, "404 Not Found", "cross-tenant decide must 404");
    assert!(body.contains("bundle_not_found"), "body: {body}");
    // No transition applied: bundle B is still Pending.
    let after = get_bundle(store.pool(), &bundle_id)
        .await
        .expect("reload")
        .expect("bundle still exists");
    assert_eq!(
        after.status,
        BundleStatus::Pending,
        "cross-tenant decide must not flip the bundle"
    );
    assert!(after.decision.is_none(), "no decision should be recorded");
}

#[tokio::test]
async fn decide_allows_same_tenant_bundle() {
    let (store, _tenant_a, tenant_b) = store_with_two_tenants().await;
    let bundle_id = create_bundle(
        store.pool(),
        &NewBundle {
            tenant_id: tenant_b,
            label: "tenant-b work",
            member_approval_ids: vec!["act-b-1".to_string()],
        },
    )
    .await
    .expect("create bundle b");

    // Tenant B decides its OWN bundle — succeeds and flips the row.
    let req = decide_req(&bundle_id, tenant_b);
    let (status, _body) = decide_response(&bundle_id, &req, Some(&store)).await;

    assert_eq!(status, "200 OK", "same-tenant decide must succeed");
    let after = get_bundle(store.pool(), &bundle_id)
        .await
        .expect("reload")
        .expect("bundle exists");
    assert_eq!(
        after.status,
        BundleStatus::Approved,
        "same-tenant decide must flip the bundle to approved"
    );
}

#[tokio::test]
async fn decide_malformed_id_404s_without_leaking_parse_error() {
    // A non-UUID bundle id previously reached the store's id parser and
    // surfaced its internal `bundle id parse: invalid character ...` string
    // with a 500 (e2e R3-F3/R1-F1). It must now 404 like any unknown
    // bundle, with no parse-internals leaked.
    let (store, tenant_a, _tenant_b) = store_with_two_tenants().await;
    let req = decide_req("not-a-uuid", tenant_a);
    let (status, body) = decide_response("not-a-uuid", &req, Some(&store)).await;

    assert_eq!(status, "404 Not Found", "malformed bundle id must 404");
    assert!(body.contains("bundle_not_found"), "body: {body}");
    assert!(
        !body.contains("bundle id parse"),
        "must not leak the store's parse-error string: {body}"
    );
    assert!(
        !body.contains("invalid character"),
        "must not leak parse internals: {body}"
    );
}
