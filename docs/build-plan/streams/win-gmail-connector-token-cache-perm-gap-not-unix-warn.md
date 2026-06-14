# win-gmail-connector-token-cache-perm-gap-not-unix-warn

**Owner**: windows-fleet — **P3 SECURITY hygiene — Gmail OAuth token-cache 0600 dropped SILENTLY on Windows.**
**Depends on**: none.
**Authored against**: origin/main `b7135a81d` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-gmail-connector-token-cache-perm-gap-not-unix-warn`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2-2026-06-15.md (finding 2 of 7).

## Problem
`crates/argo-mcp-connector-gmail/src/auth.rs` `write_cached_token` persists a live
Gmail OAuth **bearer access token** to `<cwd>/.argo/connectors/<tenant>/gmail/
access_token.json` (from `refresh_and_cache`, auth.rs:54). Both the open-time
`opts.mode(0o600)` and the post-write `set_permissions(0o600)` are inside
`#[cfg(unix)]` blocks with NO `#[cfg(not(unix))]` arm — so on Windows the 0600
hardening compiles out SILENTLY (zero warning), violating the project's
NO-THEATER convention (sibling secret-writers warn once on non-unix). P3 because
NTFS parent ACLs are usually already user-restricted, but the silent,
undocumented loss is the genuine defect.

## Fix
Add a `#[cfg(not(unix))]` one-shot (AtomicBool) `tracing::warn!` after the write,
naming the path + stating 0600 is unix-only — mirroring `argo-config/cred/io`.
Pure `perm_hardening_unix_only_note` for testability + a `#[cfg(not(unix))]`
test. Unix byte-identical.

## Verify
On Windows: `cargo test -p argo-mcp-connector-gmail --lib` green (incl. the new
`perm_hardening_note_names_file_and_gap`) + clippy clean. Conductor re-verifies
unix (the `#[cfg(unix)]` 0600 arms unchanged). Branch + IN_REVIEW.
