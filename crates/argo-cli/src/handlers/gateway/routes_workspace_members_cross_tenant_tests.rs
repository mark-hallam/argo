//! Cross-tenant negative tests for `/api/v1/workspace/members*`
//! (mission-cross-tenant-test-workspace-members).
//!
//! Pins tenant-scoping invariants on the workspace-members surface —
//! the top-3 data-exposure risk per
//! `docs/skycot/audit/cross-tenant-endpoint-coverage.md`. Lives in a
//! sibling file (rather than appending to
//! `routes_workspace_members_tests.rs`) so neither test file breaks
//! the 500-LOC per-file invariant; the sibling file already sits at
//! 319 LOC and the three new tests add ~260 LOC.
//!
//! Each test seeds two tenants via `provision_response` (the canonical
//! tenant-seed path the sibling test file uses) so they share no
//! state via parallel fixtures.

#![allow(clippy::too_many_lines)]

use super::super::auth_magic_link::sha256_hex;
use super::super::onboarding_provision::provision_response;
use super::super::routes_workspace_members_accept::accept_response;
use super::super::routes_workspace_members_invite::invite_response;
use super::list_response;
use argo_identity::user_store::get_user_by_email;
use argo_identity::user_tenant_store::list_tenants_for_user;
use argo_store::workspace_member_invitation::{
    create_invitation, list_pending_for_tenant, lookup_pending_by_token_hash,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

const TENANT_A_PRINCIPAL: &str = "human:owner@tenant-a.example";
const TENANT_B_PRINCIPAL: &str = "human:owner@tenant-b.example";

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn invite_req(body: &str, principal: &str, role_header: &str) -> String {
    let mut head =
        String::from("POST /api/v1/workspace/members/invite HTTP/1.1\r\nHost: x\r\n");
    head.push_str(&format!("X-Argo-Principal: {principal}\r\n"));
    head.push_str(&format!("X-Argo-Role: {role_header}\r\n"));
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

fn list_req(principal: &str) -> String {
    let mut head = String::from("GET /api/v1/workspace/members HTTP/1.1\r\nHost: x\r\n");
    head.push_str(&format!("X-Argo-Principal: {principal}\r\n\r\n"));
    head
}

fn provision_req(principal: &str) -> String {
    let body = json!({ "workspace_name": "Acme" }).to_string();
    let mut head =
        String::from("POST /api/v1/onboarding/provision HTTP/1.1\r\nHost: x\r\n");
    head.push_str(&format!("X-Argo-Principal: {principal}\r\n"));
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

fn invite_body(email: &str, role: &str) -> String {
    json!({
        "email": email,
        "role": role,
        "cohort_scope": [],
    })
    .to_string()
}

async fn seed_tenant_returning_id(store: &Arc<SessionStore>, principal: &str) -> TenantId {
    let (status, body) = provision_response(&provision_req(principal), Some(store)).await;
    assert_eq!(status, "201 Created", "provision body: {body}");
    let v: Value = serde_json::from_str(&body).expect("provision json");
    let raw = v["tenant_id"].as_str().expect("tenant_id in provision response");
    TenantId(Uuid::parse_str(raw).expect("tenant_id parse"))
}

#[tokio::test]
async fn cross_tenant_list_excludes_other_tenants_members_and_invites() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;

    let tenant_a = seed_tenant_returning_id(&store, TENANT_A_PRINCIPAL).await;
    let tenant_b = seed_tenant_returning_id(&store, TENANT_B_PRINCIPAL).await;
    assert_ne!(tenant_a, tenant_b, "tenant-a and tenant-b must be distinct");

    let (a_status, a_body) = invite_response(
        &invite_req(
            &invite_body("invitee-a@tenant-a.example", "user"),
            TENANT_A_PRINCIPAL,
            "owner",
        ),
        Some(&store),
    )
    .await;
    assert_eq!(a_status, "200 OK", "A invite body: {a_body}");
    let (b_status, b_body) = invite_response(
        &invite_req(
            &invite_body("invitee-b@tenant-b.example", "user"),
            TENANT_B_PRINCIPAL,
            "owner",
        ),
        Some(&store),
    )
    .await;
    assert_eq!(b_status, "200 OK", "B invite body: {b_body}");

    // Tenant A caller: list contains only A's member + A's invite.
    let (status, body) = list_response(&list_req(TENANT_A_PRINCIPAL), Some(&store)).await;
    assert_eq!(status, "200 OK", "list A body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["tenant_id"], tenant_a.to_string());
    let members = v["members"].as_array().expect("members array");
    assert_eq!(members.len(), 1, "A members: {members:?}");
    assert_eq!(members[0]["email"], "owner@tenant-a.example");
    let pending = v["pending_invitations"].as_array().expect("pending array");
    assert_eq!(pending.len(), 1, "A pending: {pending:?}");
    assert_eq!(pending[0]["email"], "invitee-a@tenant-a.example");
    for m in members {
        assert_ne!(m["email"], "owner@tenant-b.example");
    }
    for p in pending {
        assert_ne!(p["email"], "invitee-b@tenant-b.example");
    }

    // Symmetric assertion from tenant B's side.
    let (status, body) = list_response(&list_req(TENANT_B_PRINCIPAL), Some(&store)).await;
    assert_eq!(status, "200 OK", "list B body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["tenant_id"], tenant_b.to_string());
    let members = v["members"].as_array().expect("members array");
    assert_eq!(members.len(), 1, "B members: {members:?}");
    assert_eq!(members[0]["email"], "owner@tenant-b.example");
    let pending = v["pending_invitations"].as_array().expect("pending array");
    assert_eq!(pending.len(), 1, "B pending: {pending:?}");
    assert_eq!(pending[0]["email"], "invitee-b@tenant-b.example");
    for m in members {
        assert_ne!(m["email"], "owner@tenant-a.example");
    }
    for p in pending {
        assert_ne!(p["email"], "invitee-a@tenant-a.example");
    }
}

#[tokio::test]
async fn cross_tenant_invite_does_not_pollute_other_tenants_pending_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;

    let tenant_a = seed_tenant_returning_id(&store, TENANT_A_PRINCIPAL).await;
    let tenant_b = seed_tenant_returning_id(&store, TENANT_B_PRINCIPAL).await;

    let shared_email = "shared@example.com";
    let (a_status, _) = invite_response(
        &invite_req(
            &invite_body(shared_email, "user"),
            TENANT_A_PRINCIPAL,
            "owner",
        ),
        Some(&store),
    )
    .await;
    assert_eq!(a_status, "200 OK");

    // Snapshot A's invite row before B touches anything.
    let a_pending_before = list_pending_for_tenant(&store, tenant_a)
        .await
        .expect("list A pending");
    assert_eq!(a_pending_before.len(), 1, "A pending after first invite");
    let a_row_before = a_pending_before[0].clone();
    assert_eq!(a_row_before.tenant_id, tenant_a);
    assert_eq!(a_row_before.email, shared_email);

    // B invites the same email. Must be 200 OK — not 409 — because the
    // duplicate-pending check is scoped to (tenant, email), not email
    // alone.
    let (b_status, b_body) = invite_response(
        &invite_req(
            &invite_body(shared_email, "user"),
            TENANT_B_PRINCIPAL,
            "owner",
        ),
        Some(&store),
    )
    .await;
    assert_eq!(
        b_status, "200 OK",
        "B's invite for shared email must succeed under B's tenant (body: {b_body})"
    );

    // A's row must be bit-identical to its pre-state. Refetch via
    // token-hash (the same store helper the accept handler uses) so a
    // future regression that silently rewrites the token cannot pass.
    let a_row_after = lookup_pending_by_token_hash(&store, &a_row_before.token_hash)
        .await
        .expect("lookup A token hash")
        .expect("A row still exists");
    assert_eq!(
        a_row_after, a_row_before,
        "tenant-A invite row mutated by tenant-B's invite"
    );

    // B has its own pending row under B's tenant, distinct from A's.
    let b_pending = list_pending_for_tenant(&store, tenant_b)
        .await
        .expect("list B pending");
    assert_eq!(b_pending.len(), 1, "B pending after invite");
    let b_row = &b_pending[0];
    assert_eq!(b_row.tenant_id, tenant_b);
    assert_eq!(b_row.email, shared_email);
    assert_ne!(b_row.id, a_row_before.id);
    assert_ne!(b_row.token_hash, a_row_before.token_hash);

    // A still has exactly one pending row — not zero, not two.
    let a_pending_after = list_pending_for_tenant(&store, tenant_a)
        .await
        .expect("list A pending after");
    assert_eq!(a_pending_after.len(), 1);
    assert_eq!(a_pending_after[0], a_row_before);
}

#[tokio::test]
async fn cross_tenant_accept_binds_membership_to_invite_tenant_not_caller_tenant() {
    // Security-critical: a token issued by tenant A must bind the new
    // membership to tenant A, regardless of which tenant the accepter
    // is otherwise associated with. The accept handler today is
    // token-only (no principal header consulted); this test pins that
    // invariant against a future refactor that adds a principal check
    // and silently rebinds to the caller's tenant.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;

    let tenant_a = seed_tenant_returning_id(&store, TENANT_A_PRINCIPAL).await;
    let tenant_b = seed_tenant_returning_id(&store, TENANT_B_PRINCIPAL).await;

    // Mint a known token for tenant A directly via the store helper so
    // the test can submit the raw token to the accept handler. The
    // public invite handler emits a random token only via email/log.
    let raw_token = "test-token-cross-tenant-accept-3";
    let invitee_email = "accepter@example.com";
    // Use real wall-clock — the accept handler calls `Utc::now()` to
    // evaluate expiry, so a fixed epoch leaves `expires_at` deep in
    // the past and the accept returns 410 Gone.
    let now = Utc::now().timestamp();
    let invitation = create_invitation(
        &store,
        tenant_a,
        invitee_email,
        "user",
        &[],
        &sha256_hex(raw_token),
        "human:owner@tenant-a.example",
        now,
        now + 7 * 24 * 3600,
    )
    .await
    .expect("seed A invitation");
    assert_eq!(invitation.tenant_id, tenant_a);

    // Submit the accept request with a tenant-B principal header to
    // demonstrate that even an authenticated B-caller cannot coerce the
    // accept handler into rebinding the membership to B's tenant.
    let mut head =
        String::from("POST /api/v1/workspace/members/accept HTTP/1.1\r\nHost: x\r\n");
    head.push_str(&format!("X-Argo-Principal: {TENANT_B_PRINCIPAL}\r\n"));
    let body = json!({ "token": raw_token }).to_string();
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    let (status, resp_body) = accept_response(&head, Some(&store)).await;
    assert_eq!(status, "200 OK", "accept body: {resp_body}");
    let v: Value = serde_json::from_str(&resp_body).unwrap();
    assert_eq!(
        v["tenant_id"],
        tenant_a.to_string(),
        "accept response tenant_id must be the invite's tenant (A), not the caller's (B)"
    );
    assert_eq!(v["email"], invitee_email);

    // The new user is linked to tenant A — NOT tenant B. Look the user
    // up by email (the accept handler upserts on the invitation's
    // email, not on any caller-bound identity) and inspect their
    // tenant links.
    let user = get_user_by_email(&store, invitee_email)
        .await
        .expect("get_user_by_email")
        .expect("user row created by accept");
    let links = list_tenants_for_user(&store, user.id)
        .await
        .expect("list_tenants_for_user");
    assert_eq!(
        links.len(),
        1,
        "accepter should belong to exactly one tenant: {links:?}"
    );
    assert_eq!(
        links[0].tenant_id, tenant_a,
        "accepter must be linked to A (invite's tenant), not B (caller's tenant)"
    );
    assert_ne!(
        links[0].tenant_id, tenant_b,
        "regression: accept handler rebound membership to caller's tenant"
    );

    // Invitation row is now marked accepted (single-use guarantee).
    let consumed = lookup_pending_by_token_hash(&store, &sha256_hex(raw_token))
        .await
        .expect("lookup consumed invitation")
        .expect("row still present (kept for forensic trace)");
    assert!(
        consumed.accepted_at.is_some(),
        "accepted invitation must have accepted_at set"
    );
}
