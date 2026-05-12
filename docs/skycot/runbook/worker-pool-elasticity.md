# Worker-pool elasticity

The worker pool can auto-scale on observed queue depth + per-worker
utilisation. Auto-scaling is **off by default**: operators promote
through three modes — `Observe` → `RecommendOnly` → `AutoScale` —
once they've watched the recommender on their real workload for at
least a week.

The recommender is a pure function (`argo_runtime::recommend`) over
a `PoolMetricsSnapshot` plus the operator-tunable `ElasticityConfig`.
A cron tick (default 60s) snapshots the pool, asks the recommender,
optionally applies the recommendation, and records the result for
metrics + the `argo worker elasticity status` CLI.

## Configuration

`argo.toml`:

```toml
[worker_pool.elasticity]
mode = "RecommendOnly"           # Observe | RecommendOnly | AutoScale
min_workers = 1
max_workers = 8
target_queue_depth_per_worker = 3
scale_check_interval_secs = 60
scale_up_cooldown_secs = 300     # 5 min
scale_down_cooldown_secs = 600   # 10 min
```

Show the resolved config:

```sh
argo worker elasticity config        # JSON
argo worker elasticity config --text # human-readable
```

## The three modes

### `Observe`

Captures snapshots + audit rows; never emits a `ScaleUp` / `ScaleDown`
recommendation. Use this when you've just turned on the recommender
and want a week of baseline queue-depth + utilisation data before
trusting the recommendation logic.

### `RecommendOnly` (default)

The recommender produces real `ScaleUp` / `ScaleDown` recommendations
and surfaces them in `argo worker elasticity status` + the
`argo_worker_pool_target` metric, but the pool is **never** mutated.
Use this for at least one full week before flipping to `AutoScale` —
spot-check whether the recommendations agree with what you'd have
done by hand.

### `AutoScale`

The recommender's output is applied: `ScaleUp` spawns one worker
(through the existing worker-launch path) and `ScaleDown` marks one
worker `DrainRequested`. Drain is graceful — the worker finishes any
in-flight brief before terminating; no SIGKILL.

## Scale thresholds

- **Scale up** when `queue_depth > active_workers × target × 1.5`
  AND scale-up cooldown is clear AND `active_workers < max_workers`.
  Adds one worker per tick.
- **Scale down** when `queue_depth < active_workers × target × 0.3`
  AND average per-worker utilisation `< 0.25` AND scale-down
  cooldown is clear AND `active_workers > min_workers`. Drains one
  worker per tick.
- Otherwise: `Hold`.

The asymmetric cooldown (5 min up, 10 min down) follows the standard
autoscaler pattern — react quickly to load, react slowly to drops to
avoid flapping.

## Tuning `target_queue_depth_per_worker`

The default `3` is a starting point. Tune from one week of
`argo worker elasticity status --json` snapshots:

- p50 queue depth ÷ p50 active workers ≈ your steady-state
  target.
- If brief latency p95 is acceptable at that depth, keep it.
- Drop the target if brief latency p95 exceeds your SLA; raise it
  if workers are under-utilised at the current target.

## Hard floor / ceiling

- `min_workers = 1` is the safe floor — a pool of `0` cannot accept
  inbound briefs.
- `max_workers` is tuned against host CPU / memory / per-worker
  concurrency budget. Auto-tuning the cap is **not** in scope; the
  recommender clamps hard.

## Promotion path

```
week 0: Observe        → just looking
week 1: RecommendOnly  → recommendations surface; pool untouched
week 2+: AutoScale     → recommender drives the pool
```

Flip back to `RecommendOnly` immediately if you see flapping (rapid
alternation between `ScaleUp` and `ScaleDown` recommendations within
one cooldown window). That usually means `target_queue_depth_per_worker`
is set too tight for your real burst pattern; widen it before
re-promoting.

## Observability

Metric surface on `/metrics/json`:

- `argo_worker_pool_active_total` — current active worker count.
- `argo_worker_pool_queue_depth`  — combined TODO + CLAUDE queue.
- `argo_worker_pool_target`       — recommender's last `target` (or
  the active count when `Hold`).
- `argo_worker_pool_scale_events_total` — counter object
  `{up, down}` since process boot.

Live status:

```sh
argo worker elasticity status        # JSON: snapshot + recommendation + counters
argo worker elasticity status --text # human-readable summary
```

Each tick also writes an audit row (`kind = "pool_elasticity_tick"`)
through structured tracing. Once the audit sink wiring lands the row
moves into the audit chain; the serde shape is stable today.

## Interaction with other elasticity briefs

- `mission-oom-restart-handler`'s "give up after N restarts" path
  releases the chronic-OOM worker's brief back to TODO, which raises
  queue depth and may trigger a scale-up here. This is by design:
  the failing worker is replaced AND the pool grows to absorb the
  backlog.
- `mission-e2b-reap-idle` reaps **E2B sandboxes**; this brief reaps
  **workers**. Different layers; no conflict.

## What this brief is NOT

- Cross-host worker pool scaling (single-host only in v1).
- Cost-aware scaling (LLM token spend as a scale signal) — v2.
- Per-brief-tier scaling — v2.
- Cloud-native autoscaler integration (write your own controller
  against the `/metrics/json` surface).
- Calendar-aware scaling — v2.
