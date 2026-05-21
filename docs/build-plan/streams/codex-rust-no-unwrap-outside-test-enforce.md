# codex-rust-no-unwrap-outside-test-enforce

**Authored against**: `origin/main` `dff06f840` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rust-no-unwrap-outside-test-enforce`.

**Fast-mode eligible**: yes — mechanical scan + fix.

## Summary

CLAUDE.md invariant: "No `unwrap`/`expect` outside `#[cfg(test)]`". A `scripts/check-no-unwrap-outside-test.sh` may already exist (verify). Re-run the audit; fix any violations.

## Acceptance

- [ ] Enumerate every `.unwrap()` and `.expect(` callsite across `crates/*/src/**/*.rs`.
- [ ] For each, classify:
  - **TEST-CONTEXT** — inside `#[cfg(test)]` block or `tests/` dir. OK.
  - **DOC-EXAMPLE** — inside a doc-comment `///` block. OK (doc examples are illustrative).
  - **BUILD-SCRIPT** — inside `build.rs`. Acceptable for now (separate enforcement track).
  - **PRODUCTION-VIOLATION** — production-path code. **FIX**.
- [ ] For each PRODUCTION-VIOLATION:
  - Replace `.unwrap()` with `.expect("descriptive context for invariant")` if the call genuinely cannot fail (`debug_assert!`-style invariant). Add a comment explaining why.
  - Replace with `?` propagation if the caller signature allows.
  - Add a `match` or `if let` if the recovery path is genuinely needed.
- [ ] PR body table: file:line | pattern | classification | fix.
- [ ] All existing tests pass.
- [ ] If `scripts/check-no-unwrap-outside-test.sh` exists, it exits 0 post-fix.

## May touch

- Production-path Rust files where a violation is fixed.
- Test code if a fix introduces a new test scenario.

## Must NOT touch

- Test-context callsites (those are intentional).
- Doc-comment examples.
- `unsafe` blocks (they have their own discipline — separate brief).

## Notes

- Pre-flight: `rg '\.unwrap\(\)|\.expect\(' crates/ --type rust | wc -l` for total count.
- Pre-flight: `find scripts -name 'check-no-unwrap*' 2>/dev/null` to find any existing script.
- Sizing: 100-400 LOC depending on violations. Mostly a 1-2 LOC swap per callsite.
- v1.0 launch hygiene — `.unwrap()` panics in production are the #1 source of "the app crashed" reports.
