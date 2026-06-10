//! Tests for the conversational-intake turn handler — wizard path (no provider)
//! + per-tenant persistence + isolation.

use super::*;
use serde_json::Value;

fn store() -> (tempfile::TempDir, IntakeStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = IntakeStore::new(dir.path());
    (dir, store)
}

/// Drive one wizard turn (no provider) and return the JSON envelope.
async fn turn(store: &IntakeStore, tenant: &str, conv: &str, message: Option<&str>, confirm: bool) -> Value {
    let (status, value) = run_turn(store, tenant, conv, message, confirm, None)
        .await
        .expect("turn succeeds");
    assert_eq!(status, "200 OK");
    value
}

#[tokio::test]
async fn opening_turn_asks_for_the_charter() {
    let (_d, store) = store();
    let v = turn(&store, "op-a", "c1", None, false).await;
    assert_eq!(v["status"], "asking");
    assert_eq!(v["slot"], "charter");
    assert!(v["assistant_message"].as_str().is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
async fn wizard_walk_reaches_ready_then_done() {
    let (_d, store) = store();
    let tenant = "op-a";
    let conv = "walk";
    // Opening question.
    turn(&store, tenant, conv, None, false).await;
    // Answer each slot in order; the deterministic wizard advances one per turn.
    let answers = [
        "Be my support agent who clears the inbound ticket queue",
        "Sam",
        "Answer tier-1 tickets",
        "weekdays on each inbound ticket",
        "never issue a refund alone",
        "first response under five minutes",
    ];
    let mut last = Value::Null;
    for a in answers {
        last = turn(&store, tenant, conv, Some(a), false).await;
    }
    // Required slots filled → the optional DEC-048 questions follow, with the
    // early-build affordance flagged (`can_confirm`).
    assert_eq!(last["status"], "asking");
    assert_eq!(last["slot"], "access");
    assert_eq!(last["can_confirm"], true);
    let last = turn(&store, tenant, conv, Some("Xero and Gmail"), false).await;
    assert_eq!(last["slot"], "escalation");
    let last = turn(&store, tenant, conv, Some("after 2 tries"), false).await;
    // All slots handled → ready, with an assembled spec carrying the slots.
    assert_eq!(last["status"], "ready");
    assert_eq!(last["spec"]["persona"], "sam");
    assert_eq!(last["captured"]["cadence"], "weekdays on each inbound ticket");
    assert_eq!(
        last["captured"]["access_integrations"],
        serde_json::json!(["Xero", "Gmail"])
    );
    assert_eq!(
        last["spec"]["access_integrations"],
        serde_json::json!(["Xero", "Gmail"])
    );
    assert_eq!(last["spec"]["escalation"]["escalate_after_failures"], 2);
    assert!(last["spec"]["capabilities"].as_array().is_some_and(|a| !a.is_empty()));

    // Confirming flips ready → done.
    let done = turn(&store, tenant, conv, None, true).await;
    assert_eq!(done["status"], "done");
    assert_eq!(done["spec"]["persona"], "sam");
}

#[tokio::test]
async fn optional_skip_resolves_without_wedging() {
    let (_d, store) = store();
    let conv = "skips";
    turn(&store, "op-a", conv, None, false).await;
    for a in [
        "Be my support agent who clears the inbound ticket queue",
        "Sam",
        "Answer tier-1 tickets",
        "weekdays on each inbound ticket",
        "never issue a refund alone",
        "first response under five minutes",
    ] {
        turn(&store, "op-a", conv, Some(a), false).await;
    }
    // Skip both optional slots → ready with the seeded escalation defaults.
    let v = turn(&store, "op-a", conv, Some("skip"), false).await;
    assert_eq!(v["slot"], "escalation");
    let v = turn(&store, "op-a", conv, Some("not sure"), false).await;
    assert_eq!(v["status"], "ready");
    assert_eq!(v["spec"]["escalation"]["escalate_after_failures"], 3);
    assert_eq!(v["captured"]["access_integrations"], serde_json::json!([]));
}

#[tokio::test]
async fn early_confirm_bypasses_optional_questions() {
    let (_d, store) = store();
    let conv = "early";
    turn(&store, "op-a", conv, None, false).await;
    for a in [
        "Be my support agent who clears the inbound ticket queue",
        "Sam",
        "Answer tier-1 tickets",
        "weekdays on each inbound ticket",
        "never issue a refund alone",
        "first response under five minutes",
    ] {
        turn(&store, "op-a", conv, Some(a), false).await;
    }
    // The Access question is pending, but a confirm goes straight to done —
    // DEC-048: optional questions never gate the build.
    let done = turn(&store, "op-a", conv, None, true).await;
    assert_eq!(done["status"], "done");
}

#[tokio::test]
async fn vague_cadence_gets_exactly_one_followup() {
    let (_d, store) = store();
    let conv = "vague";
    turn(&store, "op-a", conv, None, false).await;
    turn(&store, "op-a", conv, Some("Be my support agent for the queue"), false).await;
    turn(&store, "op-a", conv, Some("Sam"), false).await;
    turn(&store, "op-a", conv, Some("Answer tier-1 tickets"), false).await;
    // Vague cadence → the SAME slot is re-asked once (the quality follow-up).
    let v = turn(&store, "op-a", conv, Some("Weekly"), false).await;
    assert_eq!(v["slot"], "cadence", "follow-up targets the cadence slot");
    assert!(
        v["assistant_message"]
            .as_str()
            .is_some_and(|q| q.to_lowercase().contains("day")),
        "follow-up asks for a concrete day/time"
    );
    // A second vague answer is ACCEPTED (hard cap 1) — intake never loops.
    let v = turn(&store, "op-a", conv, Some("Weekly"), false).await;
    assert_eq!(v["slot"], "exceptions", "cap honored; conversation advances");
}

#[tokio::test]
async fn conversation_persists_across_turns() {
    let (_d, store) = store();
    turn(&store, "op-a", "persist", None, false).await;
    turn(&store, "op-a", "persist", Some("Be my recruiter screener for inbound applicants"), false).await;
    // Re-load: the charter answer survived between turns.
    let reloaded = store.load("op-a", "persist").expect("persisted state");
    assert!(reloaded.charter.is_some());
}

#[tokio::test]
async fn tenant_b_starts_a_fresh_conversation_for_the_same_id() {
    let (_d, store) = store();
    // Tenant A fills the charter under a shared conversation id.
    turn(&store, "op-a", "shared", Some("Be my bookkeeping assistant for invoices"), false).await;
    // Tenant B, same id → fresh conversation, still asking for the charter.
    let b = turn(&store, "op-b", "shared", None, false).await;
    assert_eq!(b["slot"], "charter");
    assert!(store.load("op-b", "shared").is_some());
    // Tenant A's intake is untouched and unreadable by B's path.
    let a = store.load("op-a", "shared").expect("tenant A state");
    assert!(a.charter.is_some());
}

// --- Tenant-resolution isolation (fix-genesis-intake-tenant-resolution) ------
//
// The store key must come from the JWT-bound tenant, not the client-supplied
// `X-Tenant-Id` header. These mirror #2710's `resolve_tenant_tests` pattern:
// mint a `__Host-session` cookie JWT and assert the resolver/store-key honors
// it over the header. Env handling uses the process-global gateway test lock so
// the `SKYCOT_AUTH_JWT_SECRET` mutation can't be torn by a sibling module.

use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{provision_for_user, ProvisionRequest};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::Serialize;
use std::sync::Arc;

const TEST_SECRET: &str = "genesis-intake-resolution-test-secret";

/// Minimal cookie-JWT claim shape — the fields the resolver reads.
#[derive(Serialize)]
struct TestClaims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: Option<String>,
    iat: i64,
    exp: i64,
}

/// Mint a `__Host-session` cookie JWT carrying `tnt = tenant`.
fn mint_session(tenant: &TenantId) -> String {
    let now = Utc::now().timestamp();
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &TestClaims {
            sub: "11111111-1111-1111-1111-111111111111",
            email: "operator@example.com",
            tnt: Some(tenant.to_string()),
            iat: now - 10,
            exp: now + 3600,
        },
        &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .expect("mint session jwt")
}

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Provision a fresh user → returns the tenant they own.
async fn provision(store: &SessionStore, email: &str, ws: &str) -> TenantId {
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await.expect("insert user");
    provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: ws,
            company_name: None,
        },
    )
    .await
    .expect("provision")
    .tenant_id
}

