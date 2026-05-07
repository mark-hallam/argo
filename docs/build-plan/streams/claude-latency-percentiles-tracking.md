# claude-latency-percentiles-tracking — p50/p95/p99 action latency per executor kind

**Owner**: claude. **Depends on**: existing action dispatch path
(`worker_actions_executor.rs`) + argo-store metrics module.

## Scope

Today operators see total action counts but not how slow the slow path is.
This brief adds latency tracking: every action dispatch records a duration
into a per-executor-kind histogram, and `argo latency` prints
p50/p95/p99 (and max) per kind.

Histogram: bucket by power-of-2 ms (1, 2, 4, 8, 16, … 16384, +Inf).
In-process for v1; persisted to a small `latency_buckets` row per
(executor_kind, bucket) on a 60s flush.

## Approach (concrete)

1. **NEW** `crates/argo-store/src/latency_tracker.rs` (~280 LOC):
   - `LatencyTracker` — `Arc<Mutex<HashMap<ExecutorKind, [u64; N_BUCKETS]>>>`
   - `record(kind: ExecutorKind, duration: Duration)`
   - `snapshot() -> HashMap<ExecutorKind, [u64; N_BUCKETS]>`
   - `flush_to_store(store: &Store) -> ArgoResult<()>` — additive write
   - `percentile(buckets: &[u64; N_BUCKETS], p: f64) -> Option<Duration>`
2. **NEW** `crates/argo-cli/src/handlers/latency.rs` (~220 LOC):
   - `pub fn run(format: OutputFormat) -> ArgoResult<()>`
   - Reads persisted buckets, prints markdown table
3. **Modify** `crates/argo-cli/src/handlers/worker_actions_executor.rs`:
   - Wrap each `dispatch` call with `Instant::now()` + `tracker.record(kind, dur)`
4. **Modify** gateway boot — spawn 60s flush task.
5. **Modify** `crates/argo-cli/src/cli.rs` + `main.rs` — `argo latency` subcommand.

## Files you may touch

- `crates/argo-store/src/latency_tracker.rs` (NEW, ≤ 320 LOC)
- `crates/argo-store/src/latency_tracker_tests.rs` (NEW, ≤ 250 LOC)
  - 7 unit tests:
    - empty histogram → no percentiles (None)
    - single sample → p50=p95=p99=that sample
    - 100 samples uniform → p50 in expected range
    - bucket boundaries map correctly
    - flush + reload round-trips
    - per-kind isolation
    - concurrent record (`tokio::join!` 4 tasks recording 1k samples each)
- `crates/argo-cli/src/handlers/latency.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/latency_tests.rs` (NEW, ≤ 180 LOC) — 4 tests
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrow:
  wrap dispatch with timing
- `crates/argo-store/src/lib.rs` — narrow: 1 pub mod line
- `crates/argo-cli/src/cli.rs` — narrow: 1 subcommand
- `crates/argo-cli/src/main.rs` — narrow: 1 dispatch arm

## Files you must NOT touch

- argo-types — keep tracker types in argo-store
- Any executor stub — only the dispatch entry point
- argo-tui

## Acceptance criteria

- [ ] Every action dispatch records into the tracker
- [ ] `argo latency` prints markdown table: kind, n, p50, p95, p99, max
- [ ] `--format json` emits structured rows
- [ ] Buckets persisted via 60s flush; survives restart
- [ ] Concurrent recording is safe (no torn reads)
- [ ] 11 unit tests total
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-latency-percentiles-tracking`
- PR title: `latency: p50/p95/p99 per executor kind + argo latency (claude-latency-percentiles-tracking)`

## Notes for the implementer

- Power-of-2 ms buckets keep the schema tiny (≤ 16 buckets per kind) and
  percentile math is interpolation-within-bucket. Document the math.
- Schema: `latency_buckets(executor_kind TEXT, bucket_idx INTEGER, count INTEGER, PRIMARY KEY(executor_kind, bucket_idx))` with `INSERT … ON CONFLICT DO UPDATE SET count = count + ?`.
- In v1, `n` (total count) is the sum of bucket counts. Store as a derived
  field in the markdown table, not the schema.
