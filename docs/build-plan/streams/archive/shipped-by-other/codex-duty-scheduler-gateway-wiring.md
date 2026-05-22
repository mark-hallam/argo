# V1.3 — Wire DutyScheduler into gateway boot

**Owner**: codex. **Depends on**: claude-worker-duty-scheduler
(#899) merged.

## Scope

#899 landed the `argo-orchestrator::duty_scheduler` module —
`DutyStore`, `DutyScheduler`, `DutyGate` trait, and a public
`spawn_duty_scheduler` tokio task. But that scheduler is currently
**dormant** — nothing in the gateway boot path actually calls
`spawn_duty_scheduler`, so duties registered via `argo duty list/
show/pause/resume/fire-now` exist as data but never fire.

This brief is the missing link: import the scheduler into the
gateway boot path, spawn it once at startup, ensure clean shutdown.
After this lands, the mission claim "works autonomously" goes from
"infrastructure exists" to "loop is actually firing".

## Approach

`argo-orchestrator/src/lib.rs` already re-exports the public API
(`spawn_duty_scheduler`, `DutyScheduler`, `DutyStore`, `DutyGate`,
`ClosedGate`). Gateway just needs to:

1. Add `argo-orchestrator` as a dep in `argo-gateway/Cargo.toml`
   if not already present.
2. In gateway's main run/bootstrap function, after the sqlite pool
   is open: construct a `DutyStore` from the pool, construct a
   `DutyGate` impl (use `ClosedGate` for v1 — fail-safe, don't
   actually fire approval-gated duties until policy hookup is
   verified; document this), call `spawn_duty_scheduler(store,
   gate, ...)`, store the returned `JoinHandle` for shutdown.
3. On shutdown, abort the handle.

The `ClosedGate` v1-default is intentional: until duty firing is
end-to-end-tested against the auto-approval gate (separate brief),
we should NOT have duties trigger arbitrary actions. The scheduler
can run, log fires, and persist `duty_fired_session` rows, but
the gate refuses every action. This is observable + safe. A
follow-up brief will swap `ClosedGate` for `LiveGate` once the
auto-approval integration is verified.

## Files you may touch
- `crates/argo-gateway/Cargo.toml` — narrowly: add
  `argo-orchestrator = { path = "../argo-orchestrator" }`
- `crates/argo-gateway/src/lib.rs` — narrowly: import +
  spawn_duty_scheduler call in the existing run/bootstrap path
- `crates/argo-gateway/tests/duty_scheduler_boot.rs` (NEW) —
  integration test: start a gateway with a single registered
  duty, advance time / trigger a fire, verify a row appears in
  `duty_fired_session` table

DO NOT touch:
- `crates/argo-orchestrator/*` (already correct — just consume its
  public API)
- Any other gateway file unless strictly necessary for the
  spawn-and-shutdown wiring

## Acceptance criteria
- [ ] `argo-gateway` depends on `argo-orchestrator`
- [ ] Gateway boot calls `spawn_duty_scheduler` exactly once
- [ ] Returned `JoinHandle` is held; gateway shutdown aborts it cleanly
- [ ] `ClosedGate` is used by default (v1 fail-safe); a follow-up
      brief will swap to `LiveGate`
- [ ] Integration test: register a duty with a 1-second cron, run
      gateway for 3 seconds, assert a `duty_fired_session` row
      exists
- [ ] `argo doctor` reports duty-scheduler running (extends
      existing doctor checks if such a check pattern exists; OK to
      skip if doctor wiring is out of scope)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-gateway` all clean
- [ ] No regressions in existing gateway tests

## Branch + PR
- Branch: `codex-duty-scheduler-gateway-wiring`
- PR title: `gateway: wire DutyScheduler into boot path (codex-duty-scheduler-gateway-wiring)`
