# claude-gateway-routes-retrain-tests-split — split routes_retrain_tests.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `5e810881` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-gateway-routes-retrain-tests-split`.

## Scope

Pure refactor split for `crates/argo-cli/src/handlers/gateway/routes_retrain_tests.rs` (499 LOC). gateway routes_retrain_tests at 499 LOC — gateway-zone is high-traffic

Mechanical split:

1. Identify natural extraction boundary (a sibling group of helpers/tests/fixtures).
2. Move it to a sibling file (e.g. `routes_retrain_tests_helpers.rs`).
3. Add the `mod` declaration to the parent module file.
4. Re-export any pub items so callers' imports still resolve.
5. Verify both files end under ~400 LOC.

Public surface unchanged.

**Not in scope**: changing any function body; renaming any pub item; introducing new tests.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_retrain_tests.rs` (extract content).
- NEW sibling file under the same module.
- The crate's `lib.rs` or parent `mod.rs` if it declares the module.

## Files you must NOT touch

- Any caller of the file's public items.

## Acceptance criteria

- [ ] Both files end under 400 LOC.
- [ ] Public surface unchanged.
- [ ] `cargo clippy -p <containing-crate> --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <containing-crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-gateway-routes-retrain-tests-split`
- PR title: `refactor: split routes_retrain_tests.rs below 500-LOC ceiling (claude-gateway-routes-retrain-tests-split)`

## Notes for the implementer

- ~30 min mechanical refactor.
- Pure shape change; no behavior change.
