use super::{
    accept_invitation, create_invitation, ensure_schema, list_pending_for_tenant,
    lookup_pending_by_email, lookup_pending_by_token_hash, AcceptOutcome, INVITATION_TTL_SECS,
};
use crate::SessionStore;
use argo_types::TenantId;
use uuid::Uuid;

async fn store() -> SessionStore {
    let s = SessionStore::open(":memory:").await.expect("open");
    s.migrate().await.expect("migrate");
    s
}

#[tokio::test]
async fn ensure_schema_is_idempotent() {
    let s = store().await;
    ensure_schema(s.pool()).await.expect("first");
    ensure_schema(s.pool()).await.expect("second");
}

#[tokio::test]
async fn create_and_lookup_round_trips() {
    let s = store().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_i64;
    let scope = vec!["sales".to_string(), "support".to_string()];
    let created = create_invitation(
        &s,
        tenant,
        "alice@example.com",
        "user",
        &scope,
        "tokenhash-aaaa",
        "human:operator@example.com",
        now,
        now + INVITATION_TTL_SECS,
    )
    .await
    .expect("create");
    assert_eq!(created.email, "alice@example.com");
    assert_eq!(created.role, "user");
    assert_eq!(created.cohort_scope, scope);
    assert!(created.accepted_at.is_none());
    let by_email = lookup_pending_by_email(&s, tenant, "alice@example.com")
        .await
        .expect("by_email")
        .expect("row present");
    assert_eq!(by_email.id, created.id);
    let by_hash = lookup_pending_by_token_hash(&s, "tokenhash-aaaa")
        .await
        .expect("by_hash")
        .expect("row present");
    assert_eq!(by_hash.id, created.id);
    assert_eq!(by_hash.cohort_scope, scope);
}

#[tokio::test]
async fn accept_transitions_to_accepted_and_is_single_use() {
    let s = store().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_i64;
    create_invitation(
        &s,
        tenant,
        "bob@example.com",
        "admin",
        &[],
        "tokenhash-bbbb",
        "human:owner@example.com",
        now,
        now + INVITATION_TTL_SECS,
    )
    .await
    .expect("create");
    let uid = Uuid::now_v7();
    let first = accept_invitation(&s, "tokenhash-bbbb", uid, now + 60)
        .await
        .expect("accept");
    assert!(matches!(first, AcceptOutcome::Ok(_)));
    let second = accept_invitation(&s, "tokenhash-bbbb", uid, now + 90)
        .await
        .expect("second accept");
    assert_eq!(second, AcceptOutcome::AlreadyAccepted);
    // pending lookup by email is now empty (the row was consumed).
    assert!(lookup_pending_by_email(&s, tenant, "bob@example.com")
        .await
        .expect("by_email")
        .is_none());
}

#[tokio::test]
async fn accept_returns_expired_when_window_passed() {
    let s = store().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_i64;
    create_invitation(
        &s,
        tenant,
        "carol@example.com",
        "user",
        &[],
        "tokenhash-cccc",
        "human:owner@example.com",
        now,
        now + 60,
    )
    .await
    .expect("create");
    let outcome = accept_invitation(&s, "tokenhash-cccc", Uuid::now_v7(), now + 120)
        .await
        .expect("accept");
    assert_eq!(outcome, AcceptOutcome::Expired);
}

#[tokio::test]
async fn accept_unknown_token_is_not_found() {
    let s = store().await;
    ensure_schema(s.pool()).await.expect("schema");
    let r = accept_invitation(&s, "ghost", Uuid::now_v7(), 1_700)
        .await
        .expect("accept");
    assert_eq!(r, AcceptOutcome::NotFound);
}

#[tokio::test]
async fn list_pending_filters_accepted() {
    let s = store().await;
    let tenant = TenantId::new();
    let now = 1_700_000_000_i64;
    create_invitation(
        &s,
        tenant,
        "p1@example.com",
        "user",
        &[],
        "tokenhash-1",
        "human:owner@example.com",
        now,
        now + INVITATION_TTL_SECS,
    )
    .await
    .expect("p1");
    create_invitation(
        &s,
        tenant,
        "p2@example.com",
        "guest",
        &[],
        "tokenhash-2",
        "human:owner@example.com",
        now + 10,
        now + INVITATION_TTL_SECS + 10,
    )
    .await
    .expect("p2");
    accept_invitation(&s, "tokenhash-1", Uuid::now_v7(), now + 30)
        .await
        .expect("accept p1");
    let pending = list_pending_for_tenant(&s, tenant)
        .await
        .expect("list_pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].email, "p2@example.com");
}
