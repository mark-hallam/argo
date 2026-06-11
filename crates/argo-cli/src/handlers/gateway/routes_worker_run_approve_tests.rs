//! "Approve & run" keystone-fix tests — the run body's optional
//! `approve_tools` list mints ONE-SHOT session grants on the NEW run's
//! session before the turn, so the approve→run loop converges instead
//! of holding the same tools on every fresh session.
//!
//! Child module of `routes_worker_run_tests`, so `use super::*` reuses
//! the fixtures there (env guard, store, worker seeding, req builder).
//! The runs drive the REAL `process_job` engine via the mock-provider
//! seam (no network); the assertions read the grant + audit rows back
//! from the store — never anyone's say-so.
//!
//! The grant→dispatch interplay itself (a grant lets a held probation
//! call through and a one-shot is consumed after dispatch) is pinned at
//! the decision layer in `argo_agent_runner::turn_helpers_gate`
//! (`operator_grant_lets_a_probation_action_through_and_consumes`) and
//! at the store layer in `argo_store::grants_tests`. These tests pin
//! the NEW seam: the grants genuinely land on the run's own session,
//! one-shot, tenant+session-scoped, and ONLY for tools the worker
//! legitimately holds.

use super::*;
use argo_identity::PROBATION_PROVENANCE_PREFIX;
use argo_types::SessionId;
use uuid::Uuid;

/// Seed a worker whose persona carries an ACTIVE probation window
/// (`probation_until` 1h in the future) — the same marker the Genesis
/// hire chokepoint stamps. The probation gate is exactly the hold this
/// fix lets the operator clear in one action.
async fn add_probation_worker(store: &Arc<SessionStore>, tenant: TenantId) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        super::RUNNABLE_VERTICAL,
        Persona::named("Ava", "human:mark"),
        Role::new("Read invoice files and draft reminders."),
    );
    let until = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    worker
        .persona
        .provenance_chain
        .push(format!("{PROBATION_PROVENANCE_PREFIX}{until}"));
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create probation worker");
    worker
}

/// Active grants on the run's session, via the same store read the
/// agent runner uses at TurnStart.
async fn grants_for(
    store: &Arc<SessionStore>,
    session_id: &str,
) -> Vec<argo_store::SessionGrant> {
    let sid = SessionId(Uuid::parse_str(session_id).expect("session uuid"));
    store
        .list_session_grants(sid)
        .await
        .expect("list session grants")
}

/// Count of `approval.granted` audit rows written for the session.
async fn approval_audit_rows(store: &Arc<SessionStore>, session_id: &str) -> i64 {
    let sid = Uuid::parse_str(session_id).expect("session uuid");
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE session_id = ? AND kind = 'approval.granted'",
    )
    .bind(sid.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .expect("count approval.granted rows")
}

/// THE KEYSTONE: a probationary worker run WITH `approve_tools` mints
/// one ONE-SHOT grant per requested in-tool-set tool on the NEW run's
/// session — so the turn executes with the operator's approval in place
/// (the gate cascade dispatches a granted probation call; pinned in
/// `turn_helpers_gate`), instead of holding the same tools again.
#[tokio::test]
async fn approve_tools_mints_one_shot_grants_on_the_new_run_session() {
    let (env, _w) = setup_with_worker("Mariah").await;
    let worker = add_probation_worker(&env.store, env.tenant).await;
    // `file_read` + `web_search` are in customer-support's
    // `[tools].allowed` — exactly what a held run would report.
    let body = r#"{"approve_tools":["file_read","web_search"]}"#;
    let req = run_req(&worker.id, Some("owner"), body);
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(
        v["approved_tools"],
        serde_json::json!(["file_read", "web_search"]),
        "response reports the minted approvals; body: {resp}"
    );
    let session_id = v["session_id"].as_str().expect("session_id");
    assert!(!session_id.is_empty(), "a real turn ran");

    let grants = grants_for(&env.store, session_id).await;
    assert_eq!(grants.len(), 2, "one grant per approved tool: {grants:?}");
    let mut tools: Vec<&str> = grants.iter().map(|g| g.tool_name.as_str()).collect();
    tools.sort_unstable();
    assert_eq!(tools, vec!["file_read", "web_search"]);
    for g in &grants {
        assert!(
            g.is_one_shot,
            "grants must be ONE-SHOT (consumed by this turn, never a permanent elevation): {g:?}"
        );
        assert_eq!(g.source, "run_now_approve");
        assert!(g.consumed_at_secs.is_none(), "active until the turn spends it");
    }
    // Forensic chain: one ApprovalGranted audit row per grant, on the
    // run's own session (same kind the standalone approve endpoint emits).
    assert_eq!(approval_audit_rows(&env.store, session_id).await, 2);
}

