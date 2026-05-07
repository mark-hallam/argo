# claude-bench-memory-query-perf — Criterion bench for memory query latency

**Owner**: claude. **Depends on**: claude-bench-session-throughput
(introduces criterion to workspace deps).

> The agent turn loop calls `compose_context` on every prompt — it's
> on the hottest path argo has. We have no measurement of its p50
> or p95 latency. This brief installs a Criterion bench that
> publishes both, so future memory-layer changes can be compared
> against a stable baseline.

## Why

`compose_context` walks the layered memory store, dedupes, ranks, and
returns the top-N entries for prompt construction. As memory volume
grows (especially after `F-memory-shared-team` and dreams_backend),
this query's latency directly bounds turn latency. Without numbers
we cannot tell when a "small refactor" silently 5×-es the latency.

## Scope

- **NEW** `crates/argo-memory/benches/query_perf.rs` — Criterion
  bench with three cases:
  1. **small**: 100 entries seeded, single query
  2. **medium**: 10k entries seeded, single query
  3. **large**: 100k entries seeded, single query
- Each case reports p50, p95, and p99 from Criterion's measurement
- **Modify** `crates/argo-memory/Cargo.toml` — `[[bench]]` entry +
  criterion dev-dep (workspace-inherited)

## Approach

1. Build a temp memory store, seed N entries with a deterministic
   distribution of layers (working / episodic / shared-team), tags,
   and embeddings (deterministic stub vectors — no real embedding
   provider in the bench).
2. Run `compose_context(query="...")` repeatedly via Criterion.
3. Use `Throughput::Elements(1)` (latency-per-call is the metric).
4. Use `sample_size(100)` and `measurement_time(Duration::from_secs(10))`
   so the percentile estimates are tight.

## Files you may touch

- `crates/argo-memory/benches/query_perf.rs` (NEW, ≤ 250 LOC)
- `crates/argo-memory/Cargo.toml` — `[[bench]]` + `[dev-dependencies]`
  block adding `criterion = { workspace = true }`. Touch is
  narrow — Cargo.toml allowed by BLANKET CLI ALLOWLIST does
  **not** extend to argo-memory; keep the change to bench-only
  additions and a brief code-comment.
- `crates/argo-memory/src/lib.rs` — narrowly: pub-export a test-only
  builder for `MemoryStore` if not already public (≤ 5 LOC change)

## Files you must NOT touch

- `compose_context` implementation
- Layer / dreams / shared-team logic
- Schema / migrations

## Acceptance criteria

- [ ] `cargo bench -p argo-memory --bench query_perf` runs clean
- [ ] Output reports p50, p95, p99 for all three sizes
- [ ] PR body records baseline numbers + hardware
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Bench file ≤ 250 LOC; non-bench files near-untouched
- [ ] Embedding stub is deterministic — bench numbers must be
      reproducible across runs (variance < 10%)

## Branch + PR

- Branch: `claude-bench-memory-query-perf`
- PR title: `bench: compose_context latency p50/p95/p99 (claude-bench-memory-query-perf)`

## Notes for the implementer

- Stable numbers matter more than fast numbers. If Criterion reports
  high variance, increase `measurement_time` rather than seeding more.
- 100k-entry seeding may dominate setup time; use `iter_batched_ref`
  with the seeded store as the reused input.
- If `compose_context` requires an async runtime, build a single
  `tokio::runtime::Runtime` outside the timed region and `block_on`
  inside.
