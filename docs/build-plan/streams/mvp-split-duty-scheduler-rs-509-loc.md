# mvp-split-duty-scheduler-rs-509-loc

**Authored against**: origin/main `9d9f8fd40` on `2026-05-24`.
**Validate before claim**: bash scripts/check-brief-freshness.sh mvp-split-duty-scheduler-rs-509-loc

## Why this brief exists

`crates/argo-orchestrator/src/duty_scheduler.rs` is currently 509 LOC on `origin/main` — 9 LOC over the 500-LOC ceiling that CLAUDE.md lists as a non-negotiable invariant and that `scripts/conductor-pre-merge-gate.sh` gate 2 enforces. The file crossed the ceiling when PR #2088 (`fix-duty-scheduler-invokes-agent-runner`) landed `ece609817` — the AgentRunner-invoke + WorkerDutyFired audit emission added ~58 LOC to the file. The previous split (`#1170`, "split duty_scheduler.rs below 500-LOC ceiling") brought the file from ~700 down to ~470, but iterative additions since have re-breached.

Today the breach blocks the pre-merge gate from passing cleanly: every conductor tick has to run `--skip-clippy` and manually verify the breach is pre-existing. That coupling makes regression detection harder than it needs to be.

## Acceptance criteria

1. `wc -l crates/argo-orchestrator/src/duty_scheduler.rs` returns ≤ 450 (gives ~50 LOC of headroom for future additions before the next split is needed).
2. `bash scripts/conductor-pre-merge-gate.sh` passes gate 2 (LOC budget) cleanly without the breach.
3. `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean.
4. `cargo test -p argo-orchestrator` passes (all existing tests including `claim_2_live_duty_runs.rs` integration test).
5. All extracted modules are private (`pub(crate)`) and re-exported from `duty_scheduler.rs` if they need to remain part of the existing `DutyScheduler` public surface.
6. Every modified file < 500 LOC.

## Parallel-surface declarations

- No new `pub struct/enum/type` declarations crossing the crate boundary. Internal `pub(crate)` is fine.
- Existing related crates: `argo-orchestrator` only. No cross-crate ripples expected.

## Suggested split

The current file mixes ~4 concerns. Suggested module shape (worker may deviate based on actual call-graph):

- `duty_scheduler.rs` (≤300 LOC) — `DutyScheduler` struct, `new`, `tick`, `spawn_duty_scheduler` autonomous loop primitive.
- `duty_scheduler/agent_invocation.rs` (~120 LOC) — the AgentRunner invoke + `WorkerDutyFired` audit-emission helper from #2088.
- `duty_scheduler/cron_eval.rs` (~80 LOC) — the cron-expression evaluation + next-fire-time computation.

Worker should pre-flight `git grep -nF 'duty_scheduler::' -- crates/` to enumerate every external caller and ensure none reference internal symbols that would move.

## May-touch

- `crates/argo-orchestrator/src/duty_scheduler.rs`
- `crates/argo-orchestrator/src/duty_scheduler/` (NEW dir, sub-modules)
- `crates/argo-orchestrator/src/lib.rs` if module-tree edits are needed
- Any test file under `crates/argo-orchestrator/tests/` that needs an import-path update

## Must NOT touch

- `crates/argo-orchestrator/src/*` other than `duty_scheduler.rs` and the new sub-dir.
- Any consumer crate (`argo-agent-runner`, `argo-cli`, `argo-gateway`).
- The DutyScheduler public API surface — re-export from `duty_scheduler.rs` so all `use argo_orchestrator::DutyScheduler` and `use argo_orchestrator::duty_scheduler::*` paths still resolve.

## LOC budget

~150-300 LOC (pure refactor; net change should be close to zero, just moved across files).

## Priority

🟢 P2 LOC ceiling hygiene. Not demo-blocking, but every conductor tick currently runs the pre-merge gate with `--skip-clippy` due to this breach.

## Source

Pre-merge-gate output 2026-05-24 (tick-115 conductor surface):

```
[pre-merge-gate] [2/5] LOC budget (files > 500)
[pre-merge-gate FAIL] files over 500 LOC ceiling:
     509 crates/argo-orchestrator/src/duty_scheduler.rs
```
