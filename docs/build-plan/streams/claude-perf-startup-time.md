# claude-perf-startup-time — Profile + optimise gateway startup (target: < 1s on M-series)

**Owner**: claude. **Depends on**: none.

> `argo gateway start` cold-starts in noticeably more than 1 second on
> M2/M3 hardware. For interactive flows and CI smoke runs, that
> latency is felt directly. This brief profiles the startup path and
> lands narrow optimisations to bring cold-start under 1s on
> M-series.

## Why

Startup latency is a tax on every CI run, every dev iteration, every
swarm-spawned runner. Reducing it pays back compounding. The cost
typically hides in:
- migration replay on every boot
- sync-blocking work during async init
- eager loading of subsystems that aren't used yet
- discovery / heartbeat probes blocking the main path

We don't know which yet — that's why this brief profiles first.

## Scope

- **NEW** `scripts/profile-startup.sh` (≤ 80 LOC): runs
  `cargo build --release -p argo-cli`, then runs the binary 5×
  with `--profile-startup` (or `time`) and reports cold + warm
  median startup. Optionally captures a flame graph if
  `cargo flamegraph` is available.
- **NEW** `--profile-startup` flag on `argo gateway start` that:
  emits structured timing for each init phase to stderr (
  `tracing` spans + `tracing_chrome` or similar) and exits before
  serving requests. Behind a `cfg(debug_assertions)` or env-gated
  build feature so it doesn't bloat release.
- **Code-side narrow optimisations** identified during profiling:
  examples (the actual list comes from the profile, but candidates
  to look at):
  - lazy-init non-critical subsystems (memory dreams_backend,
    eval runner, marketplace) until first request
  - skip migration replay if `PRAGMA user_version` matches expected
  - parallelise independent init tasks via `tokio::join!`
  - replace any `block_in_place` / sync-IO during init with async

## Approach

1. **Profile first.** Land the script + flag, run on author's M-series,
   record per-phase timings in PR body. Don't optimise blind.
2. Pick the **top 2-3 phases** by cost; ignore everything else.
3. Land minimal, targeted patches per phase. Each phase fix should
   be a separate commit in the PR for reviewability.
4. Re-run the script; record before/after numbers in PR body.

## Files you may touch

- `scripts/profile-startup.sh` (NEW, ≤ 80 LOC)
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly: emit
  per-phase tracing spans, support `--profile-startup`
- `crates/argo-cli/src/handlers/gateway/lifecycle.rs` — narrowly:
  whatever optimisations the profile recommends
- `crates/argo-cli/src/main.rs` (BLANKET CLI ALLOWLIST) — narrowly:
  one new clap flag
- Lazy-init wires in subsystem entry points (likely 1-3 files,
  identify during profile, list in PR description)

## Files you must NOT touch

- Schema / migrations
- Anything outside the boot path
- Test files (unless adding a startup-time smoke test)
- Other CLI subcommand implementations

## Acceptance criteria

- [ ] `scripts/profile-startup.sh` runs and reports cold + warm
      startup median + per-phase breakdown
- [ ] PR body shows before/after numbers + per-phase delta
- [ ] Cold start on author's M-series < 1000ms
- [ ] Warm start < 500ms
- [ ] No regression in `cargo test --workspace`
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] No file grew > 500 LOC; touched files stay near their pre-PR size

## Branch + PR

- Branch: `claude-perf-startup-time`
- PR title: `gateway: cold-start under 1s on M-series, profiled (claude-perf-startup-time)`

## Notes for the implementer

- **Don't pre-optimise.** Profile, look at the numbers, then patch
  the top offenders. Anything else is wasted reviewer-time.
- The `--profile-startup` flag exists for the **maintainer**, not
  end users — gate it behind a build feature or a `--hidden` clap
  attribute so it doesn't show up in `--help`.
- If the profile reveals that one phase dominates (e.g. migration
  check), this PR should be just that phase. Don't bundle three
  small wins where one big one fits.
- Record hardware in PR body (`sysctl -n machdep.cpu.brand_string`).
