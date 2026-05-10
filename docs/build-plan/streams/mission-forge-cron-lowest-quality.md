# Mission load-bearing wire — Forge cron tick that picks lowest-quality skills

**Owner**: claude. **Depends on**: `argo-forge::run::ForgeRun` SHIPPED (verified at `crates/argo-forge/src/run.rs:105`); `forge::finalize_run` SHIPPED (`forge.rs:65`); `argo-skill::quality::QualityScore` SHIPPED; `DutyScheduler` SHIPPED.
**Authored against**: origin/main `c8c959a6` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-cron-lowest-quality`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #83 ("Build Forge as cron tick that picks lowest-quality skills"). Forge today runs only when an operator invokes `argo forge optimize <skill>` — it never proactively improves the lowest-quality skills. Mission Claim 4 ("skills shared and learned") promises continuous learning; without a cron tick, Forge is opt-in only.

## Scope

Add `spawn_forge_cron(deps, cfg)` that:

1. Runs on a configurable interval (default 1 hour, `ARGO_FORGE_CRON_INTERVAL_SECS`).
2. Each tick: queries `skill_quality_score` for the N lowest-scoring skills with `score < threshold` (default N=3, threshold=70.0). Configurable via `ForgeCronConfig { max_skills_per_tick: usize, score_threshold: f64, interval_secs: u64, dry_run: bool }`.
3. For each candidate: enqueues a `ForgeRun` (via existing `argo-forge::run` machinery) with budget cap from config (default $0.50/run). Existing `forge_optimize` + safety judge gating preserved.
4. Skips a candidate if it has an in-flight `ForgeRun` for the same skill_id (race-condition guard).
5. Logs `tracing::info!` per dispatched run; emits a `tracing::warn!` if zero candidates met threshold (catches misconfiguration where no skill ever qualifies).
6. `dry_run: true` mode logs candidates but does NOT spawn ForgeRuns — useful for operator dry-runs before enabling cron in prod.

Wired from gateway boot alongside `_sweeper` / `_alerts_loop` / `_audit_watchdog` (new this session) — same handle pattern.

This is NOT in scope: changing Forge's per-run logic; per-tenant cron cadence; cron pause/resume CLI (separate `argo forge cron pause` brief if needed); automatic skill rollback on Forge regression (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-forge/src/cron.rs` (NEW, ≤ 250 LOC) — `pub fn spawn_forge_cron(deps, cfg) -> JoinHandle<()>`, `ForgeCronConfig` struct + `Default`, inner `tick(now, deps, cfg)` async fn factored for testability.
- `crates/argo-forge/src/cron_tests.rs` (NEW, sibling) — 6 unit tests: tick-picks-N-lowest; tick-skips-above-threshold; tick-skips-in-flight; tick-zero-candidates-warns; dry-run-spawns-nothing; interval-knob-respected.
- `crates/argo-forge/src/lib.rs` — narrowly scoped: ONE `pub mod cron;` line + re-export `spawn_forge_cron` and `ForgeCronConfig`.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ONE block adding `let _forge_cron = spawn_forge_cron(deps.clone(), cron_cfg);` next to existing `_audit_watchdog` / `_sweeper` / `_alerts_loop` handles. Env-var driven config.
- `crates/argo-cli/tests/forge_cron_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: seed 5 skills with scores 60/65/75/80/85; run one tick with threshold=70 N=2; assert exactly 2 ForgeRuns enqueued (60 + 65) and none for the others.

## Files you must NOT touch

- `crates/argo-forge/src/forge.rs` + `run.rs` + `persist.rs` — Forge per-run machinery is upstream; reuse only
- `crates/argo-skill/src/quality.rs` — QualityScore primitive is upstream; reuse only
- `crates/argo-orchestrator/src/duty_scheduler.rs` — Forge cron is its OWN spawn loop (not a DutyScheduler entry) — keeps the duty surface focused on tenant-owned duties
- Any non-forge/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (reuses existing skill_quality_score + forge_run tables)

## Acceptance criteria

- [ ] `spawn_forge_cron(deps, cfg) -> JoinHandle<()>` exists
- [ ] `ForgeCronConfig` with `max_skills_per_tick` (default 3), `score_threshold` (default 70.0), `interval_secs` (default 3600), `dry_run` (default false)
- [ ] Env-driven config: `ARGO_FORGE_CRON_INTERVAL_SECS`, `ARGO_FORGE_CRON_MAX_SKILLS`, `ARGO_FORGE_CRON_THRESHOLD`, `ARGO_FORGE_CRON_DRY_RUN`
- [ ] Tick query orders by `score ASC`, filters `score < threshold`, LIMIT max_skills_per_tick
- [ ] Skips candidates with in-flight ForgeRun for same skill_id (idempotent across overlapping ticks)
- [ ] `tracing::info!` per dispatched run with skill_id + score
- [ ] `tracing::warn!` when zero candidates qualified (helps operators tune threshold)
- [ ] `dry_run: true` logs but does NOT enqueue
- [ ] Wired from gateway boot alongside existing handles
- [ ] 6 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-forge -p argo-cli` clean
- [ ] `cargo clippy -p argo-forge -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-forge -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-forge-cron-lowest-quality`
- PR title: `forge: cron tick that picks lowest-quality skills for auto-optimization (mission-forge-cron-lowest-quality)`

## Notes for the implementer

- The 3-skills-per-tick / 70.0 threshold defaults are conservative. A new operator deploying this won't suddenly fire 100 Forge runs at boot — they'll fire at most 3 per hour, only on skills genuinely below threshold.
- The in-flight guard is load-bearing: without it, two overlapping cron ticks (e.g. one hung for 65 minutes when interval is 60 min) would double-spawn ForgeRuns and burn budget.
- Don't use DutyScheduler here — DutyScheduler is for tenant-declared duties (`argo duty list` shows them). Forge cron is an infra background task, more like the audit watchdog. Spawn it the same way.
- The `dry_run` mode is critical for operator confidence — they can enable it for a day, watch the logs, see which skills would have been forged, and tune thresholds before flipping to live.
