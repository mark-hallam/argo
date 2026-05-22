# eval-cron-duty-fires-agent-runner-end-to-end

**Owner**: claude (evaluator-shape brief — investigation + cargo integration test).
**Depends on**: none.
**Authored against**: origin/main `432aac842` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-cron-duty-fires-agent-runner-end-to-end`.

🟡 **P1 MISSION-CLAIM-2 (Delegation is the unit).** Source: round-8 manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` Claim 2 row.

> "argo-cron — Duty scheduler ticks every minute under file-lock. Confirmed wire at gateway boot in #939. ⚠ Gap: does the duty `entry_action` actually invoke the worker's agent runner? Need to trace `Scheduler::tick → JobRunner → AgentRunner`."

The duty scheduler is the load-bearing claim-2 primitive — "be my marketing intern" means recurring duty execution. The substrate (cron crate + scheduler ticks + job runner) exists, but the **end-to-end wire from a tick to an actual agent turn has never been verified**.

## Scope

Write a cargo integration test in `crates/argo-cron/tests/` (or `crates/argo-agent-runner/tests/` if the runner side is the natural test owner) that:

1. Bootstraps `:memory:` SessionStore with a tenant + a worker + a duty.
   - Worker has `cron_spec = "* * * * *"` (every minute), `default_enabled = true`.
   - Duty has an `entry_action` (e.g. `Skill::Lookup("hello-world")` or any in-core skill).
2. Constructs the `Scheduler` and calls `Scheduler::tick(now)` with a `now` that triggers the cron.
3. **Asserts**:
   - `JobRunner::run` was invoked for the worker_id.
   - `AgentRunner::run_turn` was invoked downstream of `JobRunner`.
   - An `audit_record` row `WorkerDutyFired` was written for the (tenant, worker, duty).
   - Each step's `tenant_id` was passed correctly (cross-tenant safety).

4. **Paired negative test**: tick with `now` that does NOT match the cron → assert NO JobRunner call, NO audit row.

5. **Paired cross-tenant test**: bootstrap tenant_b's worker with same cron_spec → tick → assert tenant_a's duty fires for tenant_a's worker_id only (not tenant_b's).

### What the test catches

- Regression: someone refactors `Scheduler::tick` and drops the AgentRunner dispatch.
- Regression: someone changes the duty cron parsing and the tick fires at wrong times.
- Cross-tenant bypass: someone removes tenant_id binding from the duty lookup query.
- Audit-chain hole: someone forgets to emit `WorkerDutyFired`.

If the test reveals the wire is BROKEN (Scheduler ticks but AgentRunner is NOT invoked) → ship `#[ignore]`'d + FINDINGS-ONLY note. Fix-brief follows.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-cron/tests/cron_duty_fires_agent_runner_test.rs` (preferred location), or `crates/argo-agent-runner/tests/cron_duty_e2e_test.rs` if test fixtures already exist there.
- `crates/argo-cron/Cargo.toml` or `crates/argo-agent-runner/Cargo.toml` — narrowly scoped: add dev-deps if missing.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip argo-cron row 🟡 → 🟢 or update note with findings.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-cron/src/**` — read-only audit, NOT a refactor.
- `crates/argo-agent-runner/src/**` — read-only.
- Other crates outside the call chain.

## Acceptance criteria

- [ ] 3 tests (happy / no-match-negative / cross-tenant-isolation) all assertive (not just smoke-runs).
- [ ] Tests pass with `cargo test -p argo-cron --test cron_duty_fires_agent_runner_test`.
- [ ] If pre-flight reveals the wire is broken, ship `#[ignore]`'d + FINDINGS-ONLY.
- [ ] Gap manifest updated.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `eval-cron-duty-fires-agent-runner-end-to-end`
- PR title: `test(cron): duty tick fires agent-runner end-to-end + cross-tenant (eval-cron-duty-fires-agent-runner-end-to-end)`

## Notes for the implementer

- ~150-300 LOC expected. 3 tests at ~50-80 LOC each + fixture setup.
- **The cross-tenant test is non-optional**. Per memory `feedback_enterprise_grade_defaults.md`: every multi-tenant primitive needs a cross-tenant negative test.
- **Don't refactor production code**. If the wire is broken, document + ignore + FINDINGS-ONLY.
- Cite round-8 manifest verbatim in PR body.
