# win-portability-cred-credentials-unix-perms

**Owner**: windows-fleet — **P1 PORTABILITY (won't-compile).**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Problem
`std::os::unix` (PermissionsExt / OpenOptionsExt / mode(0o600)) in
argo-config/src/cred/io.rs + argo-credentials (master_key.rs, lib.rs) does NOT
compile on Windows. The 0o600 cred-file lockdown is unix-only.

## Fix
`#[cfg(unix)]` the existing chmod-style perm code; add a `#[cfg(windows)]`
path. On Windows, restrict the cred/key file via an ACL if cheap, ELSE write
the file normally and emit a one-shot `tracing::warn!` that file-perm
hardening is unix-only (documented gap, NOT a silent downgrade). Keep the unix
path byte-identical. The file must compile + the cred round-trip tests pass on
BOTH unix (cargo test on conductor) and windows (cargo test here).

## Verify
`cargo test -p argo-config -p argo-credentials` green on Windows; conductor
re-runs on unix. Push branch + IN_REVIEW.
