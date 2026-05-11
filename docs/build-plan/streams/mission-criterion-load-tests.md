# Mission load-bearing wire — Criterion load-test benchmarks

**Owner**: claude. **Depends on**: existing crate test infrastructure; the `mission-cargo-audit-disabled-workflow` precedent (#1325-merged) shows the disabled-workflow + local-runnable pattern for "CI billing locked-out" period.
**Authored against**: origin/main `47749a1b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-criterion-load-tests`.

> SOURCE: MISSION_BACKLOG.md Section 1 #15 ("Add load tests via criterion for: agent-turn p99 latency, audit-write throughput, gateway request/sec, MemoryStore append/read, A/B select_variant"). **Tier M (perf observability).** Today the project has zero performance regression detection — a change that 3x-es agent-turn latency would ship undetected until production. This brief lands a criterion-driven micro-benchmark suite covering the 5 named perf-load-bearing surfaces.

## Scope

NEW criterion benches in 4 crates targeting the 5 perf surfaces:

### `crates/argo-runtime/benches/agent_turn.rs` (≤ 200 LOC)
Bench `agent_turn::p99_latency` — single-turn execution from input prompt to action-dispatch through a mock transport. Measures the runtime loop's per-turn overhead (loop-internal cost) independent of LLM call. Mock transport returns fixed 50-token response in 5ms. Group: `agent_turn`. Iterations: 100 per measurement.

### `crates/argo-audit/benches/audit_write.rs` (≤ 150 LOC)
Bench `audit_write::throughput` — sustained audit-row append rate to a fresh sqlite store. Two benches: (a) sequential append (baseline), (b) 8-worker parallel append (contention). Group: `audit_write`. Measures `rows_per_second`.

### `crates/argo-cli/benches/gateway_request.rs` (≤ 220 LOC)
Bench `gateway_request::rps` — boot a gateway w/ no provider keys (skip startup-check), hit `/health` and `/providers` with N parallel reqwest clients, measure `requests_per_second`. Two benches: serial (1 client) + parallel (16 clients). Group: `gateway_request`.

### `crates/argo-store/benches/memory_store.rs` (≤ 180 LOC)
Bench `memory_store::append_read` — `MemoryStore::append` followed by `MemoryStore::read_range` against a fresh sqlite store. Two benches: (a) write 1000 entries then read all, (b) interleaved 10x (append 100, read 100) measuring cache-warming behavior. Group: `memory_store`.

### `crates/argo-skill/benches/select_variant.rs` (≤ 150 LOC)
Bench `select_variant::ab_decision` — `A/BSelector::select(variants)` decision-time for skill-variant routing. Measures the in-memory weighted-random selection over N variants. Group: `select_variant`. Iterations: 10_000 (cheap).

NEW disabled workflow `.github/workflows/bench.yml.disabled` (≤ 100 LOC) — mirrors `mission-cargo-audit-disabled-workflow` (#1325) pattern. Runs `cargo bench` across the 5 benches w/ criterion's `--save-baseline` + `--baseline` comparison against `main`. Gated to weekly schedule + `workflow_dispatch`. Stays `.disabled` per the GitHub Actions billing freeze noted in BUILD_STATUS header.

NEW local-runnable script `scripts/run-bench.sh` (≤ 80 LOC) — runs all 5 benches sequentially, dumps criterion's HTML report to `target/criterion/`, summarizes regressions ≥ 5% from baseline.

This is NOT in scope: setting baseline thresholds that fail the CI build (that's a Tier M follow-up brief once we have 3-month perf history); end-to-end perf tests with real LLM calls (separate brief — much higher cost / variance); kernel-level profiling / flamegraphs (separate); per-tenant load tests with concurrent sessions (separate); the criterion-on-CI cost of dedicated bench-runner machines (separate ops decision).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/benches/agent_turn.rs` (NEW, ≤ 200 LOC)
- `crates/argo-runtime/Cargo.toml` — narrowly scoped: add `criterion` dev-dep + `[[bench]]` entry
- `crates/argo-audit/benches/audit_write.rs` (NEW, ≤ 150 LOC)
- `crates/argo-audit/Cargo.toml` — narrowly scoped: add `criterion` dev-dep + `[[bench]]`
- `crates/argo-cli/benches/gateway_request.rs` (NEW, ≤ 220 LOC)
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add `criterion` dev-dep + `[[bench]]`
- `crates/argo-store/benches/memory_store.rs` (NEW, ≤ 180 LOC)
- `crates/argo-store/Cargo.toml` — narrowly scoped: add `criterion` dev-dep + `[[bench]]`
- `crates/argo-skill/benches/select_variant.rs` (NEW, ≤ 150 LOC)
- `crates/argo-skill/Cargo.toml` — narrowly scoped: add `criterion` dev-dep + `[[bench]]`
- `Cargo.toml` (workspace) — narrowly scoped: add `criterion` to `[workspace.dependencies]` if not present
- `.github/workflows/bench.yml.disabled` (NEW, ≤ 100 LOC)
- `scripts/run-bench.sh` (NEW, ≤ 80 LOC, mode 755)
- `docs/skycot/runbook/perf-benches.md` (NEW, ≤ 200 LOC) — one-page operator runbook: how to run, how to interpret criterion's output, when to suspect regression vs noise

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema
- Existing src/test files (benches are NEW files only)
- Vertical content / frontend / channels

## Acceptance criteria

- [ ] `cargo bench -p argo-runtime --bench agent_turn` runs and emits criterion's HTML report under `target/criterion/agent_turn/`
- [ ] `cargo bench -p argo-audit --bench audit_write` runs (sequential + 8-parallel groups)
- [ ] `cargo bench -p argo-cli --bench gateway_request` runs (serial + 16-parallel groups)
- [ ] `cargo bench -p argo-store --bench memory_store` runs (write-then-read + interleaved groups)
- [ ] `cargo bench -p argo-skill --bench select_variant` runs (10k iterations on weighted selector)
- [ ] `bash scripts/run-bench.sh` runs all 5 + writes summary line per bench to stdout
- [ ] `.github/workflows/bench.yml.disabled` is valid GitHub Actions YAML (lint via `yq` or `actionlint` if available; otherwise inspect manually for `on:` / `jobs:` shape)
- [ ] Runbook covers: how to run, how to read p50/p95/p99 numbers, "this isn't noise" thresholds (>5% on 100-iteration benches = real signal; <5% = run again), what to do when regression is real
- [ ] `cargo clippy -p argo-runtime -p argo-audit -p argo-cli -p argo-store -p argo-skill --all-targets -- -D warnings` clean (benches included via `--all-targets`)
- [ ] `cargo test -p argo-runtime -p argo-audit -p argo-cli -p argo-store -p argo-skill` still passes (benches don't break unit tests)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-criterion-load-tests`
- PR title: `bench: criterion suite for 5 perf-load-bearing surfaces (mission-criterion-load-tests)`

## Notes for the implementer

- Use `criterion = { version = "0.5", default-features = false, features = ["cargo_bench_support", "html_reports"] }` as the dep — minimal feature set, no `tokio` feature unless the bench is async (only `agent_turn` and `gateway_request` need async; others are sync).
- For async benches: use `criterion::async_executor::TokioExecutor` so the runtime's mock-transport + reqwest fan-out work correctly. Don't try to roll your own async-bench wrapper.
- `agent_turn`: the mock transport returns a fixed-shape JSON response after `tokio::time::sleep(Duration::from_millis(5)).await` — this isolates loop-internal cost from real-LLM-cost. Use `criterion::black_box` on the input prompt + output action to prevent dead-code elimination.
- `audit_write`: parallel-append bench should use `tokio::task::JoinSet` to spawn 8 concurrent appenders to the SAME sqlite store. This exercises the WAL write path's contention behavior — which the just-merged WAL-tuning brief (#1334) makes more interesting.
- `gateway_request`: use `wiremock` to skip the actual provider startup-check, OR set `ARGO_GATEWAY_SKIP_STARTUP_CHECK=1` if that env-var exists (verify via `rg 'SKIP_STARTUP_CHECK\|skip_startup_check' crates/argo-cli/src/handlers/gateway/`). The bench should NOT hit a real provider.
- `memory_store`: use `tempfile::TempDir` for the sqlite store. The bench's setup-cost (DB open + ensure_schema) should be outside the `iter()` body so we measure append-read only.
- `select_variant`: 10k iterations because the operation is microsecond-scale; criterion needs many iterations to get statistical signal.
- The `.disabled` extension is the established pattern from the just-merged `mission-cargo-audit-disabled-workflow` (#1325) — workflow file exists but doesn't run until renamed. Operators rename to `.yml` when GitHub Actions billing is restored. Document this in the runbook.
- The runbook is the OPERATOR-FACING deliverable; the benches are the LOAD-BEARING signal. If a regression shows up, the operator needs to know whether to chase it (signal) or shrug (noise) — the runbook makes that call.
- DO NOT add `cargo bench` to the default pre-push hook — benches are slow and operators don't want them on every commit. The runbook should explicitly say "run on suspect commits + before releases."
