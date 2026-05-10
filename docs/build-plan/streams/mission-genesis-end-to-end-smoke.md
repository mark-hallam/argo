# mission-genesis-end-to-end-smoke — single-turn Genesis → vertical → first-duty smoke

**Owner**: claude. **Depends on**: nothing (Genesis single-turn shipped #885). **Tier M — Mission MVP critical path.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: "operator types one sentence, gets a working autonomous worker." Genesis
single-turn shipped (#885) and worker-create exists, but there is no smoke test pinning
the claim end-to-end. Without it, regressions can silently break the demo path. This
brief is plumbing — it stitches existing primitives into a single integration test that
fails loudly if any link breaks. Multi-turn refinement is explicitly deferred.

## Approach (concrete)

1. **NEW** `crates/argo-cli/tests/genesis_smoke_e2e.rs` (~280 LOC):
   - Set up an isolated tmpdir as `ARGO_HOME`.
   - Invoke the CLI binary (or call the in-process handler) for:
     `argo genesis "build me a marketing intern named Maya who posts to Marketplace"`.
   - Assert the emitted artifacts on disk:
     - vertical TOML (manifest with worker name `maya`, role `marketing_intern`)
     - prompt pack (system + persona prompts present, non-empty)
     - action vocab (at least 1 action declared)
     - duty with cron `0 9 * * 1-5` (mon-fri 9am)
   - Invoke `argo worker create maya --from-genesis`; assert worker row in the store.
   - Advance the duty scheduler clock to fire the first cron tick (use the existing
     test-time-injection helper used elsewhere in the orchestrator tests).
   - Assert: an action lands in the action ledger for worker `maya`.
   - Assert: the audit chain hashes validate (call existing audit-validate helper).
2. **Modify** `crates/argo-genesis/src/lib.rs` ONLY if minor wiring is needed to expose
   a deterministic single-turn entry point for the test (e.g. a `#[cfg(any(test, feature
   = "test-helpers"))]` constructor that bypasses LLM by returning a fixed
   genesis-output fixture). NO new sub-modules; if more than ~30 LOC of wiring
   is needed, BLOCK with a note.
3. **NEW** `crates/argo-cli/tests/fixtures/genesis_maya_fixture.json` if step 2 path is
   taken — pre-recorded genesis output for deterministic test runs.
4. **No new feature code** beyond minimal test-helper wiring. The brief is a smoke test.
5. **Rustdoc**: any new `pub` test-helper carries doc + example.

## Files you may touch

- `crates/argo-cli/tests/genesis_smoke_e2e.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/tests/fixtures/genesis_maya_fixture.json` (NEW) — recorded fixture
- `crates/argo-genesis/src/lib.rs` — narrowly: ≤ 30 LOC of test-helper wiring if needed
- `crates/argo-genesis/Cargo.toml` — narrowly: `test-helpers` feature flag if needed

## Files you must NOT touch

- Genesis core logic (#885 already shipped).
- Worker create handler — call existing entrypoint.
- Duty scheduler core — call existing test-time helper.
- Audit chain — call existing validate helper.
- Any UI / gateway code.
- No new sub-modules under `argo-genesis`.

## Acceptance criteria

- [ ] Single integration test passes deterministically (no flaky LLM dependency).
- [ ] Test asserts vertical TOML emission (worker name `maya`, role marketing_intern).
- [ ] Test asserts duty with cron `0 9 * * 1-5` is registered.
- [ ] Test asserts first-duty cron fire produces an action ledger row.
- [ ] Test asserts audit chain validates.
- [ ] No new feature code beyond ≤ 30 LOC of test-helper wiring.
- [ ] `cargo test -p argo-cli --test genesis_smoke_e2e` clean.
- [ ] `cargo clippy -p argo-cli -p argo-genesis --all-targets -- -D warnings` clean.

## Branch + PR

- Branch: `claude-mission-genesis-end-to-end-smoke`
- PR title: `tests: end-to-end Genesis → first-duty smoke (mission-genesis-end-to-end-smoke)`
