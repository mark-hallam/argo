# codex-test-naming-convention-sweep

**Authored against**: `origin/main` `dff06f840` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-test-naming-convention-sweep`.

**Fast-mode eligible**: yes — mechanical sweep.

## Summary

Rust test function names should follow `<scenario>_<expected_outcome>` for readability when the test suite reports failures. Audit the workspace's test functions and rename outliers to the convention. Pure cosmetic + readability — no behavior change.

## Acceptance

- [ ] Enumerate every `#[test]` and `#[tokio::test]` function across `crates/*/tests/` and `crates/*/src/**/*.rs` `#[cfg(test)]` blocks.
- [ ] For each test function, classify:
  - **OK** — name follows `<scenario>_<expected_outcome>` (e.g. `cron_create_stamps_workspace_tenant`, `empty_input_returns_none`).
  - **UNCLEAR-OK** — convention not followed but name is genuinely descriptive (e.g. `roundtrip_works`).
  - **RENAME** — short / non-descriptive (`test_foo`, `it_works`, `case1`).
- [ ] PR body table: file | original_name | suggested_name | classification.
- [ ] Rename RENAME-class functions. Keep UNCLEAR-OK as-is (low value to churn).
- [ ] No semantic change to test bodies. Use `git mv`-friendly diffs (rename only).
- [ ] All tests still pass post-rename.

## May touch

- Test function names across the workspace.
- Test-helper module names if they collide with new function names.

## Must NOT touch

- Production code.
- Test bodies / assertions.
- Integration test file names (those have a different naming convention).

## Notes

- Pre-flight: `rg '#\[(test|tokio::test)\]' crates/ -A 1 | head -40` for spot-check.
- Sizing: 200-500 LOC depending on how many renames. Pure mechanical.
- Helps the next year of debugging — `cargo test` failure output becomes self-documenting.
