# fix-cu-macos-permission-revoke-mid-run-test-workspace-clippy-drift

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cu-macos-permission-revoke-mid-run-test-workspace-clippy-drift`.

🔴 **P0 CI HYGIENE — every workspace-clippy PR carries a boundary-note for this test file.**

Source: claude worker session end report 2026-05-23 (operator-relayed). Verified by conductor: PRs #2084, #2085, #2086, #2087, #2089, #2093, #2096 all carry boundary notes citing `crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs`. The drift surfaces during `bash scripts/workspace-clippy-fast-fail.sh` — crate-scoped `cargo check -p argo-executor-computer-use-macos --tests` passes clean on main, but the workspace gate trips.

This is the same "drift between crate-scoped clippy and workspace clippy" pattern that #2076 introduced via the `MacosCuRunRequest` field changes (#2076 added fields the #2074 test never updated). Every PR that hits the workspace gate has to write a HARD-RULE-2 boundary note acknowledging it — pollutes every PR body + de-risks worker confidence in the gate.

## Scope

1. Run `bash scripts/workspace-clippy-fast-fail.sh` against current main. Capture the exact warning(s) for `permission_revoke_mid_run_test.rs`.
2. Identify the drift: missing/extra struct fields, removed types, changed signatures — find via the warning + recent `MacosCuRunRequest` changes in git log.
3. Update the test file to compile clean against current `argo-executor-computer-use-macos` API. Preserve test intent — only update type bindings.
4. Re-run `workspace-clippy-fast-fail.sh` → assert clean.
5. Re-run `cargo test -p argo-executor-computer-use-macos --test permission_revoke_mid_run_test` → all tests still pass (the test's contract from #2074 — partial-failure audit-chain integrity — must stay green).

## Parallelism

- Manager-worker allowed: no
- Max children: 0

## Files you may touch

- `crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs` — narrowly scoped: update type bindings only
- `docs/build-plan/BUILD_STATUS.md` — standard row flip

## Files you must NOT touch

- The test's assertions or scenarios — only update bindings to match current API
- Production `crates/argo-executor-computer-use-macos/src/` — pure test fix
- Other cu-macos tests

## Acceptance criteria

- [ ] `bash scripts/workspace-clippy-fast-fail.sh` exits 0 (no boundary notes needed)
- [ ] `cargo test -p argo-executor-computer-use-macos --test permission_revoke_mid_run_test` still passes — #2074's contract preserved
- [ ] PR body explicitly states "closes the boundary-note regression on main; future PRs touching workspace clippy no longer need to declare a boundary"
- [ ] Every modified file <500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `fix-cu-macos-permission-revoke-mid-run-test-workspace-clippy-drift`
- PR title: `fix(cu-macos): un-drift permission_revoke_mid_run_test against current MacosCuRunRequest API (fix-cu-macos-permission-revoke-mid-run-test-workspace-clippy-drift)`

## Notes for the implementer

- ~20-80 LOC expected. Pure mechanical update of type bindings.
- The drift cascade: #2076 added `tenant_id`/`worker_id` fields to `MacosCuRunRequest` → all callers had to update → this test (from #2074, predates #2076) was missed in the sweep.
- **Don't refactor the test scenarios.** Just update the struct-literal binding sites to match current `MacosCuRunRequest::new(...)` shape.
- Worker session-end report cited 7 PRs explicitly — the noise has been compounding for hours.
