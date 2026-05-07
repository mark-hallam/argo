# claude-bench-session-throughput — Criterion bench for session-create throughput

**Owner**: claude. **Depends on**: none.

> Argo has zero formal benchmarks today. Performance regressions land
> silently. This brief introduces the **first** Criterion bench in the
> tree — sessions/sec on `session create`. Future perf briefs build on
> the bench harness this brief installs.

## Why

`argo session create` is the entry point for almost every flow:
gateway, CLI, swarm runners. We have no number for "how many sessions
per second can a single argo binary create against a warm sqlite WAL?"
Without that number we can't:
- detect regressions when store schema or hot-path code changes
- size swarm fan-out (how many parallel agents can share a store?)
- justify or reject perf-flavoured PRs

## Scope

- **NEW** `crates/argo-cli/benches/session_throughput.rs` — Criterion
  bench, one group with two cases:
  1. cold store: fresh tempdir, fresh sqlite, measure first-1000-sessions throughput
  2. warm store: pre-seed 10k sessions, measure next-1000 throughput
- **Modify** `crates/argo-cli/Cargo.toml` — `[[bench]]` entry +
  `criterion` dev-dep
- **Modify** workspace `Cargo.toml` — add `criterion = "0.5"` to
  `[workspace.dependencies]` if not already present (this brief is
  the introducer)

## Approach

1. Reuse `argo_cli::handlers::session_new::session_new_for_test` (or
   the lowest-level fn that just constructs a `Session` row + persists
   to store) as the bench inner-loop. **Do not** spawn the full clap
   binary — that buries the signal in process startup.
2. Use `criterion::Criterion::default().sample_size(50)` and
   `iter_batched` with `BatchSize::SmallInput`.
3. Bench reports throughput in `sessions/sec` via `Throughput::Elements(N)`.
4. Run with `cargo bench -p argo-cli --bench session_throughput`.

## Files you may touch

- `crates/argo-cli/benches/session_throughput.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/Cargo.toml` (BLANKET CLI ALLOWLIST)
- `Cargo.toml` (workspace) — narrowly: add `criterion = "0.5"` to
  workspace deps if missing
- `crates/argo-cli/src/handlers/session_new.rs` — narrowly: pub-export
  a test-only helper if needed (≤ 5 LOC change)

## Files you must NOT touch

- Any other handler implementation
- Schema / migrations
- Other crate Cargo.tomls

## Acceptance criteria

- [ ] `cargo bench -p argo-cli --bench session_throughput` runs clean
- [ ] Bench output reports `sessions/sec` for both cold + warm cases
- [ ] Numbers recorded in PR description as the **baseline** for future
      perf work (no target threshold yet — this brief sets the bar)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] No bench file > 200 LOC; touched non-bench files unchanged in size
- [ ] Bench is **not** wired into CI (compile-only); it's a local tool

## Branch + PR

- Branch: `claude-bench-session-throughput`
- PR title: `bench: criterion harness for session create throughput (claude-bench-session-throughput)`

## Notes for the implementer

- Criterion is the de-facto Rust bench crate; pick `0.5` and pin in
  workspace deps so all future bench briefs inherit the same version.
- Keep the bench *purely* against the store layer — no tokio runtime
  if you can avoid it. If a runtime is needed, build it once outside
  the timed region.
- Record cold + warm numbers in the PR body; the values become the
  reference point for `claude-perf-store-batched-writes`.
