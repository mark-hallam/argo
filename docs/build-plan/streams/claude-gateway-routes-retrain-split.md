# claude-gateway-routes-retrain-split — split routes_retrain.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `45c21803` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-gateway-routes-retrain-split`.

## Scope

Pure refactor split for `crates/argo-cli/src/handlers/gateway/routes_retrain.rs` (491 LOC). routes_retrain.rs at 491 LOC.

Mechanical split: identify a sibling cluster (helpers, tests, fixtures), move to a sibling file, preserve public surface, both files end under 400 LOC.

**Not in scope**: changing any function body; renaming pub items.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_retrain.rs` (extract content).
- NEW sibling file under same module.
- Parent  if it declares the module.

## Files you must NOT touch

- Any caller of the file's public items.

## Acceptance criteria

- [ ] Both files end under 400 LOC.
- [ ] Public surface unchanged.
- [ ] `cargo clippy -p <crate> --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-gateway-routes-retrain-split`
- PR title: `refactor: split routes_retrain.rs below 500-LOC ceiling (claude-gateway-routes-retrain-split)`

## Notes for the implementer

- ~30 min mechanical refactor; pure shape change.
