# win-portability-cli-handler-unix-perms

**Owner**: windows-fleet — **P1 PORTABILITY (won't-compile).**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Problem
These argo-cli handlers use unix-only file-perm APIs and won't compile on
Windows: backup_create.rs, backup_verify.rs, audit_export.rs, gdpr_delete.rs,
genesis_export.rs, onboard_stages.rs, connector_authorize_state.rs,
gateway/routes_me_export.rs, self_update_install.rs, migrate_encrypt.rs,
gateway/bootstrap_token.rs, doctor_install_hooks.rs.

## Fix
cfg-gate each unix perm call (#[cfg(unix)] keep, #[cfg(windows)] no-op-or-ACL
with a documented-gap warn, same pattern as the cred brief). Group the edits;
keep functions <80 LOC, files <500. Unix behavior unchanged.

## Verify
`cargo build -p argo-cli` compiles on Windows; `cargo test -p argo-cli` for the
touched handlers green; conductor re-verifies on unix. Branch + IN_REVIEW.
