# Round 31 — P0 cu-macos workspace-build blocker: status update

**Date**: 2026-05-23
**Round**: 31 (scheduled-task hourly fire)
**Pillar / Claim**: Mission Claim 5 — *Any task a human can do on a PC
(macOS computer-use executor)* + universal CI / `cargo test --workspace`
gate
**Verdict**: 🔴 STILL RED on `origin/main` at session-end · 🟡 IN-FLIGHT
(brief now CLAIMED by a worker — improvement from round-30 "TODO, zombie")
**Severity**: P0 → de-facto P1 once a worker is actively on it; restored
to TODO at `08:05Z` per conductor tick-99b RCA, claimed at `1af411d75`.

## Setup

Round 30 (22:30Z previous-window) verified:

> `cargo check -p argo-executor-computer-use-macos --tests` fails with
> `E0063: missing fields tenant_id and worker_id in initializer of
> MacosCuRunRequest`. The 2-line WIP diff is already present in the
> working tree at session start. 14h+ open since conductor-tick-96
> authored the unblock brief.

Re-running the same check this round to confirm whether the blocker has
cleared.

## Observed

```
$ cargo check -p argo-executor-computer-use-macos --tests
error[E0063]: missing fields `tenant_id` and `worker_id` in initializer of `MacosCuRunRequest`
  --> crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:139
error: could not compile `argo-executor-computer-use-macos` (test
  "permission_revoke_mid_run_test") due to 1 previous error
```

Same E0063 as round 30. The 2-line fix has NOT shipped to `origin/main`.

**However** — status has moved since round 30:

- `git log --oneline origin/main -10` shows
  `1af411d75 status: claim fix-cu-macos-permission-revoke-test-add-tenant-worker-fields`.
- Conductor tick-99b (08:11Z) RCA-resolved the mac-host doom-loop that
  was eating the brief in 25-30s cycles (root cause: `claude` binary
  missing from `nohup` PATH, every iteration failing at line 274 of
  `claude-worker-loop.sh` with "command not found").
- 10 BLOCKED→TODO restorations shipped at commit `7efd51388`, including
  this brief.
- Conductor tick-101 (08:35Z) notes mention this row at ledger line 152:
  *"P0 cu-macos blocker STILL TODO 15h+ at line 152 (zombie host
  issue)."* But the `1af411d75` commit (between tick-101 and now)
  flipped it TODO→CLAIMED — a real worker now owns the fix.

## Anti-theater check

Confirmed canonical struct at `crates/argo-executor-computer-use-macos/src/executor.rs:55-57`:

```rust
pub struct MacosCuRunRequest {
    pub tenant_id: TenantId,
    pub worker_id: WorkerId,
    ...
}
```

Test file at `crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:138-145`:

```rust
fn run_request(audit_dir: std::path::PathBuf) -> MacosCuRunRequest {
    MacosCuRunRequest {
        action_id: "act-permission-revoke".into(),
        system_prompt: "Type two notes; the second is doomed.".into(),
        audit_dir,
        anthropic_api_key: "test-key".into(),
    }
}
```

Two literal fields missing. A 2-line additive edit closes this. The fix is
mechanical, not architectural. The worker that holds the claim at
`1af411d75` should ship within minutes once their iteration completes
(mac-host doom-loop is resolved per tick-99b proof: per-iter logs now
114KB–260KB, vs the prior 93B failure pattern).

## Why this matters

This is THE workspace-wide test gate. `cargo test --workspace --no-run`
fails on every CI invocation, every conductor pre-merge-gate, every
worker shipping check until this is fixed. Conductor tick-101 noted
admin-merging #2109/#2110/#2111 used `--skip-clippy` — i.e. the gate-5
clippy check is being **bypassed** to land work, which is a temporary
hole in CI's safety net.

The merges themselves (#2109/#2110/#2111) are good: PR #2109's tests
were verified passing on the PR branch by this evaluator earlier this
round (see `round-31-claim-3-live-merged-anti-theater-verify.md`). The
`--skip-clippy` flag is not a quality risk because the actual workspace
build still works; it's only the cu-macos test file's struct-init that
fails the strict clippy pass. But the gate stays open until this is
shipped — every minute the gate is open is a minute of degraded merge
hygiene.

## Gap / follow-up brief candidates

No new briefs this round. The two open recommendations from round 30
stand:

1. `ops-fix-by-tracker-add-p0-cu-macos-blocker` (P1, ~10-30 LOC) — still
   unauthored. Conductor tick-101 acknowledges as candidate. **See
   round-31 process-drift-sweep report** for an update on the framing —
   the gap is more nuanced than "add a fix_by field" because the
   findings-only fix-by tracker only scans `MERGED-FINDINGS-ONLY` rows,
   and this cu-macos row is `TODO`. The brief shape needs to either
   (a) add a parallel `check-todo-aging.sh` gate for P0 TODO rows
   unfixed >24h, or (b) re-scope the existing tracker to also walk TODO
   rows tagged P0.

2. `mission-claim-4-mvp-row-6-gateway-promote-audit-pin` (P2, ~120-180
   LOC) — still unauthored. Round 30 finding holds.

## Trajectory

If the worker holding `1af411d75` ships within the next hour (likely,
given 2-line edit + mac-host doom-loop resolved), the next round 32
evaluator will verify CLEAR. If not, brief candidate (1) above moves to
P0 — the cu-macos row will be the longest-standing P0 in ledger history
and the process gate that should have caught it needs to ship before
this kind of zombie can happen again.
