# win-computer-use-executor-first-build

**Owner**: windows-fleet — **P1 CROWN JEWEL — first real Windows compile.**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Context
`argo-executor-computer-use-windows` (2482 LOC, real `windows` 0.58 crate:
UIAutomation + SendInput + BitBlt) was written on macOS/Linux where its
`cfg(target_os="windows")` deps NEVER compile. This is its FIRST real build on
Windows. High-value, Windows-unique.

## Do
1. `cargo build -p argo-executor-computer-use-windows` on Windows (the
   target_os=windows deps now resolve for the first time).
2. Fix every compile error in the windows-crate code that was never validated
   (API signatures, feature flags, HWND/types, BitBlt buffer handling).
3. `cargo clippy -p argo-executor-computer-use-windows -- -D warnings`.
4. Commit docs/skycot/windows/CU-EXECUTOR-BUILD.md: what compiled, what was
   fixed, what's still stubbed (honest — NO THEATER).

## Verify
The crate compiles + clippy-clean on Windows. Branch + IN_REVIEW. (Smoke test
is the sibling brief win-computer-use-executor-smoke.)
