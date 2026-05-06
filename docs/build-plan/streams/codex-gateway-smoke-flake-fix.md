# V1.1 — Fix gateway_smoke pre-existing flakes

**Owner**: codex. **Depends on**: nothing.

## Scope

The `gateway_smoke` and `gateway_smoke_writes` integration
test suites have known pre-existing flakes that have been
acknowledged across multiple PRs (#843 H-handoff-cancel
notes, codex-connector-stripe block at tick 60). These
flakes:
- Block `cargo test --workspace` on clean `origin/main`
- Are routinely cited as "outside brief touch list" by
  codex tabs implementing unrelated work
- Erode trust that `--workspace` tests actually pass

Confirmed flakes (verify each on clean main before fixing):
- `gateway_smoke_writes::session_delete_returns_400_on_bad_id_and_404_on_missing`
  — `no such table: session` — schema init order bug
- `gateway_smoke_writes::db_vacuum_and_prune_return_200`
  — same family
- `gateway_smoke::*` — database lock / listener timing
  flakes (broader; identify specific test ids during
  investigation)

This brief: investigate root cause + ship a focused fix.
Most likely root cause is that test setup doesn't run
migrations (or doesn't apply ALL migrations) before each
test — sqlite test DBs created without the H-stack tables.
Could also be a missing `migrate().await?` call in the test
fixture chain.

## Acceptance criteria
- [ ] Identify root cause for each flake (commit a short
      RCA note in the PR body)
- [ ] All 3 listed flakes pass on `cargo test --workspace`
      from a clean checkout, repeated 5x consecutively
- [ ] No cosmetic changes; the fix is the migration /
      fixture wiring
- [ ] Existing passing tests still pass (no regressions)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Update the "Known pre-existing flakes" list in
      BUILD_STATUS.md tick-60 amendment to remove the
      fixed entries

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- The actual smoke tests live in `crates/argo-cli/tests/`
  (NOT `crates/argo-gateway/tests/` — corrected tick66):
  - `crates/argo-cli/tests/gateway_smoke.rs` — narrowly:
    fixture / setup wiring only
  - `crates/argo-cli/tests/gateway_smoke_v2.rs` — same
  - `crates/argo-cli/tests/gateway_smoke_writes.rs` — same
  - `crates/argo-cli/tests/gateway_smoke_streaming.rs` —
    same
  - `crates/argo-cli/tests/gateway_smoke_admin.rs` — same
  - `crates/argo-cli/tests/gateway_smoke_show.rs` — same
- Also worth checking: `crates/argo-cli/tests/setup_wizard.rs`
  + `crates/argo-cli/tests/setup_check.rs` if the env-leak
  flake reported on `setup_steps::vertical_step_skips_empty_registry`
  (tick66 codex-activity-feed-cli boundary note) overlaps
  scope — that's an `ARGO_VERTICALS_DIR` env-isolation bug
  that may share root cause with the schema-init issues
- `crates/argo-gateway/src/lib.rs` — narrowly: ONLY if
  the root cause is a missing init step (e.g.
  `db.migrate()` not called on startup)
- `crates/argo-store/migrations/<next>__test_fixture_*.sql`
  (NEW) — ONLY if the root cause needs a new migration
- `docs/build-plan/BUILD_STATUS.md` — narrowly: remove
  the listed flakes from the tick-60 amendment list

## Branch + PR
- Branch: `codex-gateway-smoke-flake-fix`
- PR title: `gateway: fix pre-existing gateway_smoke flakes (codex-gateway-smoke-flake-fix)`
