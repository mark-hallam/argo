# claude-approval-mobile-deep-link-prep — code-only deep-link handler refactor

**Owner**: claude. **Depends on**: nothing. **Phase B mobile/UX narrow.**

**Authored against**: origin/main 2026-05-09 (split-out from `claude-approval-mobile-deep-link-improvements` per conductor; the parent brief required mobile-device QA which workers can't satisfy. This sub-brief is the code-only portion that ships now).

## Why

The mobile deep-link handler currently has rough edges in URL parsing, fallback behaviour, and error-state rendering. The parent brief (`claude-approval-mobile-deep-link-improvements`) bundled a code refactor with on-device manual QA — but the QA gate cannot be satisfied by current workers (no mobile build/test environment). This sub-brief lands the **code-only** parts that have unit-testable contracts.

## Approach (concrete)

1. Refactor the deep-link parser (likely in `crates/argo-cli/src/handlers/approval_*.rs` or similar — verify) to a pure-fn shape that takes a URL string and returns a typed enum of intent.
2. Add fallback handling: malformed URL → clear error rather than panic; missing path → default to inbox view.
3. Unit-test every branch of the parser (happy path, malformed, missing-path, unrecognised-action).
4. Keep all changes pure-Rust + unit-testable; do NOT add anything that requires a device.

## Files you may touch

- The existing approval-deep-link handler module (locate via `rg 'fn.*deep_link' crates/argo-cli/src/handlers/`)
- A sibling `*_tests.rs` file
- `crates/argo-cli/src/handlers/mod.rs` — narrowly if a NEW module is added

## Files you must NOT touch

- Any TUI / mobile build manifest
- Anything requiring device-side validation — that lands via the parent brief once a QA worker is available

## Acceptance criteria

- [ ] Pure-fn parser returns typed enum for every input shape
- [ ] Malformed URLs return clear error, do NOT panic
- [ ] ≥6 unit tests covering happy + malformed + missing + unrecognised branches
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (no new failures vs HARD RULE 2 baseline)
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-approval-mobile-deep-link-prep`
- PR title: `mobile: deep-link parser refactor (code-only prep) (claude-approval-mobile-deep-link-prep)`

## Notes for the implementer

- This is a refactor with strong unit-test coverage. No on-device behaviour changes are in scope.
- Once this lands, the parent brief `claude-approval-mobile-deep-link-improvements` can be re-evaluated for the device-QA portion.
