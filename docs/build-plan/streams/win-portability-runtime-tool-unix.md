# win-portability-runtime-tool-unix

**Owner**: windows-fleet — **P1 PORTABILITY (won't-compile).**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Problem
argo-runtime/src/oom_restart.rs (unix signal / proc memory) + argo-tool/src/
path_guard.rs use `std::os::unix`. OOM-restart self-heal + path-guard are
unix-coupled.

## Fix
cfg-gate. For oom_restart on Windows: either a Windows memory-pressure
equivalent (GlobalMemoryStatusEx via the `windows` crate) or a documented
no-op restart-disabled path with a warn — do NOT fake it (NO THEATER: if
OOM-restart can't work on Windows yet, say so in the cfg(windows) branch).
path_guard: port the canonicalize/symlink-escape check to a cross-platform
form (std::path + std::fs::canonicalize work on Windows; only the unix-perm
bits need gating).

## Verify
`cargo test -p argo-runtime -p argo-tool` green on Windows; conductor re-runs
on unix (oom_restart unix path MUST stay intact). Branch + IN_REVIEW.
