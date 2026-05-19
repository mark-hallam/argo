# Stream Audit-P2-2 — dynamic skill-learning enrollment on worker hire

**Owner**: claude.
**Depends on**: PM teammate brief `mission-pm-skill-overlay-load` SHOULD merge first (the overlay-load fix is the substrate this brief activates). Not a hard block — these can ship in either order — but the value of this brief is multiplied once overlay-load is live.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-skill-learning-dynamic-enroll`.
**Source**: 2026-05-19 codex audit P2-4, validated by claude `VALIDATION.md` (CONFIRMED — `spawn_per_worker_learning_cron` only called from `boot.rs:218` and tests; no enrollment from worker-hire path).

## Scope

`crates/argo-cli/src/handlers/gateway/boot.rs:190-222` enumerates `worker WHERE archived_at IS NULL` at gateway boot and spawns a learning cron per worker. Workers hired AFTER boot do NOT get a cron — they only start participating in the learning loop on the next gateway restart.

For production deployments where workers are created throughout the day, this means **mission claim 4 (skills shared and learned) is broken for new workers** until restart. Combined with the PM teammate brief's overlay-load fix (closes the read side), this brief closes the write/enrollment side.

Approach: hook the worker-creation path to invoke `spawn_per_worker_learning_cron` for the freshly-created worker. Place the hook in `argo-identity::store::create_worker` (the one creation entry point), guarded by config so the dev-mode "no learning cron" path remains opt-out.

A test verifies: start gateway → hire worker via API → wait one cron tick → assert that a skill candidate produced by this new worker's session is processed by the cron (proving the cron is running for this worker, not just the boot-time set).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-identity/src/store.rs` — extend `create_worker` (and `create_worker_from_draft`) signatures to take an `&AgentDeps` (or a narrowly-scoped `LearningEnrollHandle`). On successful insert + commit, call `enroll_for_learning(worker_id, tenant_id)`. If `AgentDeps` is heavy to pass here, factor a `LearningEnrollHandle` from `AgentDeps.learning` that's the minimum surface.
- `crates/argo-skill/src/per_worker_learning_cron.rs` — add `pub fn enroll_worker(deps: &LearningDeps, worker_id, tenant_id)` that spawns a `spawn_per_worker_learning_cron` handle for one worker. Idempotent: if a handle already exists for this `worker_id`, return early (no double-spawn).
- `crates/argo-skill/src/per_worker_learning_registry.rs` (NEW) — `pub struct WorkerLearningRegistry { handles: DashMap<WorkerId, JoinHandle<()>> }`. Stored on `AgentDeps`. On enroll, insert; on tenant/worker archive (out of scope here, but plan for it), remove.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — boot now constructs `WorkerLearningRegistry::new()`, threads into `AgentDeps`, and the existing loop at line 190-222 calls `registry.enroll_worker(...)` instead of free-form `spawn_per_worker_learning_cron`. Net effect: same boot behavior + dynamic post-boot path.
- `crates/argo-cli/tests/post_boot_worker_learning_smoke.rs` (NEW) — three test cases: (1) hire worker via API, wait one cron tick, assert handle exists in registry; (2) duplicate hire (same worker_id, idempotent enroll) returns early; (3) when `learning.enabled = false` in config, no enrollment fires (back-compat).

## Files you must NOT touch

- The cron's tick logic — that's correct.
- The `worker_skill` table — separate brief (PM-track `mission-pm-skill-overlay-load`) handles the read side.
- Worker archival code — assume archive is rare and runs via a maintenance path; registry cleanup on archive is a separate brief (`Out of scope` below).
- The `argo-genesis` worker-generation path — Genesis calls `create_worker` already; once that's hooked, Genesis-generated workers get the learning cron for free.

## Acceptance criteria

- [ ] `create_worker` enrolls the new worker in the learning registry as part of the same call.
- [ ] Idempotency: a duplicate-id enroll attempt is a no-op (no panic, no double-spawn).
- [ ] Existing boot-time enrollment (`boot.rs:190-222`) is routed through the same `registry.enroll_worker` path — single source of truth for "this worker has a learning handle."
- [ ] `learning.enabled = false` in config → no enrollment at boot OR on hire (full opt-out).
- [ ] Three new tests in `post_boot_worker_learning_smoke.rs` pass.
- [ ] No regression in existing boot tests.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim in PR body.
- [ ] `cargo test -p argo-identity -p argo-skill -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-audit-skill-learning-dynamic-enroll`
- PR title: `skill: enroll workers in learning cron at hire, not just boot (mission-audit-skill-learning-dynamic-enroll)`

## Notes for the implementer

- The `WorkerLearningRegistry` lives in `argo-skill` (not `argo-identity`) — identity creates the worker, the skill crate owns the learning surface. Keep responsibilities clean.
- `JoinHandle<()>` on the registry: when a worker is archived (out of scope), the handle should be aborted. For now: document the leak as known + add a TODO comment with a follow-up brief reference.
- If `create_worker` is called inside a test that doesn't have a full `AgentDeps`, the registry should be optional (`Option<WorkerLearningRegistry>`). Document the test-friendly pattern.
- The cron tick interval is config-driven (`learning.interval_secs`); the test should set this to a small value (e.g. 1s) so the assertion doesn't time out.

## Out of scope (follow-up briefs)

- Worker-archive cleanup of learning handles (revisit when archive lifecycle gets first-class treatment).
- Pause/resume learning per worker (operator surface in worker profile) — Track L Phase 2.
- Tenant-level "pause all learning" big-red-button — separate brief.
- Re-enrollment on cron crash (currently a crashed cron stays dead until next boot) — fault-tolerance, separate brief.
