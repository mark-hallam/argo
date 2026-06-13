# win-portability-tmp-and-home-paths

**Owner**: windows-fleet — **P1 PORTABILITY.**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Problem
~143 hardcoded `/tmp` references + `~/.argo` home expansion assume unix. On
Windows /tmp doesn't exist and ~ isn't expanded.

## Fix
Production (non-test) code: replace hardcoded `/tmp` with
`std::env::temp_dir()`; resolve the argo home via a cross-platform helper
(HOME on unix, USERPROFILE on windows — or the `dirs` crate if already a dep:
grep first). Do NOT churn test files that intentionally use unix tmp unless
they fail on Windows — the win-test-suite brief inventories those. Scope to
production paths that block the binary running on Windows.

## Verify
The gateway + cli locate temp/home dirs on Windows; `cargo test` for touched
crates green; conductor confirms unix unchanged. Branch + IN_REVIEW.
