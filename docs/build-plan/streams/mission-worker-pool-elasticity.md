# Mission load-bearing wire — worker pool elasticity (auto-scale on queue depth)

**Owner**: claude. **Depends on**: existing argo-runtime worker pool / supervisor (`crates/argo-runtime/src/worker.rs` + sibling — verify at claim time); existing metrics surface (`crates/argo-cli/src/handlers/gateway/metrics_json.rs`).
**Authored against**: origin/main `20240927` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-worker-pool-elasticity`.

> SOURCE: MISSION_BACKLOG.md Section 8 Claim 8 #242 ("Worker pool elasticity (auto-scale on queue depth)"). **Tier P (scale).** Today: the operator manually sets `worker_count` in `argo.toml`; if the inbound brief queue spikes, workers saturate and queue depth grows unbounded. The operator only finds out when SLAs slip. This brief adds a passive monitor that observes queue depth + per-worker utilisation and emits scale-up / scale-down recommendations (and, when explicitly enabled, acts on them by spawning / draining workers within configured bounds).

## Scope

NEW `crates/argo-runtime/src/pool_elasticity.rs` (NEW, ≤ 260 LOC):

- `pub struct ElasticityConfig { min_workers: u32, max_workers: u32, target_queue_depth_per_worker: u32, scale_check_interval: Duration, scale_up_cooldown: Duration, scale_down_cooldown: Duration, mode: ElasticityMode }`.
- `pub enum ElasticityMode { Observe, RecommendOnly, AutoScale }` — default `RecommendOnly`. `AutoScale` is opt-in.
- `pub struct PoolMetricsSnapshot { ts: DateTime<Utc>, active_workers: u32, queue_depth: u32, per_worker_utilization: Vec<f32> }` — captured each tick.
- `pub enum ScaleRecommendation { Hold, ScaleUp { target: u32, reason: String }, ScaleDown { target: u32, reason: String } }`.
- `pub fn recommend(snapshot: &PoolMetricsSnapshot, cfg: &ElasticityConfig, last_scale_at: Option<DateTime<Utc>>) -> ScaleRecommendation` — pure function:
  - If `queue_depth > active_workers * target_queue_depth_per_worker * 1.5` AND not in scale-up cooldown → recommend scale up (one worker at a time; clamped at `max_workers`).
  - If `queue_depth < active_workers * target_queue_depth_per_worker * 0.3` AND average `per_worker_utilization < 0.25` AND not in scale-down cooldown → recommend scale down (one worker at a time; clamped at `min_workers`).
  - Else `Hold`.
- `pub async fn apply_recommendation(deps: &AgentDeps, rec: &ScaleRecommendation, cfg: &ElasticityConfig) -> ArgoResult<()>` — when `mode == AutoScale`, spawns or drains a worker. Drain is graceful: marks the worker `DrainRequested`, waits for in-flight brief to land, then terminates. Spawn invokes the existing worker-launch path.
- `pub async fn emit_audit_row(deps: &AgentDeps, snapshot: &PoolMetricsSnapshot, rec: &ScaleRecommendation) -> ArgoResult<()>` — appends `kind = "pool_elasticity_tick"` row with snapshot + recommendation + applied? for observability.

NEW `crates/argo-runtime/src/pool_elasticity_tests.rs` (NEW, ≤ 180 LOC, included via `#[path]`) — 7 unit tests: recommend-hold-at-target-depth, recommend-scale-up-on-150pct-over-target, recommend-scale-down-on-30pct-under-and-low-util, scale-up-blocked-during-cooldown, scale-down-blocked-when-already-at-min, scale-up-blocked-when-already-at-max, recommendation-shape-matches-spec.

### Metrics

EXTEND `crates/argo-cli/src/handlers/gateway/metrics_json.rs` (≤ 25 LOC delta — verify file at claim time). Add metrics gauges:
- `argo_worker_pool_active_total` — current worker count.
- `argo_worker_pool_queue_depth` — current TODO/CLAIMED count.
- `argo_worker_pool_target` — last `ScaleRecommendation.target` (matches active when `Hold`).
- `argo_worker_pool_scale_events_total{direction="up"|"down"}` — counter.

### Cron integration

Coordinate w/ in-flight `mission-cron-tick-maintenance-jobs`. Register `pool_elasticity_tick` system job (default interval 60s — short cadence since recommendations are cooldown-gated).

If the cron-tick brief hasn't landed: NEW standalone `crates/argo-runtime/src/pool_elasticity_cron.rs` (≤ 60 LOC). Folds into cron scaffolding later.

### CLI

EXTEND `crates/argo-cli/src/cli_worker_cmds.rs` (≤ 20 LOC delta) — new `WorkerCmd::ElasticityStatus`:
```
argo worker elasticity-status          # current pool state + last recommendation
argo worker elasticity-config <args>   # show / set elasticity config (mode, bounds, etc.)
```

NEW `crates/argo-cli/src/handlers/worker_elasticity.rs` (NEW, ≤ 130 LOC) — reads latest pool snapshot + recommendation, emits JSON; subcommand for config view/edit.

### Config

EXTEND `argo.toml` parsing (verify path at claim time — likely `crates/argo-config/src/lib.rs`). New `[worker_pool.elasticity]` section:
```toml
[worker_pool.elasticity]
mode = "RecommendOnly"           # Observe | RecommendOnly | AutoScale
min_workers = 1
max_workers = 8
target_queue_depth_per_worker = 3
scale_check_interval_secs = 60
scale_up_cooldown_secs = 300
scale_down_cooldown_secs = 600
```