/// A `POST /api/v1/genesis/conversation` request with the given cookie +
/// optional `X-Tenant-Id` override header.
fn intake_req(jwt: &str, override_tenant: Option<&TenantId>) -> String {
    let header = override_tenant
        .map(|t| format!("X-Tenant-Id: {t}\r\n"))
        .unwrap_or_default();
    let body = r#"{"conversation_id":"c1","message":"Be my support agent"}"#;
    format!(
        "POST /api/v1/genesis/conversation HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/json\r\n\
         Cookie: __Host-session={jwt}\r\n{header}Content-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Acceptance 1 — an authed tenant-A request that ALSO carries
/// `X-Tenant-Id: B` keys the intake store on A (the JWT tenant), never B.
/// The header cannot redirect the store key.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn authed_request_keys_on_jwt_tenant_not_header_override() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = session_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;
    assert_ne!(tenant_a, tenant_b);

    let jwt = mint_session(&tenant_a);
    let req = intake_req(&jwt, Some(&tenant_b));
    let resolved = resolve_intake_tenant(&req, Some(&store)).await;

    assert_eq!(resolved, tenant_a.to_string(), "store key must be A's tenant");
    assert_ne!(
        resolved,
        tenant_b.to_string(),
        "X-Tenant-Id:B must NOT redirect the intake store key"
    );

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Acceptance 1 (store contents) — driving a turn under the JWT-resolved
/// tenant writes ONLY A's intake row; B's store is untouched even though the
/// request carried `X-Tenant-Id: B`. Asserts store contents, not just status.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn cross_tenant_header_cannot_read_or_poison_b_intake() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let session = session_store().await;
    let tenant_a = provision(&session, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&session, "b@beta.test", "Beta B").await;

    let jwt = mint_session(&tenant_a);
    let req = intake_req(&jwt, Some(&tenant_b));
    let resolved = resolve_intake_tenant(&req, Some(&session)).await;

    // Drive the turn against a temp intake store under the resolved tenant —
    // exactly what `respond` does, minus the global default dir.
    let (_d, intake) = store();
    run_turn(&intake, &resolved, "c1", Some("Be my support agent"), false, None)
        .await
        .expect("turn");

    // A's row exists; B's was never created (no read, no poison).
    assert!(
        intake.load(&tenant_a.to_string(), "c1").is_some(),
        "tenant A's intake row must exist under A's key"
    );
    assert!(
        intake.load(&tenant_b.to_string(), "c1").is_none(),
        "tenant B's intake store must be untouched by A's cross-tenant header"
    );

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

/// Acceptance 2 — a no-header authed request keys on the JWT tenant, NOT the
/// cwd/config-default tenant the old `extract_tenant` collapsed everyone onto.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn no_header_authed_request_keys_on_jwt_tenant_not_default() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", TEST_SECRET);

    let store = session_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;

    // The tenant a no-session (anonymous/demo) request would resolve to.
    let no_session = resolve_intake_tenant(
        "POST /api/v1/genesis/conversation HTTP/1.1\r\n\r\n",
        Some(&store),
    )
    .await;

    let jwt = mint_session(&tenant_a);
    let req = intake_req(&jwt, None);
    let resolved = resolve_intake_tenant(&req, Some(&store)).await;

    assert_eq!(resolved, tenant_a.to_string(), "must key on the JWT tenant");
    assert_ne!(
        resolved, no_session,
        "authed request must NOT collapse onto the cwd/config-default tenant"
    );

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}

#[tokio::test]
async fn confirm_during_required_followup_defers_instead_of_bypassing() {
    // Adversarial-review P1 (2026-06-10): the bypass window opens when the
    // LLM delta fills ALL required slots from one rich answer while the
    // quality follow-up for a vague slot is still pending — `is_complete()`
    // is true, so the old gate would have honored `confirm` and silently
    // skipped the follow-up. Seed that exact persisted state (the wizard
    // path can't reach it one slot at a time), then confirm.
    let (_d, store) = store();
    let conv = "bypass";
    let mut state = argo_genesis::IntakeState {
        charter: Some("Be my support agent for the queue".to_string()),
        persona: Some("Sam".to_string()),
        responsibilities: vec!["Answer tier-1 tickets".to_string()],
        cadence: Some("Weekly".to_string()),
        exceptions: vec!["never issue a refund alone".to_string()],
        ..argo_genesis::IntakeState::default()
    };
    state.apply_raw_answer(
        IntakeSlot::SuccessCriteria,
        "first response under five minutes",
    );
    state.pending_slot = Some(IntakeSlot::Cadence);
    state.followups_used.push(IntakeSlot::Cadence);
    store.save("op-a", conv, &state).expect("seed state");

    // Confirm while the follow-up is pending -> REFUSED: the follow-up is
    // re-asked, not abandoned; not done.
    let v = turn(&store, "op-a", conv, None, true).await;
    assert_eq!(v["status"], "asking", "confirm deferred, not honored");
    assert_eq!(v["slot"], "cadence", "the follow-up is re-asked");
    assert_eq!(v["can_confirm"], false);
    // One answer (even vague again - the cap accepts it) unblocks confirm.
    let v = turn(&store, "op-a", conv, Some("Weekly"), false).await;
    assert_eq!(v["slot"], "access", "cap honored; walk advances to optionals");
    assert_eq!(v["can_confirm"], true);
    let done = turn(&store, "op-a", conv, None, true).await;
    assert_eq!(done["status"], "done");
}
