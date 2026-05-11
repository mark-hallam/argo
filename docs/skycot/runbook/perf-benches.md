# Perf Benches Runbook

Operator guide to the criterion-driven micro-benchmark suite that
landed under stream `mission-criterion-load-tests`. The benches
detect performance regressions on 5 load-bearing surfaces before
they reach production traffic.

## What's benchmarked

| Crate          | Bench file              | What it measures                               |
| -------------- | ----------------------- | ---------------------------------------------- |
| argo-runtime   | `agent_turn.rs`         | Planner per-turn overhead (LLM call mocked)    |
| argo-audit     | `audit_write.rs`        | Audit-row append rate (sequential + 8-parallel)|
| argo-cli       | `gateway_request.rs`    | HTTP request fan-out (serial + 16-parallel)    |
| argo-store     | `memory_store.rs`       | Sqlite append+read (batch + interleaved)       |
| argo-skill     | `select_variant.rs`     | A/B variant SHA-256 decision (10k iter)        |

## How to run

The fast path is the wrapper script:

```bash
scripts/run-bench.sh                # run all 5 benches sequentially
scripts/run-bench.sh --baseline     # save criterion baseline 'main'
```

Total wall-clock: 15–25 min on a warm laptop. Re-runs are cheap
once `target/` is warm.

To run a single bench (faster iteration when chasing a regression):

```bash
cargo bench -p argo-runtime --bench agent_turn
cargo bench -p argo-audit   --bench audit_write
cargo bench -p argo-cli     --bench gateway_request
cargo bench -p argo-store   --bench memory_store
cargo bench -p argo-skill   --bench select_variant
```

Output: criterion writes a per-bench HTML report under
`target/criterion/<group>/<case>/report/index.html`. The
`target/criterion/report/index.html` index links to every group.

## How to read the output

Each case prints a summary like:

```
agent_turn/loop_overhead_one_turn
                        time:   [4.8521 ms 4.9038 ms 4.9612 ms]
                        thrpt:  [201.56 elem/s 203.91 elem/s 206.10 elem/s]
                 change: time:   [-1.2% +0.8% +2.9%]
                         thrpt: [-2.8% -0.8% +1.2%]
                  None: No change in performance detected.
```

- **time** — `[lower 95% CI, point estimate, upper 95% CI]`. The
  point estimate is the most-likely mean per iteration. Take it
  as p50.
- **thrpt** — same shape, but per-second. Useful for the
  audit-write / gateway-request groups that emit throughput.
- **change** — same shape relative to the saved baseline. Only
  present when a baseline exists. Criterion classifies the change
  into: `No change`, `Change within noise threshold`, `Performance
  has regressed`, `Performance has improved`.

For p95 / p99: criterion does not surface percentiles directly on
the stdout summary. The full HTML report (`target/criterion/<group>/
<case>/report/index.html`) shows the per-sample distribution
chart; visually read p95 / p99 off that.

## Is it noise or signal?

> **Rule of thumb: ≥ 5% change on a 100-iteration bench is real
> signal. < 5% — run again.**

The benches are tuned to keep per-iteration noise under ~3%. A
regression that clears 5% on a single run is unlikely to be
sampling variance; a regression that does not clear 5% should be
re-run before any action.

Quick checks before chasing a "regression":

1. **Re-run the bench.** Single-run variance can hit 4-5%; two
   consecutive runs both showing the regression are much harder
   to explain by noise.
2. **Run on the affected crate alone.** Workspace state can vary
   if other tabs are also building.
3. **Check for thermal throttling.** Sustained cargo bench loads
   one core hard; on a hot laptop the second bench in a sequence
   may run slower than the first. Let the machine cool and re-run.
4. **Check `taskset` / process pinning.** A perf-comparison run
   should use the same core(s) across baseline and candidate
   runs to keep cache + branch-predictor state comparable.

When a regression is real:

1. **`git bisect run`** with a criterion comparison script
   between two commits. The 5% threshold is the bisect predicate.
2. **Re-run the bench on the suspect commit** to confirm.
3. **Look for the obvious suspects** — added allocator pressure,
   new sync points on a hot path, a `Vec::clone` in a loop,
   a regression in the `--release` profile (e.g. the `lto = "fat"`
   setting in the workspace Cargo.toml went away in a merge).

## Baselines and CI

The workflow file at `.github/workflows/bench.yml.disabled`
mirrors the `mission-cargo-audit-disabled-workflow` (#1325)
pattern. While GitHub Actions billing is in freeze (see
`docs/build-plan/BUILD_STATUS.md` header), the workflow stays
`.disabled` — it is reviewable but does not consume CI minutes.

To enable when billing resumes:

1. `git mv .github/workflows/bench.yml.disabled
   .github/workflows/bench.yml`
2. Commit + push. The workflow then runs weekly (Sun 02:00 UTC)
   and is dispatchable manually via `gh workflow run bench.yml`.
3. The first manual `workflow_dispatch` run will save a `main`
   baseline that subsequent weekly runs compare against.

## When to run benches

- **Suspect commit.** A diff that touches `crates/argo-runtime`,
  `crates/argo-audit`, `crates/argo-store`, `crates/argo-cli`'s
  gateway, or `crates/argo-skill`'s a/b path. Re-run the bench
  for the affected crate(s).
- **Before a release.** Run all 5 benches against the release
  candidate; compare to the prior tag's baseline. A regression
  ≥ 5% on any group is a hard release blocker.
- **Investigating a prod-perf incident.** Reproduce locally
  against the suspect commit + main; the bench gives a quick
  ratio without needing a full prod environment.

## When NOT to run benches

- **In the default pre-commit / pre-push hook.** Benches are
  slow (~20 min) and noise-prone on a busy laptop. The hook
  must stay under ~10s.
- **As a unit-test substitute.** The benches measure throughput
  / latency; they do not assert behavioral correctness. The
  per-crate `cargo test` suite is the correctness gate.
- **On every PR.** Cost-benefit is bad. The weekly cadence + a
  release-time pass catches drift early enough.

## Limitations and known-unmeasured surfaces

- **No real LLM call.** `agent_turn` uses a mock caller that
  returns in 5ms. The bench measures the planner's loop-internal
  cost; real-LLM-latency regressions need a separate brief.
- **No per-tenant isolation cost.** Multi-tenant write
  amplification (worker-presence, audit chain, etc.) is part of
  the sqlite path benched in `memory_store`, but not isolated.
- **No cross-machine baseline.** Bench numbers are
  machine-relative. Compare delta% across runs on the same
  machine, not absolute numbers across operators' laptops.
- **No kernel-level profiling.** For deep dives use `perf` or
  `flamegraph` (separate brief: `mission-flamegraph-tooling` —
  not yet in the queue).
- **Gateway request bench uses a minimal in-process listener.**
  `argo-cli` is binary-only so the bench cannot link to the
  gateway routes directly; the listener mimics the response
  shape. A regression in the real gateway's routes will not show
  up here.

## See also

- `scripts/run-bench.sh` — the wrapper
- `.github/workflows/bench.yml.disabled` — the workflow
- `docs/build-plan/streams/mission-criterion-load-tests.md` —
  the brief
- `docs/build-plan/streams/mission-cargo-audit-disabled-workflow.md`
  (#1325 precedent) — the disabled-workflow pattern this
  mirrors
