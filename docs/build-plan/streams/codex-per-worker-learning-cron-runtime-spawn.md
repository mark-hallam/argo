# codex-per-worker-learning-cron-runtime-spawn — Spawn per-worker learning cron from gateway boot (F-04 P1)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-per-worker-learning-cron-runtime-spawn`.
**Source**: F-04 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 4 (skills shared and learned):** Workers learn skills from successful trajectories (`docs/skycot/MISSION.md:36`). Without runtime scheduling, the substrate is testable but workers do not autonomously learn.
- **V1 success scenario #2:** A learned skill propagates to another worker (`docs/skycot/MISSION.md:214`). Requires the per-worker rating → overlay-proposal pipeline to FIRE in production, not just in tests.

## Scope

P1 wiring. The substrate is shipped:
- `per_worker_learning_tick` (`crates/argo-skill/src/per_worker_learning_cron.rs:62`)
- `spawn_per_worker_learning_cron` (`crates/argo-skill/src/per_worker_learning_cron.rs:119`)
- Re-exported from `argo-skill` (`crates/argo-skill/src/lib.rs:94`)

But no production caller invokes it. The only non-test reference is the BUILD_STATUS.md ledger note. Workers never learn in production unless an external operator manually fires the tick.

This is exactly the "land scaffold now, wire later" failure mode per `DRIFT_PREVENTION.md:45`.

Scope:
- Spawn `per_worker_learning_cron` from `crates/argo-cli/src/handlers/gateway/boot.rs` lifecycle setup (gateway is the natural owner of long-running tenant-scoped tasks).
- Pass tenant context per workspace + skill store handle.
- Retain a shutdown handle alongside other gateway tasks (graceful shutdown path).
- Add config knobs for `[skill_learning]` block in `argo-config` (interval, enabled flag).
- Add integration test: write `skill_rating` rows into the store, advance the cron tick, assert an overlay proposal lands without directly calling `per_worker_learning_tick`.

LOC: ~250.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/boot.rs` — spawn cron alongside existing long-running tasks; retain shutdown handle. ≤80 modified lines.
- `crates/argo-skill/src/per_worker_learning_cron.rs` — narrowly: ensure `spawn_per_worker_learning_cron` signature accepts the production-required context (tenant_id, store, shutdown rx). ≤40 modified lines.
- `crates/argo-config/src/sections.rs` — add `[skill_learning]` block (`enabled`, `interval_secs`, `tenant_filter`). ≤30 modified lines.
- `crates/argo-config/src/lib.rs` — narrowly: register the new section. ≤15 modified lines.
- `crates/argo-skill/tests/per_worker_learning_cron_runtime.rs` (NEW) — integration test exercising the boot-spawn path. ≤120 LOC.

## Files you must NOT touch

- `crates/argo-skill/src/per_worker_learning_cron.rs` core logic — that's already shipped and tested.
- `crates/argo-agent-runner/**` — this is gateway/boot scope, not turn-loop scope.

## Acceptance criteria

- [ ] `boot.rs` spawns `per_worker_learning_cron` on gateway startup.
- [ ] Shutdown handle is retained and called during graceful shutdown.
- [ ] `[skill_learning]` config block parses correctly; default `enabled = true` with `interval_secs = 3600`.
- [ ] Integration test proves: writing rating rows → advancing cron → overlay proposed in store, without calling the tick function directly.
- [ ] `grep "per_worker_learning_tick\|spawn_per_worker_learning_cron" crates --include="*.rs" -l | grep -v '_tests.rs\|/tests/'` returns at least the new boot.rs caller (not just exports).
- [ ] `cargo test -p argo-skill -p argo-cli per_worker_learning` passes.
- [ ] `cargo clippy -p argo-skill -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-per-worker-learning-cron-runtime-spawn`
- PR title: `feat(gateway): spawn per-worker skill learning cron from boot (codex-per-worker-learning-cron-runtime-spawn) — F-04 P1`

## Notes for the implementer

- Use `tokio::spawn` with a `tokio::sync::oneshot` (or `watch`) for shutdown propagation. Match the pattern used by other gateway-spawned long-running tasks (search `boot.rs` for `tokio::spawn` examples).
- The cron should iterate all configured tenants/workspaces, not just the first. Use `WorkspaceStore::list_all_tenants` or equivalent.
- `[skill_learning]` config matters because operators may want to disable autonomous learning in certain environments (e.g., regulated tenants). Make `enabled = false` skip cleanly.
- After this brief, V1 success scenario #2 (Maya learns → Alex inherits) actually runs in production, not just tests.
