//! FINDINGS-ONLY (`#[ignore]`'d) audit-chain integrity tests
//! produced by `eval-argo-agent-runner-turn-lifecycle` (the
//! 2026-05-23 turn-lifecycle audit). Each test pins an
//! invariant that is MISSING on `origin/main` at `5a8726465`.
//!
//! The follow-up fix-briefs (`fix-runner-audit-transactional`
//! and `fix-pre-turn-freeze-audit`) un-ignore the relevant
//! test.

#![allow(unused_imports)]

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

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 3.2 (pre-turn freeze must produce audit row)"]
async fn pre_turn_freeze_rejection_must_emit_worker_freeze_blocked() {
    // Scenario: freeze worker_a, then start a turn against a
    // session bound to worker_a. The turn rejects (correct)
    // but currently emits ZERO `WorkerFreezeBlocked` audit
    // rows — the pre-turn freeze check at
    // `turn_run_loop.rs:57-71` returns Err BEFORE the audit
    // sink is constructed (line 80).
    //
    // Expected behavior post-fix: exactly one
    // `WorkerFreezeBlocked` row at the pre-turn gate, with
    // `phase: "pre_turn"`, plus the existing per-dispatch
    // `phase: "pre_dispatch"` row for any tool call.
    panic!(
        "FINDINGS-ONLY: pending fix-pre-turn-freeze-audit. \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §3.2."
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
