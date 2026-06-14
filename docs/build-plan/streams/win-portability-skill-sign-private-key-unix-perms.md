# win-portability-skill-sign-private-key-unix-perms

**Owner**: windows-fleet — **P2 SECURITY — ed25519 skill-signing private key written without 0600, SILENTLY, on Windows.**
**Depends on**: none.
**Authored against**: origin/main `b7135a81d` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-portability-skill-sign-private-key-unix-perms`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2-2026-06-15.md (finding 1 of 7).

## Problem
`crates/argo-skill/src/sign.rs:354` — the `#[cfg(not(unix))]` arm of
`write_secret_file` silently delegates to plain `fs::write` (no permission
hardening, NO warning). The unix arm opens with `mode(0o600)`. This persists the
cosign-style **ed25519 PRIVATE signing key** (`write_keypair_to_dir` sign.rs:159,
reached from the production `argo skill keygen` handler `skill_keygen.rs:13`), so
on Windows the private key inherits the parent dir's (typically Users-readable)
ACLs. It is the LONE secret-writer with no NO-THEATER warning — every sibling
(argo-credentials, argo-config/cred/io.rs:281, gateway/bootstrap_token.rs) emits
a one-shot warning on non-unix.

## Fix
Mirror the established sibling pattern: in the `#[cfg(not(unix))]`
`write_secret_file` arm, `write_file` then emit a one-shot (AtomicBool-gated)
`tracing::warn!` naming the path + stating 0600 is unix-only — so the downgrade
is OBSERVABLE, not silent. Pure `perm_hardening_unix_only_note` for testability +
a `#[cfg(not(unix))]` test asserting the note. Unix path byte-identical.

## Verify
On Windows: `cargo test -p argo-skill --lib` green (incl. the new
`perm_hardening_note_names_file_and_gap`) + clippy clean. Conductor re-verifies
unix (unix 0600 arm unchanged). Branch + IN_REVIEW.
