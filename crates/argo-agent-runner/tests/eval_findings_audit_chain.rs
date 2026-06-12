//! FINDINGS-ONLY (`#[ignore]`'d) audit-chain integrity tests
//! produced by `eval-argo-agent-runner-turn-lifecycle` (the
//! 2026-05-23 turn-lifecycle audit). Each test pins an
//! invariant that is MISSING on `origin/main` at `5a8726465`.
//!
//! The follow-up fix-briefs (`fix-runner-audit-transactional`
//! and `fix-pre-turn-freeze-audit`) un-ignore the relevant
//! test.

#![allow(unused_imports)]

use argo_agent_runner::run_turn_with_events;
use argo_store::{SessionCreate, SessionStore, WorkerId};
use argo_types::{ArgoError, SessionId};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 3.1 (audit writes must be in-tx with mutation)"]
async fn append_message_and_audit_write_must_share_transaction() {
    // Scenario: kill the process between
    // `store.append_message(Assistant, ...)` and
    // `audit.write(AssistantMessage, ...)`. On restart the
    // messages table should NOT carry an orphan row that has
    // no corresponding audit_record.
    //
    // Today the audit sink uses a `BatchedWriter` whose flush
    // is on a background task; the assistant-message INSERT
    // and the audit-record INSERT live in two distinct
    // transactions. The `mission-workflow-audit-write-
    // transactional` shape from #2078 fixes the same gap for
    // workflow_audit; the fix brief lifts that pattern to the
    // runner per-round mutation+audit pairs.
    //
    // Test wiring requires `write_in_tx` to be hot-wired into
    // `turn_helpers_run::run_one_round` and
    // `dispatch_one_tool_call`. Pending that, this test stays
    // as a scaffold pinning the invariant.
    panic!(
        "FINDINGS-ONLY: pending fix-runner-audit-transactional. \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §3.1."
    );
}

/// Stand up an in-memory store + workspace + a session whose worker is
/// frozen — the fixture for the pre-turn-freeze-audit pin. Extracted so the
/// test body stays under the 80-LOC fn budget.
async fn setup_frozen_session() -> (Arc<SessionStore>, TempDir, SessionId, WorkerId) {
    let cwd = TempDir::new().expect("tempdir");
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate");
    let (tenant, workspace) = store
        .ensure_workspace("freeze-audit", cwd.path().to_string_lossy().as_ref())
        .await
        .expect("ensure workspace");

    let worker = WorkerId::new();
    let session = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: "test-model",
            provider: "test-provider",
            title: Some("freeze-audit"),
            vertical_id: None,
            worker_id: Some(worker),
        })
        .await
        .expect("create session");

    argo_store::worker_freeze::freeze(
        store.pool(),
        &tenant.to_string(),
        &worker.to_string(),
        "principal:owner",
        "incident response",
    )
    .await
    .expect("freeze worker");

    (store, cwd, session, worker)
}

#[tokio::test]
async fn pre_turn_freeze_rejection_must_emit_worker_freeze_blocked() {
    // Scenario: freeze the session's worker, then start a turn
    // bound to it. The turn rejects (correct) at the pre-turn
    // freeze gate — which fires BEFORE manifest load and any
    // provider work, so no mock server is needed.
    //
    // Post-fix (fix-pre-turn-freeze-audit / finding 3.2): the
    // audit sink is hoisted above the freeze gate, so the
    // rejection now leaves exactly one `WorkerFreezeBlocked`
    // row (`phase: "pre_turn"`) on the tamper-evident trail.
    // Synchronous audit writes so the row is visible without a flush.
    std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");

    let (store, cwd, session, worker) = setup_frozen_session().await;

    // The turn must reject with the cooperative freeze invariant.
    // `TurnResult` is not `Debug`, so match the Result by hand rather
    // than `expect_err`.
    match run_turn_with_events(&store, cwd.path(), session, Some("hello"), false, 1, None).await {
        Err(ArgoError::Invariant(m)) => {
            assert!(m.contains("is frozen by"), "expected freeze invariant, got: {m}");
        }
        Err(other) => panic!("expected freeze invariant, got: {other:?}"),
        Ok(_) => panic!("frozen worker turn must reject, but it succeeded"),
    }
    store.flush_pending_writes().await.expect("flush");

    // Exactly one WorkerFreezeBlocked row, tagged phase=pre_turn.
    let rows = store
        .list_audit_for_session(session)
        .await
        .expect("list audit");
    let freeze_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.kind == "worker_freeze_blocked")
        .collect();
    assert_eq!(
        freeze_rows.len(),
        1,
        "expected exactly one worker_freeze_blocked row, got {}: {rows:?}",
        freeze_rows.len()
    );
    let payload: serde_json::Value =
        serde_json::from_str(&freeze_rows[0].payload_json).expect("payload is json");
    assert_eq!(
        payload.get("phase").and_then(|v| v.as_str()),
        Some("pre_turn"),
        "expected phase=pre_turn in payload, got: {payload}"
    );
    assert_eq!(
        payload.get("worker_id").and_then(|v| v.as_str()),
        Some(worker.to_string().as_str()),
        "expected worker id in payload, got: {payload}"
    );

    // The pre-turn gate fires before any provider/tool work, so it is
    // the ONLY freeze row — no TurnStart, no per-dispatch row.
    assert!(
        rows.iter().all(|r| r.kind != "turn_start"),
        "frozen turn must reject before TurnStart: {rows:?}"
    );
}

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 3.3 (swallowed audit-write failures must be counted)"]
async fn swallowed_audit_write_failures_must_be_observable() {
    // Scenario: simulate audit-write IO failure (e.g.
    // disk-full) during TurnEnd. The turn correctly does NOT
    // fail (the audit emit is best-effort), but today the
    // failure is invisible — no metric, no log at WARN or
    // above, just `let _ = audit.write(...)`.
    //
    // Expected behavior post-fix: an
    // `audit_emit_failure_total{kind="TurnEnd"}` counter
    // increments, surfaceable through the existing Prometheus
    // /OTLP exporter (#938 ground).
    panic!(
        "FINDINGS-ONLY: pending fix-runner-audit-emit-metrics. \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §3.3."
    );
}
