# win-identity-fs-root-path-portability

**Owner**: windows-fleet — **P1 CROSS-PLATFORM CORRECTNESS — from #2911.**
**Depends on**: none.
**Authored against**: origin/main `83bce61f4` on `2026-06-13`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/TEST-FAILURES.md (#2911 first Windows test inventory).

## Problem
`argo-identity` fs_root persistence produces MIXED separators on Windows
(`...\SkycotDemo/Inbox`) — a path is joined with a literal `/` somewhere
instead of `Path::join`. Two `store_hire_fs_root` tests + a doctest fail; the
worker flagged this as likely a REAL path-join bug, not just a test literal
(one case even resolves a wholly different base dir on Windows —
home-relative vs configured). fs_root is the teammate's sandbox root, so a
wrong join is a real containment concern.

## Fix
Grep argo-identity fs_root construction (store_hire_fs_root + the env→fs_root
resolver `resolved_fs_root_from_env`). Replace literal `/` joins with
`Path::join` / `PathBuf`; make the env→fs_root resolution work on Windows
(USERPROFILE-aware, see #2904). Confirm the resolved base dir is correct on
Windows.

## Verify
On Windows: fs_root persists with consistent native separators + the env
resolver returns the right path; the 2 tests + doctest pass. Conductor
re-verifies argo-identity on unix unchanged. Branch + IN_REVIEW.
