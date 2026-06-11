//! Day-1 unblock fix 2 tests — the run-now response surfaces probation
//! holds (`held_tools` + `probation`) so the SPA can tell the user WHY
//! a probationary teammate's turn produced no tool effects.
//!
//! Child module of `routes_worker_run_tests`, so `use super::*` reuses
//! the fixtures there (env guard, store, worker seeding, req builder).

use super::*;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::PROBATION_PROVENANCE_PREFIX;
use argo_types::SessionId;
use uuid::Uuid;

/// Seed a worker whose persona carries an ACTIVE probation window
/// (`probation_until` 1h in the future) — the same marker the Genesis
/// hire chokepoint stamps.
async fn add_probation_worker(store: &Arc<SessionStore>, tenant: TenantId) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        "customer-support",
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

/// Write one `probation.hold` audit row for `(session, tool)` — the
/// exact row shape `argo_agent_runner::turn_helpers_gate::
/// emit_probation_hold` persists when the dispatch gate holds a call.
async fn write_hold_row(store: &Arc<SessionStore>, tenant: TenantId, session: SessionId, tool: &str) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        Some(session),
        AuditKind::ProbationHold,
        serde_json::json!({
            "tool": tool,
            "call_id": "call-1",
            "reason": "probation_window_active",
            "source": "genesis_p3_probation",
        }),
        None,
    )
    .await
    .expect("write probation.hold row");
}

/// A probationary run whose session carries ProbationHold audit rows
/// reports `held_tools` (unique, first-hold order) + `probation: true`
/// in the response JSON. Drives the REAL `finish_run` response path —
/// the holds are read back from the audit rows the gate wrote, never
/// asserted from anyone's say-so.
#[tokio::test]
async fn run_during_probation_reports_held_tools_and_probation_true() {
    let (env, _w) = setup_with_worker("Mariah").await;
    let worker = add_probation_worker(&env.store, env.tenant).await;
    let session = SessionId(Uuid::now_v7());
    // Two holds on the same tool + one on another: the response must
    // de-duplicate while preserving first-hold order.
    write_hold_row(&env.store, env.tenant, session, "file_write").await;
    write_hold_row(&env.store, env.tenant, session, "file_write").await;
    write_hold_row(&env.store, env.tenant, session, "send_email").await;

    let outcome = super::super::routes_worker_run_exec::RunOutcome {
        session_id: session.0.to_string(),
        status: "completed",
        chars: 12,
        approved_tools: Vec::new(),
    };
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = super::super::finish_run(
        &env.store,
        &req,
        &env.tenant,
        &worker,
        "manual_run_now",
        None,
        &outcome,
    )
    .await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(
        v["held_tools"],
        serde_json::json!(["file_write", "send_email"]),
        "unique held tools in first-hold order; body: {resp}"
    );
    assert_eq!(v["probation"], true, "body: {resp}");
}

/// A run with no probation window and no hold rows reports the additive
/// fields as empty/false — existing consumers see only extra keys.
/// Exercises the FULL endpoint (real mock-provider turn via `post_run`).
#[tokio::test]
async fn post_probation_run_reports_empty_held_tools_and_probation_false() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(v["held_tools"], serde_json::json!([]), "body: {resp}");
    assert_eq!(v["probation"], false, "body: {resp}");
    // The pre-existing response contract is untouched (additive only).
    assert!(v["action_id"].is_string());
    assert_eq!(v["status"], "completed");
}

/// `held_tools_for_session` is fail-soft: an unparseable session id (the
/// failed-run path reports `session_id: ""`) yields an empty list, never
/// an error that could fail the response.
#[tokio::test]
async fn held_tools_lookup_is_empty_for_blank_session() {
    let (env, _worker) = setup_with_worker("Mariah").await;
    let held =
        super::super::held_tools_for_session(&env.store, &env.tenant, "").await;
    assert!(held.is_empty());
}