NO migration.

This is NOT in scope: cross-host worker pool scaling (single-host only in v1; multi-host orchestrator is #202); cost-aware scaling (LLM token cost as a scale signal — v2); per-brief-tier scaling (Tier M briefs scale differently than Tier P — v2); ECS / Kubernetes / cloud-native autoscaler integration (operator writes their own controller against the metric surface); auto-tuning `target_queue_depth_per_worker` from historical data (v2 if operators ask); preemptive scale-up before known traffic spikes (calendar-aware scaling is v2).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/src/pool_elasticity.rs` (NEW, ≤ 260 LOC)
- `crates/argo-runtime/src/pool_elasticity_tests.rs` (NEW, ≤ 180 LOC)
- `crates/argo-runtime/src/lib.rs` (EXTEND, ≤ 5 LOC — re-export)
- `crates/argo-runtime/src/pool_elasticity_cron.rs` (NEW, ≤ 60 LOC) — standalone OR fold into cron-tick scaffolding
- `crates/argo-cli/src/cli_worker_cmds.rs` (EXTEND, ≤ 20 LOC)
- `crates/argo-cli/src/handlers/worker_elasticity.rs` (NEW, ≤ 130 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-cli/src/handlers/gateway/metrics_json.rs` (EXTEND, ≤ 25 LOC delta)
- `crates/argo-cli/tests/worker_elasticity_smoke.rs` (NEW, ≤ 140 LOC — RecommendOnly mode in-process test)
- `crates/argo-config/src/lib.rs` (EXTEND, ≤ 20 LOC — `[worker_pool.elasticity]` parser; verify at claim time)
- `docs/skycot/runbook/worker-pool-elasticity.md` (NEW, ≤ 180 LOC) — operator runbook

## Files you must NOT touch

- The actual worker-launch path beyond invoking the existing spawn API (don't rewrite worker bootstrap)
- `crates/argo-store/migrations/**` (no migration)
- Audit-chain hash logic (immutable)

## Acceptance criteria

- [ ] `cargo build -p argo-runtime -p argo-cli` succeeds
- [ ] `cargo test -p argo-runtime pool_elasticity` passes all 7 unit tests
- [ ] `cargo test -p argo-cli --test worker_elasticity_smoke` passes (RecommendOnly mode validated end-to-end)
- [ ] Default config is `mode = "RecommendOnly"` — no scaling actions unless operator opts into `AutoScale`
- [ ] Scale-up triggers at 1.5× target depth; scale-down at 0.3× target + utilization < 25%
- [ ] Cooldowns: scale-up 5 min, scale-down 10 min (scale-down is more conservative per industry pattern)
- [ ] `argo worker elasticity-status` returns current snapshot + last recommendation as JSON
- [ ] Prometheus metrics gauges exposed on `/metrics` (active, queue_depth, target, scale_events)
- [ ] Drain path is graceful — no in-flight brief is lost on scale-down
- [ ] Audit row appended per elasticity tick (whether action taken or not, for observability)
- [ ] No migration; config-only addition
- [ ] Runbook documents: when to flip from RecommendOnly to AutoScale; how to tune target_queue_depth_per_worker for a workload
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-worker-pool-elasticity`
- PR title: `argo-runtime: worker-pool elasticity recommender (Observe / RecommendOnly / AutoScale) + metrics + cron tick (mission-worker-pool-elasticity)`

## Notes for the implementer

- The recommender is a pure function — make it easy to test offline w/ synthetic snapshots. The `apply_recommendation` is the impure half.
- Default mode is `RecommendOnly`. Operators see "I would scale up here" in the audit row + metric without anything actually happening. This is the safe-by-default posture; AutoScale is opt-in only after the operator validates the recommender on their workload.
- DO NOT scale by more than 1 worker per tick. Multi-step scaling is unstable; cooldowns let the system settle between adjustments.
- The drain path on scale-down is load-bearing: a worker marked `DrainRequested` must finish its in-flight brief (or release the claim cleanly if the brief is in a checkpoint-safe state). Don't SIGKILL workers. Use the existing graceful-shutdown path (signal trap from `mission-health-check-endpoint` shipped earlier).
- Per-worker utilization is the rolling % of last `scale_check_interval` the worker was non-idle (in `claim-with-retry` or `<brief>-execution` mode). Below 25% across all workers means the pool is over-provisioned.
- DO NOT scale below `min_workers = 1`. A pool of 0 workers can't accept inbound briefs. Document this floor in the runbook.
- DO NOT scale above `max_workers` — clamp hard. Operators tune `max_workers` against their host CPU / memory / per-worker concurrency budget. Auto-tuning the cap is out of scope.
- The cooldown asymmetry (5 min scale-up, 10 min scale-down) follows standard autoscaler patterns: react fast to load, react slow to drops to avoid flapping.
- Coordinate w/ `mission-oom-restart-handler` parallel brief: that brief's "give up after N restarts" path RELEASES the brief back to TODO, which increases queue depth, which could trigger a scale-up here. That feedback loop is intended: chronic-OOM workers get replaced AND the pool grows to absorb the backlog. Audit-chain visibility makes the chain of events traceable.
- Coordinate w/ `mission-e2b-reap-idle` parallel brief: that brief reaps E2B sandboxes; this brief reaps workers. Different layers; no overlap.
- DO NOT introduce auto-scaling based on cost (LLM token spend). v2 brief. Queue depth + utilization is sufficient for v1.
- The runbook should walk through: starting in Observe → flip to RecommendOnly → review week of audit rows → flip to AutoScale.
