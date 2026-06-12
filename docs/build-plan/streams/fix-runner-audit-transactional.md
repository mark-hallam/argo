# fix-runner-audit-transactional

**Owner**: claude or codex — runner hot-path + audit integrity → conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `de0e01ea3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-runner-audit-transactional

## Problem (FINDINGS-ONLY test 3.1 already pins this)

The runner's per-round `append_message` (turn mutation) and `audit.write`
(hash-chained audit row) are SEPARATE awaits, not one transaction — so a
crash/error between them leaves the turn mutated with no audit row, or an
audit row with no mutation. The hash-chained tamper-evident audit trail is
a core product differentiator; a mutation that escapes the chain is an
integrity hole.

Evidence: the pairs are split across
`crates/argo-agent-runner/src/turn_helpers_run.rs` (`append_message` at
:275/:388 + `audit.write` at :295/:349/:378/:467) and `turn_run_loop.rs`
(User `append_message` :74 + TurnStart/TurnEnd audit). The pinned test
`crates/argo-agent-runner/tests/eval_findings_audit_chain.rs:13`
(`#[ignore] "FINDINGS-ONLY: pins finding 3.1 (audit writes must be in-tx
with mutation)"`) is waiting for this fix.

## Fix

Make each mutation+audit pair atomic: add a transactional overload (the
mutation and the audit `INSERT` share one SQLite transaction that commits
together or rolls back together). Mirror the established shape from
**#2078** (`mission-workflow-audit-write-transactional`) which applied
exactly this to the workflow runner — REUSE that pattern, do not invent a
parallel tx helper. Update the ~4 round sites; un-ignore test 3.1.

**Related sibling (fast-follow, NOT this brief):** finding 3.3
(`eval_findings_audit_chain.rs:62` — "swallowed audit-write failures must
be counted") is adjacent; if the tx refactor naturally surfaces the
swallowed-error count, close it here and note it; otherwise leave 3.3 for
its own row.

## May touch

- `crates/argo-agent-runner/src/turn_helpers_run.rs`, `turn_run_loop.rs`
- the store mutation/audit helpers they call (transactional overload)
- `crates/argo-agent-runner/tests/eval_findings_audit_chain.rs` (un-ignore 3.1)

## Acceptance / verifier

1. Test 3.1 un-ignored + GREEN: a forced audit-write failure rolls back the
   paired turn mutation (no orphan mutation), proven by the test.
2. `cargo test -p argo-agent-runner` green; clippy clean; no regression in
   the round/turn happy path.

~150-250 LOC. **🔴 P1 — audit-chain integrity on the runner hot path.**
