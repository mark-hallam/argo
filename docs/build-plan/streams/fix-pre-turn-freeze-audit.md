# fix-pre-turn-freeze-audit

**Owner**: claude or codex — audit completeness; conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `de0e01ea3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-pre-turn-freeze-audit

## Problem (conductor-verified + FINDINGS-ONLY test 3.2 pins this)

A frozen worker's pre-turn rejection emits **zero audit row**. Verified by
direct read: `crates/argo-agent-runner/src/turn_run_loop.rs` runs the
pre-turn freeze gate at lines 57-69 and `return Err(ArgoError::Invariant(
freeze_message(...)))` at line 65 — but the `AuditSink` is only constructed
at line 80, AFTER the freeze check. So a kill-switch/freeze block leaves no
trace on the tamper-evident trail (the operator can't see that a frozen
worker was prevented from acting). The per-dispatch path
(`turn_helpers_freeze.rs`) emits a freeze audit; the pre-turn path does not.

The pinned test `eval_findings_audit_chain.rs:41` (`#[ignore]
"FINDINGS-ONLY: pins finding 3.2 (pre-turn freeze must produce audit row)"`)
is waiting.

## Fix

Hoist the `AuditSink` construction ABOVE the freeze check (it only needs
`store` + `session.tenant_id`, both available earlier), and emit a
`WorkerFreezeBlocked` (or the existing freeze audit kind the per-dispatch
path uses — grep `turn_helpers_freeze.rs` for the kind, REUSE it) row
before the `return Err`. Un-ignore test 3.2.

## May touch

- `crates/argo-agent-runner/src/turn_run_loop.rs`
- `crates/argo-agent-runner/tests/eval_findings_audit_chain.rs` (un-ignore 3.2)

## Acceptance / verifier

1. Test 3.2 un-ignored + GREEN: a pre-turn run against a frozen worker
   produces exactly one freeze audit row before the rejection.
2. Hoisting the sink does not change the happy path (no double-emit);
   `cargo test -p argo-agent-runner` green; clippy clean.

~40-80 LOC. **🔴 P1 — audit completeness on the kill-switch path.**
