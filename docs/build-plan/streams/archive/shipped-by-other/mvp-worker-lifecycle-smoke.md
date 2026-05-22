# mvp-worker-lifecycle-smoke — end-to-end worker lifecycle integration test

**Owner**: claude. **Depends on**: nothing (uses already-shipped primitives). **Tier 1 MVP — proves the v1.0 happy path end-to-end.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

No single test today drives the full lifecycle: gateway boot → worker create → charter set → action proposed (Ask) → human approval → executor run → audit row recorded. This brief adds that test. Failing this smoke = v1.0 cannot ship.

## Approach (concrete)

NEW `crates/argo-cli/tests/worker_full_lifecycle_smoke.rs` (≤ 350 LOC). Steps:

1. Spin up gateway via existing test helpers (see `tests/gateway_smoke.rs` and `tests/common/mod.rs` for the proven pattern — listener-signal + tempdir ARGO_HOME).
2. POST `argo worker create` with a minimal charter via the CLI binary.
3. Trigger an Ask via `worker actions execute` against a manifest that has one auto-approval-gated action (use `verticals/finance-ar` or a minimal in-test fixture).
4. Approve the resulting Ask via `argo session approve`.
5. Assert the executor ran (action status = Completed in the audit row).
6. Assert at least one audit row with kind=ActionExecuted and matching session_id.

## Files you may touch

- `crates/argo-cli/tests/worker_full_lifecycle_smoke.rs` (NEW, ≤ 350 LOC)
- `crates/argo-cli/tests/common/mod.rs` — narrowly: hoist any reusable test-fixture helper if needed (≤ +50 LOC)

## Files you must NOT touch

- Any source under `crates/*/src/`
- Any vertical manifest beyond reading

## Acceptance criteria

- [ ] Test file ≤ 350 LOC
- [ ] `cargo test -p argo-cli --test worker_full_lifecycle_smoke` passes deterministically (run 3x to confirm not flaky)
- [ ] Test uses a tempdir for ARGO_HOME (no leak)
- [ ] Test cleans up gateway process on exit (use `tests/common/listener_signal.rs` pattern)

## Branch + PR

- Branch: `mvp-worker-lifecycle-smoke`
- PR title: `cli: end-to-end worker-lifecycle smoke test (mvp-worker-lifecycle-smoke)`
