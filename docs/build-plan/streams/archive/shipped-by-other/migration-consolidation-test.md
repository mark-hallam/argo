# Stream — Migration consolidation test

**Owner**: codex. **Depends on**: nothing (independent;
helps every other stream by catching regressions).

## Scope

A single integration test that boots a fresh in-memory
sqlite DB and applies every migration in
`crates/argo-store/migrations/` in numerical order. Verifies:

- All migrations apply cleanly (no SQL errors).
- The expected tables exist after the full run.
- Idempotent re-application (running the migrate function
  twice is a no-op or rejects cleanly).
- A representative INSERT works on each new table from the
  H-stack migrations (worker_budget, session, handoff,
  shared_team_memory) — guards against schema drift.

This catches the failure mode that bit during the swarm: a
new migration silently broke the chain on fresh databases
even though existing dev DBs still worked.

## Files you may touch
- `crates/argo-store/tests/migration_chain.rs` (NEW)
- `crates/argo-store/Cargo.toml` — narrowly scoped: ONE dev-
  dependency line if a new dep is genuinely needed (e.g.
  `tokio-test`). Most likely none — the workspace already has
  `tokio` + `sqlx`.

## Files you must NOT touch
- Any existing migration file.
- Any existing source file in the workspace.
- `crates/argo-store/src/**` — read-only; the test exercises
  the public `migrate` API.

## Acceptance criteria
- [ ] `cargo test -p argo-store --test migration_chain`
      passes on a clean checkout.
- [ ] Test asserts all expected tables exist by their names
      (codex enumerates: `session` (chat), `skill`, `memory`,
      `worker`, `worker_budget`, etc — full list in the
      migrations).
- [ ] Test asserts each H-stack table accepts a sample
      INSERT. If a table doesn't exist yet (because its
      brief hasn't merged), SKIP that subtest with a
      `println!("missing: <table>; skipping")` line — do
      NOT fail. The test must remain green throughout the
      gradual H follow-up rollout.
- [ ] Test runs in <10 seconds (fast enough to keep in CI).
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-migration-consolidation-test`
- PR title: `store: integration test verifying migration chain applies cleanly (migration-consolidation-test)`

## Notes
- Use `SessionStore::open(":memory:").await` for the test
  store. Look at `crates/argo-skill/src/tests.rs` for the
  exact pattern.
- Idempotency check: after migrate(), call migrate() again
  and assert no error (or error type is "already applied",
  not "table exists"). Pick whichever the existing
  implementation returns.
