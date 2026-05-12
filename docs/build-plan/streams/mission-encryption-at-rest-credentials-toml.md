# Mission load-bearing wire — Encrypt credentials.toml at rest

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-encryption-at-rest-credentials-toml`.

> SOURCE: MISSION_BACKLOG.md Section 7 #214. Connector API keys / OAuth tokens live in `~/.skycot/credentials.toml` (or workspace `argo.toml [credentials]` section). Today plain-text. Enterprise tenants need at-rest encryption for the credentials file independently of the DB encryption brief.

## Scope

Encrypt credential values via `age` (RFC 9580-like) under a workspace-master-key:

1. NEW `argo-credentials` crate with `read_encrypted(path, master_key) -> CredentialMap` + `write_encrypted(...)`.
2. File format: TOML wrapping `value = "<age-encrypted-blob-base64>"` per entry. `master_key` lives outside this file (env / OS keychain / hardware).
3. CLI `argo credentials encrypt-all` migrates a plaintext file in place.
4. Backward-compat: if a value field is plain string (not `{ value = "..." }` envelope), treat as plaintext (legacy single-tenant deployments).
5. Master-key resolution: `ARGO_CREDENTIALS_MASTER_KEY=<base64>` env, fallback to `~/.skycot/.master_key` file (0600 perms), fallback to error.

This is NOT in scope: OAuth token rotation (separate brief); per-credential ACL (one master key = one workspace in v1); HSM-backed master keys; FIDO2 unlock.

## Files you may touch

- `crates/argo-credentials/` (NEW crate, ≤ 600 LOC total)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 150 LOC) — public API
  - `src/age_wrap.rs` (NEW, ≤ 200 LOC) — `age`-crate-backed encrypt/decrypt helpers
  - `src/master_key.rs` (NEW, ≤ 150 LOC) — env/file resolution
  - `src/lib_tests.rs` (NEW, sibling, ≤ 250 LOC) — 8 tests: encrypt-roundtrip / mixed-plain-and-encrypted / missing-master-key / wrong-master-key / migrate-all-plaintext / file-perms-warn / etc.
- `Cargo.toml` workspace — add `argo-credentials` to members
- `crates/argo-cli/src/handlers/credentials_encrypt_all.rs` (NEW, ≤ 180 LOC) — CLI
- `crates/argo-cli/src/cli_credentials_cmds.rs` — `EncryptAll` variant (verify file exists; else `cli.rs`)

## Files you must NOT touch

- The existing connector credential-read paths in connector crates — those continue to receive decrypted values; the encryption is transparent at read time
- `argo-store` — credentials don't live in the DB

## Acceptance criteria

- [ ] NEW `argo-credentials` crate compiles standalone
- [ ] Round-trip: encrypted file decrypts back to identical map
- [ ] Mixed plaintext + encrypted entries supported (back-compat)
- [ ] Master-key resolution: env > file > error (no silent default)
- [ ] Master-key NEVER logged (Debug redacted)
- [ ] `argo credentials encrypt-all` migrates existing files; warns if file perms > 0600
- [ ] `cargo clippy -p argo-credentials --all-targets -- -D warnings` clean
- [ ] All new files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-encryption-at-rest-credentials-toml`
- PR title: `credentials: at-rest age encryption (mission-encryption-at-rest-credentials-toml)`

## Notes

- `age` crate (https://crates.io/crates/age) is the canonical Rust impl; lightweight (~70k LOC) and well-audited.
- Master-key on disk at 0600 is a deliberate operator-tradeoff; document the security model (an attacker with shell access still wins; the protection is against backup/snapshot exfil + accidental file sharing).
- Don't conflate this with the DB encryption brief — different threat models, different keys.
