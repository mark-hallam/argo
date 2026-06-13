# win-line-ending-and-path-sep-test-audit

**Owner**: windows-fleet — **P2 PORTABILITY.**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Problem
Tests/code that hardcode `\n` line endings or `/` path separators or unix
absolute paths break on Windows (CRLF, backslash). 

## Fix
Grep for `assert*` on literal `/`-joined paths + `"\n"`-split assumptions in
test + production code that the build/test inventory flags. Use
`std::path::MAIN_SEPARATOR` / `Path` joins / normalize line endings. Scope to
what actually fails on Windows — don't churn cosmetic cases.

## Verify
Touched tests pass on Windows AND unix (conductor re-runs unix). Branch + IN_REVIEW.