/// No privilege escalation: a requested tool the worker does NOT
/// legitimately hold — unknown (`wire_funds`) or manifest-denied
/// (`terminal`) — is SKIPPED: no grant minted, run not errored.
#[tokio::test]
async fn approve_tools_outside_worker_tool_set_are_not_granted() {
    let (env, _w) = setup_with_worker("Mariah").await;
    let worker = add_probation_worker(&env.store, env.tenant).await;
    let body = r#"{"approve_tools":["file_read","wire_funds","terminal"]}"#;
    let req = run_req(&worker.id, Some("owner"), body);
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "skipping must not error the run; body: {resp}");
    let v = parse(&resp);
    assert_eq!(
        v["approved_tools"],
        serde_json::json!(["file_read"]),
        "only the in-tool-set tool is approved; body: {resp}"
    );
    let session_id = v["session_id"].as_str().expect("session_id");
    let grants = grants_for(&env.store, session_id).await;
    assert_eq!(grants.len(), 1, "exactly one grant: {grants:?}");
    assert_eq!(grants[0].tool_name, "file_read");
    assert!(
        !grants.iter().any(|g| g.tool_name == "wire_funds" || g.tool_name == "terminal"),
        "neither the unknown nor the denied tool may ever be granted"
    );
}

/// Regression: a run WITHOUT `approve_tools` mints nothing — the
/// probation hold posture is exactly what it was before this fix.
#[tokio::test]
async fn run_without_approve_tools_mints_no_grants() {
    let (env, _w) = setup_with_worker("Mariah").await;
    let worker = add_probation_worker(&env.store, env.tenant).await;
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(v["approved_tools"], serde_json::json!([]));
    let session_id = v["session_id"].as_str().expect("session_id");
    assert!(grants_for(&env.store, session_id).await.is_empty());
    assert_eq!(approval_audit_rows(&env.store, session_id).await, 0);
}

/// One-shot scope: the approval is bound to THE run it rode in on. A
/// second run (no `approve_tools`) gets a FRESH session with ZERO
/// grants — combined with the gate-decision pin that a grant-less
/// probation call holds, the approval cannot leak across runs.
#[tokio::test]
async fn second_run_without_approve_tools_gets_a_grant_free_session() {
    let (env, _w) = setup_with_worker("Mariah").await;
    let worker = add_probation_worker(&env.store, env.tenant).await;
    let approved = run_req(
        &worker.id,
        Some("owner"),
        r#"{"approve_tools":["file_read"]}"#,
    );
    let (s1, r1) = post_run(&worker.id.to_string(), &approved, Some(&env.store)).await;
    assert_eq!(s1, "200 OK", "body: {r1}");
    let first_session = parse(&r1)["session_id"].as_str().expect("sid 1").to_string();
    assert_eq!(grants_for(&env.store, &first_session).await.len(), 1);

    let bare = run_req(&worker.id, Some("owner"), "{}");
    let (s2, r2) = post_run(&worker.id.to_string(), &bare, Some(&env.store)).await;
    assert_eq!(s2, "200 OK", "body: {r2}");
    let second_session = parse(&r2)["session_id"].as_str().expect("sid 2").to_string();
    assert_ne!(first_session, second_session, "every run is a fresh session");
    assert!(
        grants_for(&env.store, &second_session).await.is_empty(),
        "the first run's approval must not follow the worker to the next session"
    );
}
