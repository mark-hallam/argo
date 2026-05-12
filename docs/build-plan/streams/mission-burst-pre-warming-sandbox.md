# Mission load-bearing wire — Burst pre-warming for sandbox creation

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 8 #255. E2B sandbox creation has cold-start latency (~10-15s). When a vertical fires multiple sandboxed actions in succession, each cold-start chains. Pre-warm a small pool during low-activity windows so bursts run hot.

## Scope

Extend `argo-executor-computer-use::SandboxPool` with predictive pre-warming:

1. **Activity-driven trigger** — track per-vertical sandbox-spawn rate over a sliding 5-min window. When rate > N (operator-tunable, default 3 spawns/5min), pre-warm 2 extra sandboxes.
2. **Time-of-day model** — pre-warm 1 sandbox at the start of each operator-active hour (from operator's quiet-hours config inverse).
3. **Cap** — max pre-warm pool size = 5 (configurable). Don't keep more.
4. **Idle reaper** — pre-warmed sandboxes idle > 10 min get reaped (sibling brief `mission-cu-sandbox-pool-reaper` handles in-band reaping; this brief calls into it).
5. **Audit row** per pre-warm + per use-of-prewarmed (kind: `runtime.sandbox_prewarmed`, `runtime.sandbox_prewarm_consumed`).

This is NOT in scope: ML-driven prediction (sliding-window heuristic only); per-tenant pre-warm budgets (default global pool in v1); pre-warm of non-E2B sandbox types (E2B only — Wayland/native have different lifecycle).

## Files you may touch

- `crates/argo-executor-computer-use/src/sandbox_prewarm.rs` (NEW, ≤ 350 LOC)
- `crates/argo-executor-computer-use/src/sandbox_prewarm_tests.rs` (NEW, sibling, ≤ 280 LOC) — 6 tests
- `crates/argo-executor-computer-use/src/pool.rs` (≤ 480 LOC after) — narrowly scoped: pre-warm tap-in
- `argo-config/src/sections.rs` — `[runtime.sandbox_prewarm] enabled = false; spawn_rate_threshold = 3; max_prewarm = 5`
- `docs/skycot/RUNBOOK.md` — "Sandbox pre-warming" subsection

## Files you must NOT touch

- E2B API surface; the sandbox-reaper (sibling brief)
- Per-execution sandbox-action logic

## Acceptance criteria

- [ ] Sliding-window rate detection; pre-warm triggers at threshold
- [ ] Time-of-day hourly pre-warm during operator-active hours
- [ ] Cap at max_prewarm; no overflow
- [ ] Pre-warmed sandboxes consumed FIFO by dispatch
- [ ] Audit rows on pre-warm + consume
- [ ] `enabled = false` (default) — no extra E2B spend
- [ ] 6 unit tests covering the rate + cap + hour triggers
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-burst-pre-warming-sandbox`
- PR title: `executor: sandbox pre-warming for burst workloads (mission-burst-pre-warming-sandbox)`

## Notes

- Cold-start matters more for demos and real-time work than for nightly batch. The threshold defaults reflect demo-cadence (3 spawns / 5min); operators with batch workloads may want this off.
- Cost tradeoff: pre-warm consumes E2B-minutes even when unused. Default off respects operator wallet.
- Pair with sandbox-pool-reaper — pre-warm creates, reaper destroys; they balance.
