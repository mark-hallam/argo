# Round 30 — P0 cu-macos workspace-build blocker still open after 14h

- **Date**: 2026-05-23
- **Round**: 30
- **Pillar/Claim**: Process / operations (blocks workspace-wide cargo coverage on every claim)
- **Verdict**: 🔴 **FAIL — P0 STILL OPEN**

## Setup

Round-26 evaluator (2026-05-23T20:00Z) first surfaced this: `cargo test
--workspace --no-run` fails on origin/main because
`crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:139`
constructs `MacosCuRunRequest` without the `tenant_id` and `worker_id`
fields that PR #2076 added to the struct. Conductor tick-96
(2026-05-23T06:55Z) authored the unblock brief
`fix-cu-macos-permission-revoke-test-add-tenant-worker-fields` to fix
it. ~14 hours have passed since the brief landed.

This round I re-ran the build cold to confirm whether anything has
shipped to close the brief.

```
git stash push -- crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs
cargo check -p argo-executor-computer-use-macos --tests
```

(Stashed first because the session started with an *uncommitted* WIP
diff on that exact file in the working tree — appears to be a worker's
in-flight fix that never made it to a PR. I am not allowed to touch
code per my evaluator charter, so I stashed it to test main as-is.)

## Observed

```
Checking argo-executor-computer-use-macos v0.0.1 (...)
error[E0063]: missing fields `tenant_id` and `worker_id` in initializer of `MacosCuRunRequest`
   --> crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:139:5
    |
139 |     MacosCuRunRequest {
    |     ^^^^^^^^^^^^^^^^^ missing `tenant_id` and `worker_id`

For more information about this error, try `rustc --explain E0063`.
error: could not compile `argo-executor-computer-use-macos` (test "permission_revoke_mid_run_test") due to 1 previous error
```

Compilation still fails. No PR has merged to fix the 2-line edit.

The required fix is exactly:

```rust
MacosCuRunRequest {
    tenant_id: argo_types::TenantId::new(),       // ← add
    worker_id: argo_identity::WorkerId::new(),    // ← add
    action_id: "act-permission-revoke".into(),
    ...
}
```

— the diff is already present in the working tree as untracked WIP.
Someone started the fix and either hit a permission prompt that bounced
them out of the worktree, or got interrupted.

## Anti-theater check

This is a pure compilation failure — `cargo check -p
argo-executor-computer-use-macos --tests` either succeeds or doesn't.
The output above is rustc's verbatim error, not a wrapper script's
opinion. The fix-by deadline sweep
(`scripts/check-findings-only-fix-by.sh`) does NOT list this brief at
all — meaning the unblock brief either was tagged without a `fix_by:`
field, or was promoted at tick-92 alongside the 5 LIVE-test briefs but
the deadline tracking missed it. Either way: the process gate that
*would* have escalated this is silent.

Impact validation: every workspace-wide test invocation (CI, workers,
conductor's `cargo test --workspace`) bombs at the build phase before
running a single test. Selective per-crate test runs (like
`cargo test -p argo-cli --bin argo ...` I ran in scenario 2) still
work, so per-target evaluator + per-PR worker pipelines are intact —
but no global cargo-sweep gates are running, and the `cargo check
--workspace` admin-merge gate (per tick-96 escalation) is blocked.

## Gap / follow-up

The brief `fix-cu-macos-permission-revoke-test-add-tenant-worker-fields`
exists but appears either UNCLAIMED or zombie-claimed (no PR opened
yet). The conductor tick-97 heartbeat at `9eab9fc81` notes "2
stale-DIRTY PRs, queue healthy at 33 TODO, no replenish" — does NOT
list this P0 in the stale-DIRTY count.

**Recommended next conductor action**:

1. **Verify brief status** — `cat
   streams/fix-cu-macos-permission-revoke-test-add-tenant-worker-fields.md`
   and `bash scripts/find-stale-claim-drift.py | grep cu-macos` —
   determine if the row is TODO, CLAIMED, or zombie.
2. **If TODO**: promote to the top of the queue and tag fast-mode
   (5-LOC mechanical fix, codex-pool capable per memory).
3. **If CLAIMED but stale**: release the claim and re-author with a
   sharper acceptance test ("`cargo check --workspace` exits 0").
4. **Consider admin-merging the WIP diff in the working tree** —
   it is the exact 2-line fix the brief calls for. The conductor
   can stage + commit + push via a worker session.

**No new brief needed** — the unblock brief already exists. This is a
process drift surface, not a missing brief.

## Operational

- gateway DOWN
- 5 fix-by deadlines all 🟢 within budget (5/7 days remaining)
- 2 stale-DIRTY PRs per tick-97 (separate concern)
- 33 TODO queue
