# fix-duty-scheduler-invokes-agent-runner

**Owner**: claude.
**Depends on**: `eval-cron-duty-fires-agent-runner-end-to-end` (#2085 MERGED with FINDINGS-ONLY ignores).
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-duty-scheduler-invokes-agent-runner`.

🔴 **P0 MISSION-CLAIM-2 BROKEN WIRE — duty scheduler doesn't actually drive workers.** Source: PR #2085 review pre-flight grep.

> "Gateway boots `argo_orchestrator::DutyScheduler` (not `argo_cron::Scheduler`), and `DutyScheduler::tick` creates a Session row but **never invokes `AgentRunner::run_turn`** and writes **zero audit rows**. `AuditKind::WorkerDutyFired` doesn't exist."

Mission claim 2 ("delegation is the unit") demands recurring duty execution. The substrate exists (cron crate + scheduler), the gateway boots a scheduler (orchestrator's `DutyScheduler`), the scheduler ticks correctly — **BUT the tick never reaches AgentRunner**. Every duty cron is a no-op. Demo claim 2 is fake.

## Scope

Wire `DutyScheduler::tick` to actually invoke `AgentRunner::run_turn` + emit `WorkerDutyFired` audit row.

### Implementation

1. **Add `AuditKind::WorkerDutyFired` variant** in `crates/argo-audit/src/event_kind.rs` (canonical AuditKind enum). Payload shape: `{ tenant_id, worker_id, duty_id, fired_at, cron_spec }`.

2. **Modify `argo-orchestrator::DutyScheduler::tick`**:
   - For each matched duty: load worker by (tenant_id, worker_id).
   - Construct `Turn::Duty { duty_id, cron_spec, ... }` invocation context.
   - Invoke `AgentRunner::run_turn(ctx)`.
   - On success: emit `WorkerDutyFired` audit row INSIDE the same transaction as the session creation.
   - On failure: emit `WorkerDutyFailed` (NEW variant) audit row + log + don't crash the scheduler.

3. **Drop `#[ignore]`** on #2085's 2 contract tests + verify they pass.

4. **Cross-tenant negative**: tenant_a's duty fires for tenant_a's worker only — pinned in #2085's existing test.

### Failure-injection test

Mock the AgentRunner to return Err → assert WorkerDutyFailed audit row written + scheduler keeps ticking.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-audit/src/event_kind.rs` — narrowly scoped: 2 new variants.
- `crates/argo-orchestrator/src/duty_scheduler.rs` (or wherever DutyScheduler lives — find first).
- `crates/argo-orchestrator/src/duty_scheduler_tests.rs` — extend with failure-injection.
- `crates/argo-agent-runner/tests/cron_duty_e2e_test.rs` — un-ignore the 2 FINDINGS-ONLY tests.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `argo-cron` — that's the cron-parsing primitive; not what's broken. DutyScheduler USES it correctly.
- `AgentRunner::run_turn` itself — already exists, just needs to be called from DutyScheduler.
- The duty cron table schema — no migration needed.

## Acceptance criteria

- [ ] `AuditKind::WorkerDutyFired` + `WorkerDutyFailed` variants added.
- [ ] `DutyScheduler::tick` invokes `AgentRunner::run_turn` for each matched duty.
- [ ] Audit row emission INSIDE the scheduler's transaction (atomic — both land or neither).
- [ ] #2085's 2 `#[ignore]`'d tests pass after the fix.
- [ ] Failure-injection test pins atomic rollback.
- [ ] Cross-tenant negative still passes.
- [ ] `cargo clippy -p argo-orchestrator -p argo-audit -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `fix-duty-scheduler-invokes-agent-runner`
- PR title: `fix(orchestrator): DutyScheduler invokes AgentRunner + emits audit (fix-duty-scheduler-invokes-agent-runner)`

## Notes for the implementer

- ~200-400 LOC expected. Variant adds ~30, scheduler wire ~80, tests un-ignored ~30, failure-injection ~80, atomic-tx ~50.
- **The atomic-tx contract is load-bearing** — match the pattern from #2078 (mission-workflow-audit-write-transactional). If audit fails, the duty session shouldn't appear to have fired either.
- **`WorkerDutyFailed` is just as important as `WorkerDutyFired`** — silent failures are worse than visible ones.
- After this lands, the conductor flips manifest row argo-cron 🔴 → 🟢 (wire fixed).
- Cite #2085 review pre-flight finding + manifest's WIRE BROKEN P0 row verbatim.
