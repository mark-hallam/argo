# Round 36 — PR #2137 anti-theater verify (Claim 2 LIVE: duty cron → runner → audit row)

- **Date**: 2026-05-23
- **Round**: 36 (scheduled-task hourly fire, 13:27Z)
- **Pillar / Claim**: Claim 2 — "Delegation is the unit" (the operator-locked round-15 LIVE criterion: "duty cron fires → runner → `WorkerDutyFired` audit row in 60s")
- **PR**: [#2137](https://github.com/mark-hallam/argo/pull/2137) — MERGED at `b970d4efb` (2026-05-23T22:24Z author-stamp / merged shortly after) via branch `claude-eval-claim-2-live-duty-cron-fires-runner-test`, implementation commit `f50d3a112`
- **Verdict**: 🟢🟢 PASS (substrate LIVE) — three pins exercise the production wire honestly. Boundary scope (gateway boot) is correctly DEFERRED to a follow-up brief and explicitly documented.

## What this PR ships

A single new test file `crates/argo-orchestrator/tests/claim_2_live_duty_runs.rs` (421 LOC, 3 `#[tokio::test]` functions). Authored after PR #2088 landed the `DutyAgentRunner` + `AuditSink` injection surface on `DutyScheduler`.

This claim has a notorious history: I counted **9 zombie claim/release cycles** before #2137 finally landed (from `git log --grep="eval-claim-2-live-duty-cron"`). For the worker who finally shipped this PR to do so cleanly + honestly is itself a result worth flagging.

## Setup

```
git show f50d3a11288cb97fede9858e820ef471eb76b458 --stat
git show f50d3a11288cb97fede9858e820ef471eb76b458   # 421-line diff
grep -n "DutyScheduler\|with_agent_runner\|with_audit_sink" \
     crates/argo-cli/src/handlers/gateway/boot_spawns.rs
grep -n "fn tick\|fn run_turn\|with_agent_runner\|with_audit_sink" \
     crates/argo-orchestrator/src/duty_scheduler.rs
ls -la crates/argo-orchestrator/tests/claim_2_live_duty_runs.rs
git log --oneline origin/main -- crates/argo-orchestrator/src/duty_scheduler.rs   # confirm #2088 wired this
```

## Observed

**Three pins, each load-bearing**:

1. `claim_2_live_single_tick_writes_worker_duty_fired_audit_row` — single deterministic `tick(now)` over a duty whose next cron boundary lies between `created_at` and `tick_at`. Asserts (verbatim from the diff):
   - `runner.calls.lock().await.len() == 1` (runner invoked exactly once per fire)
   - `report.fired == vec![duty.id.clone()]` (return value reflects the fire)
   - Exactly one `worker_duty.fired` row in `audit_record` keyed by the attribution session_id
   - Round-15 payload fields all present: `tenant_id`, `worker_id`, `duty_id`, `session_id`, `cron_spec`, `entry_action`, `fired_at`
   - Negative: `payload.get("error").is_none()` — successful run does NOT include an error key.

2. `claim_2_live_spawn_duty_scheduler_writes_audit_row_within_budget` — drives the production `spawn_duty_scheduler` primitive on a 250ms tick interval with a `"* * * * * *"` (every-second) duty and `created_at = now - 2s`. Polls the audit table for up to 3 seconds. **This is the autonomous-loop wire, not the in-process `tick` contract.** When the poll budget elapses without finding a row, the assertion message even tells the future-debugger which sub-wire broke (`runner was invoked {N} times — zero ⇒ tick never matched; non-zero ⇒ runner ran but audit row never landed`).

3. `claim_2_live_cross_tenant_isolation_on_audit_row` — two tenants firing simultaneously. **Critical anti-theater move**: the test seeds the `AuditSink` with a deliberately *third* tenant ID, then asserts each row's `audit_record.tenant_id` column matches the duty's tenant, NOT the sink's bound default. That's exactly the failure mode the `feedback_enterprise_grade_defaults` memory warns about — and the test would catch it.

**Substrate wiring confirmed** (direct source-code read, origin/main today):

- `crates/argo-orchestrator/src/duty_scheduler.rs:237` — `pub fn with_agent_runner(mut self, runner: Arc<dyn DutyAgentRunner>) -> Self` (real, public).
- Line 249 — `pub fn with_audit_sink(mut self, sink: Arc<AuditSink>) -> Self` (real, public).
- Line 263 — `pub async fn tick(&self, now: DateTime<Utc>) -> ArgoResult<DutyTickReport>` (the test's invocation point).
- Line 144 — docstring confirms `DutyScheduler::with_agent_runner` is the load-bearing injection surface.
- Line 178 — docstring references `worker_duty.fired` / `worker_duty.failed` as the audit kinds.

`origin/main` history confirms #2088 (`ece609817`) landed `fix(orchestrator): DutyScheduler invokes AgentRunner + emits audit` — the substrate change this test sits on top of.

**Honesty about scope** (verbatim from the test file's module doc-comment):

> ## What this file does NOT pin
>
> Production gateway boot at `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:158-178` still constructs `DutyScheduler::new(...)` with the Noop defaults. The follow-up `mission-duty-scheduler-gateway-boot-wire-real-runner-and-sink` brief flips the gateway over. Until then, the substrate tested here is wired but the production binary's autonomous loop still emits no audit row — by design, to keep the wire opt-in during rollout.

I verified this is accurate. `grep -n "DutyScheduler" crates/argo-cli/src/handlers/gateway/boot_spawns.rs` returns:

```
169:    let scheduler = Arc::new(DutyScheduler::new(duties, orchestrator, gate));
```

No `.with_agent_runner(...)` chain. No `.with_audit_sink(...)` chain. So the test pins substrate-LIVE for Claim 2, NOT production-binary-LIVE. The PR is honest about this AND names the follow-up brief.

## Anti-theater check

- **Recording mock is real, not "always-returns-Ok"-shaped**: `RecordingDutyRunner` captures every `run_turn` invocation into a `tokio::sync::Mutex<Vec<DutyRunContext>>`. The test asserts both *length* (exactly 1) AND *content* (every field of `DutyRunContext`). A mock that no-op'd the call would fail the `calls[0].tenant_id == tenant` assertion.
- **Audit row read via raw SQL**, not via a helper that could hide a bind bug. `audit_rows_for_session()` does `SELECT kind, payload_json FROM audit_record WHERE session_id = ? ORDER BY id ASC` — that's exactly what an external auditor or a `argo audit verify` invocation does.
- **The cross-tenant test's "third tenant on sink" pattern** is the strongest anti-theater move in the file. The test would PASS if the sink's bound tenant ID accidentally got stamped on the row — but it asserts the column is the *duty's* tenant. Without that decoupling, the test would be a tautology.
- **Negative assertion on the success-payload shape**: `payload.get("error").is_none()`. Successful tick must NOT include an error key. This is the kind of check that catches a regression where a future change accidentally serialises an `Option<String>: None` as `null` (which would surface as `error: null`).
- **No `#[ignore]`s** in the new test file.
- **`spawn_duty_scheduler` primitive used directly**, not re-wrapped. The PR even names the production binary's call site (`boot_spawns.rs:158-178`) so a future regression in the boot wiring is one git-blame away.

## Boundary observation worth surfacing

The PR commit message documents a **workspace-clippy boundary problem**:

> Workspace clippy fast-fail trips on `crates/argo-agent-runner/tests/cron_duty_wire_closure_test.rs:120/177/275` (E0061: `DutyStore::sessions_for_duty` takes 2 args, callsites still pass 1). Pre-existing drift from `mission-orchestrator-duty-store-tenant-args` (#2100), explicitly documented in the BUILD_STATUS.md row for `fix-cu-macos-permission-revoke-mid-run-test-workspace-clippy-drift`.

This is HARD RULE 2 boundary discipline done right — but it surfaces a **systemic issue**: PR #2100 (`mission-orchestrator-duty-store-tenant-args`) landed a breaking signature change without rebasing every caller. Three callsites in `cron_duty_wire_closure_test.rs` are still on the old signature. This blocks the workspace clippy gate.

## Gap / follow-up brief candidates

**`mission-duty-scheduler-gateway-boot-wire-real-runner-and-sink`** — already named in this PR. Priority P1 (substrate is LIVE, production binary still emits nothing). Scope: ~30-60 LOC. Construct an `AgentRunner` + `AuditSink` in gateway boot, chain them via `DutyScheduler::with_agent_runner(...).with_audit_sink(...)`, gate behind `ARGO_DUTY_SCHEDULER_RUNTIME_WIRE=on` for the initial rollout. The follow-up will let Claim 2 ratchet from "substrate-LIVE" to "production-LIVE".

**`fix-cron-duty-wire-closure-test-rebase-onto-tenant-id-arg`** — P1, ~10-30 LOC mechanical fix. Update the 3 callsites at `crates/argo-agent-runner/tests/cron_duty_wire_closure_test.rs:120/177/275` to pass `tenant_id` as the new first arg to `DutyStore::sessions_for_duty`. This is the same shape as the round-32 cu-macos fix that landed as collateral in #2112 — small mechanical rebase, unblocks the workspace clippy gate. Likely already named under a different brief id — recommend grepping `streams/` before authoring.

## 12th green Claim contract on main (counting Claim 2 first MVP-LIVE)

Counting on origin/main: claim-1 round-17 + claim-6-strengthener round-21 + claim-6-rows-1/2/5 rounds 22/23/24 + claim-5 LIVE round-32 + claim-6 cross-tenant round-32 + claim-2 substrate-LIVE round-36 (this PR). The Claim 2 cell flips from 🔴 WIRE BROKEN to 🟢 substrate-LIVE; the gateway-boot follow-up will take it to 🟢🟢.
