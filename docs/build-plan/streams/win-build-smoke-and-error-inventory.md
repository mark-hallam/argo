# win-build-smoke-and-error-inventory

**Owner**: windows-fleet — **P1 FOUNDATION — discovery.**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Goal
First real `cargo build --workspace` on actual Windows. Capture EVERY compile
error (this codebase has only ever been built on macOS/Linux). The deliverable
is the inventory, not green.

## Do
1. `cargo build --workspace 2>&1` — capture full output.
2. Group errors by crate + root cause (unix-only std imports, hardcoded paths,
   cfg gaps, missing windows deps).
3. Write docs/skycot/windows/BUILD-ERRORS.md: per-crate error list + a
   suggested-fix one-liner each + which queued win-* brief covers it (or NEW).
4. If a fix is <10 lines and obvious, you MAY include it; otherwise inventory
   only — the targeted win-portability-* briefs own the fixes.

## Verify
The report exists, is grouped, and cross-references the other win-* briefs.
Push branch + IN_REVIEW.
