# Round 53 — Claim 2 (Delegation-as-unit) cold cargo regression verify

**Date**: 2026-05-24 (UTC) / 2026-05-25 09:35 AEST
**Round**: 53
**Pillar / Claim**: Claim 2 (Delegation is the unit) — "duty cron fires → runner → `WorkerDutyFired` audit row in 60s" (MISSION.md operator-locked round-15 criterion)
**Verdict**: 🟢 **PASS** — 3/3 cold cargo tests green

## Setup

Cold build with fresh CARGO_TARGET_DIR against `origin/main` HEAD `8b454f505`:

```bash
CARGO_TARGET_DIR=/tmp/r53-claim2-target cargo test -p argo-orchestrator --test claim_2_live_duty_runs --no-run
```

Then test run:

```bash
CARGO_TARGET_DIR=/tmp/r53-claim2-target cargo test -p argo-orchestrator --test claim_2_live_duty_runs
```

Cold build: **11m 11s** (compiler-finished message + executable produced). System load average was 64/87/90 from 3 concurrent worker `cargo` processes — wall-clock would be ~2-4min on an idle Mac. Test run: **0.51s** post-build.

## Observed

```
running 3 tests
test claim_2_live_single_tick_writes_worker_duty_fired_audit_row ... ok
test claim_2_live_cross_tenant_isolation_on_audit_row ... ok
test claim_2_live_spawn_duty_scheduler_writes_audit_row_within_budget ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
```

