# mission-duty-scheduler-gateway-boot-wire-real-runner-and-sink

- **Priority**: 🟡 P1 (Claim 2 production-binary-LIVE gap; substrate-LIVE shipped via #2137 but the gateway boot path still constructs a bare DutyScheduler that never invokes a real AgentRunner)
- **Owner**: claude
- **Type**: mission (production wire-up; gateway boot path; ~40-80 LOC)
**Authored against**: origin/main `ccdff2958` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-duty-scheduler-gateway-boot-wire-real-runner-and-sink`.

## Premise

Mission claim 2 ("Delegation is the unit") requires that a duty cron at hire-time really runs a worker turn when it fires. PR #2137 (`eval-claim-2-live-duty-cron-fires-runner-test`, MERGED 2026-05-23) shipped the **substrate-LIVE** contract: 3 tests pin `DutyScheduler::with_agent_runner` + `with_audit_sink` + `tick` so that the closure-loop wire is regression-protected.

BUT — `crates/argo-cli/src/handlers/gateway/boot_spawns.rs:169` still constructs a **bare** scheduler:

```rust
let scheduler = Arc::new(DutyScheduler::new(duties, orchestrator, gate));
//                                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//                                          no .with_agent_runner(...)
//                                          no .with_audit_sink(...)
```

`DutyScheduler::with_agent_runner` + `with_audit_sink` are the optional builders at `crates/argo-orchestrator/src/duty_scheduler.rs:237` + `:249` that hand the scheduler the real wires. Without them, the production gateway-boot spawns a scheduler that **ticks every minute, refuses every duty via `ClosedGate`, and emits no audit row** — claim 2 is provably LIVE in the test suite but provably NOT LIVE in the running gateway.

Round-36 evaluator (2026-05-23T13:27Z) called this out explicitly: PR #2137's module doc names this follow-up, but no one has claimed it.

## What to do

Update `boot_spawns.rs::spawn_duty_scheduler_task` (the function around line 169) to construct a real `DutyAgentRunner` + `AuditSink` and feed them to the scheduler via the existing builder methods:

```rust
let scheduler = Arc::new(
    DutyScheduler::new(duties, orchestrator, gate)
        .with_agent_runner(real_runner)
        .with_audit_sink(real_sink),
);
```

Specifics to verify against current main when claiming:

1. **`DutyAgentRunner` trait.** It's at `crates/argo-orchestrator/src/duty_scheduler.rs` (grep for `trait DutyAgentRunner`). The gateway-boot needs a concrete implementor — likely the same one the gateway already constructs for HTTP `POST /verticals/.../workflows/.../execute` (`routes_workflow_execute.rs`). Reuse, don't duplicate.
2. **`AuditSink`.** Construct via `AuditSink::new_with_tenant(store, tenant_id)` matching the per-tenant pattern used elsewhere in `boot_spawns.rs`. The duty being fired carries its own tenant_id, so the sink either needs to be tenant-keyed at fire-time or constructed per-tenant when the duty enters the gate.
3. **`ClosedGate`.** Today it denies every fire. After this brief, the gate is the still-correct V1 fail-safe — but the *path past* the gate now does real work. The gate flip is operator-decisional and outside this brief's scope. Document the new wire as "ready for the gate to open" in the boot-message string.

## May-touch

- `crates/argo-cli/src/handlers/gateway/boot_spawns.rs` — the boot wire.
- `crates/argo-cli/src/handlers/gateway/boot.rs` (or wherever the AgentRunner is already constructed for HTTP execute) — extract a shared constructor if duplicated.
- 1-2 new tests under `crates/argo-cli/tests/` or `crates/argo-orchestrator/tests/` pinning that:
  - `spawn_duty_scheduler_task` constructs a scheduler whose `agent_runner` is `Some` (use a public accessor or, if none exists, a smaller direct test that constructs the scheduler with builder methods + asserts the result).
  - When the gate flips to allowing, a fired duty really invokes the runner + writes a `WorkerDutyFired`-shaped audit row. Use the existing PR #2137 test pattern as the reference; the new test pins the **gateway-boot construction path**, not the scheduler in isolation.

## Hands-off

- `crates/argo-orchestrator/src/duty_scheduler.rs` — production substrate is already correct (#2137 verified). Do NOT add new builder methods; thread the existing ones.
- The `ClosedGate` → `AutoApprovalGate` flip — operator-decisional, separate brief.
- Any cron-frequency tuning — out of scope.

## Verification

- `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- `cargo test -p argo-cli --tests` green (new test + existing tests).
- Boot the gateway locally, observe the `eprintln!` boot-message string reflects the wired runner + sink (e.g. `gate=closed, runner=wired, sink=wired`).
- `cargo test -p argo-orchestrator --test claim_2_live_duty_cron_fires_runner_test` still green (regression guard — production wire-up must not break the substrate contract).

## Acceptance

- `boot_spawns.rs:169` constructs the scheduler with both builder methods called.
- A new test pins the boot-construction contract (runner + sink are `Some` after `spawn_duty_scheduler_task`).
- The PR #2137 regression suite still passes.
- Boot-message string reflects the new wire so operators see at startup that the path is ready.
- Claim 2 promoted from substrate-LIVE → **production-binary-LIVE**.

## Provenance

Round-36 evaluator finding (2026-05-23T13:27Z). PR #2137's substrate-LIVE pins explicitly name this follow-up in their module doc. PR #2137 is the regression contract; this brief is the production wire.

Related shipped:
- [[eval-claim-2-live-duty-cron-fires-runner-test]] (#2137) — substrate-LIVE pins (the seam this brief makes load-bearing).
- [[mission-orchestrator-duty-store-tenant-args]] (#2100) — tenant-id threading on DutyStore (load-bearing for the sink construction here).