All 3 `#[tokio::test]` cases passed. Cold finish 0.51s. Last touched in eval at r37 + r47 cluster verifications (post-merge of #2088 + #2137 chain on 2026-05-23) — **6-rounds-cold in pure evaluation cycle** (r48-r52 untouched).

## What the test pins

Per `crates/argo-orchestrator/tests/claim_2_live_duty_runs.rs:1-78` docstring, this is the canonical mission-level pin for the round-15 SLA. Three independent contracts:

### Pin 1 — `claim_2_live_single_tick_writes_worker_duty_fired_audit_row` (lines 158-237)

Deterministic single-tick path:
- Builds in-mem `(DutyStore, OrchestratorStore, SessionStore)` triple with `migrate()` applied
- Inserts a 60-second-interval `DutyRecord` with `created_at = 2026-05-06T12:00:00Z`
- Drives `scheduler.tick(2026-05-06T12:01:30Z)` (one boundary crossed)
- Asserts:
  - `report.fired == vec![duty.id]` + `report.run_failed.is_empty()`
  - `runner.calls.len() == 1` (runner invoked exactly once)
  - Recorded call: `tenant_id`, `worker_id`, `duty_id`, `cron_spec`, `entry_action`, `fired_at` all match
  - `duties.sessions_for_duty(tenant, &duty.id)` returns exactly 1 attribution
  - `audit_record` table has exactly 1 `worker_duty.fired` row
  - Payload fields: `tenant_id`, `worker_id`, `duty_id`, `session_id`, `cron_spec`, `entry_action`, `fired_at` all present; `error` key MUST be absent on success

### Pin 2 — `claim_2_live_spawn_duty_scheduler_writes_audit_row_within_budget` (lines 251-300)

Autonomous-loop primitive over tight wall-clock budget:
- Uses the production `spawn_duty_scheduler` helper that gateway boot binds (`crates/argo-cli/src/handlers/gateway/boot_spawns.rs:172`) — the only difference is the tick interval (250 ms here vs. 60 s in production)
- `cron = "* * * * * *"` (every second) + `created_at = Utc::now() - 2s` guarantees next boundary is "now or earlier"
- Polls audit table with 50ms cadence, up to 3s budget
- Asserts `worker_duty.fired` audit row lands within budget — error message distinguishes "tick never matched" (runner never called) vs "runner ran but audit row never landed" (regression against round-15 SLA)

### Pin 3 — `claim_2_live_cross_tenant_isolation_on_audit_row` (lines 308-421)

Cross-tenant isolation under live wire:
- Two tenants insert duties with identical cron expression + created_at
- One scheduler, one sink (sink's bound tenant is a **third** tenant — would notice mistaken stamp from sink default)
- One tick crosses both boundaries
- Asserts:
  - `report.evaluated == 2`, `report.fired.len() == 2`
  - Each tenant owns exactly 1 attribution row → exactly 1 session_id
  - `session_a != session_b`
  - Payload `tenant_id`, `worker_id`, `duty_id` match each tenant's record
  - **Cross-tenant leak guards**: `payload_a["worker_id"] != payload_b["worker_id"]`, same for `duty_id` and `session_id`
  - `audit_record.tenant_id` **column** (not just payload field) reads back as the duty's tenant, not the sink's bound default — proves cross-tenant guard is on the column-level write, not just the JSON payload

## Anti-theater check

| Risk | How verified |
|---|---|
| Test rewrites itself green | Static surface check at `crates/argo-orchestrator/src/lib.rs:80-85` confirms `spawn_duty_scheduler`, `DutyAgentRunner`, `DutyRecord`, `DutyRunContext`, `DutyScheduler`, `DutyStore`, `OpenGate`, `OrchestratorStore` are still all `pub use`-exported on `origin/main`. None have been renamed or hidden. |
| Substrate regressed silently | `crates/argo-orchestrator/src/duty_scheduler.rs:161-187` still has `with_agent_runner` + `with_audit_sink` + `tick` with current signatures matching the test's invocations. `spawn_duty_scheduler` at line 380 still exists. Crate-internal `duty_scheduler_tests.rs:241-405` still has the unit-level pins (`tick_invokes_runner_with_duty_context_on_auto_approve`, `tick_emits_worker_duty_fired_audit_row_with_payload`, `tick_emits_worker_duty_failed_when_runner_errors`, `tick_continues_after_runner_failure_for_subsequent_duties`). |
| Gateway boot still NoopRunner | **No** — gateway now wires a **real** `GatewayDutyAgentRunner` (`crates/argo-cli/src/handlers/gateway/boot_spawns.rs:206-220+`), not the Noop placeholder the test docstring warned about. **Progressive substrate hardening**: the test's "What this file does NOT pin" section is now over-cautious — the gateway-boot wire flip cited (follow-up brief `mission-duty-scheduler-gateway-boot-wire-real-runner-and-sink`) has shipped. |
| Vacuous pass on cross-tenant pin | The sink's bound tenant is a deliberate **third** tenant, so a regression that stamped `audit_record.tenant_id` from the sink instead of the duty would produce a value that matches NEITHER tenant_a NOR tenant_b — `assert_eq!(tenant_column_a, tenant_a.0.to_string())` would FAIL. Plus `assert_ne!(payload_a["worker_id"], payload_b["worker_id"])` would catch a regression that mixed up the duty→session→audit row plumbing. |

## CLAUDE.md status check

> Claim 2 (Delegation is the unit) — 🟢 **LIKELY-LIVE** (#2088 + #2137 merged 2026-05-23; needs cold rerun of `claim_2_live_duty_runs` to confirm no flakes)

**Round 53 round-trip confirms: not flaky. All 3 tests cold-cargo green 0.51s.** CLAUDE.md status can be upgraded from 🟢 LIKELY-LIVE to 🟢 LIVE on the basis of:
- 6-rounds-cold zero-touch in pure evaluation cycle
- This run + r37 cluster verification + r47 closure verification
- Progressive substrate hardening (gateway boot now uses real runner, not Noop)

**Recommend operator approval to flip CLAUDE.md Claim 2 from "LIKELY-LIVE" to "LIVE"** (this is operator-locked, evaluator does not flip).

**36th anti-theater-honest verification this week.**

## Gap / follow-up brief candidates

**None new** from this scenario. All carry-forward gaps tracked in sibling drift-sweep report.

The PR #2088 follow-up `mission-duty-scheduler-gateway-boot-wire-real-runner-and-sink` is now ✅ shipped (verified via boot_spawns.rs:172,200 + GatewayDutyAgentRunner struct on `origin/main`). Test docstring's deferral note at lines 56-66 is stale; that's a P3 docs-edit candidate but not worth its own brief (worker who next touches the test can update the docstring inline).
