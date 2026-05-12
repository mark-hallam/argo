# Mission load-bearing wire — SQLite encryption-at-rest (sqlcipher)

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-encryption-at-rest-sqlcipher`.

> SOURCE: MISSION_BACKLOG.md Section 7 #213. Enterprise tenants need at-rest encryption for the SQLite DB. SQLCipher is the de-facto path — drop-in replacement for `libsqlite3` with AES-256 page-level encryption keyed via PRAGMA.

## Scope

Add **opt-in** SQLCipher support behind a build-time feature flag + a runtime `[security] encryption.key` config field:

1. **Cargo feature** `sqlcipher` on `argo-store` that swaps the `sqlx` SQLite driver for SQLCipher-linked variant.
2. **Config field** `[security.encryption] key_source = "env:ARGO_DB_ENCRYPTION_KEY" | "file:/path/to/key" | "none"`. Default `none` (backward-compatible — SQLCipher build still works as plain SQLite when key is `none`).
3. **`PRAGMA key = '<hex>'`** issued at connection time (first statement before any other SQL) when key_source resolves.
4. **Re-key migration tool** — `argo db rekey --new-key-source <src>` for in-place key rotation (uses `PRAGMA rekey`).
5. **Documentation** in `docs/skycot/RUNBOOK.md` — how to enable, how to back up the key, why losing the key means losing the data.

This is NOT in scope: file-level encryption (this is row-level via SQLCipher); per-tenant keys (one DB = one key in v1); HSM integration; FIPS-140-3 validation (use SQLCipher Commercial Edition for that — separate procurement).

## Files you may touch

- `crates/argo-store/Cargo.toml` (≤ 30 LOC delta) — `[features] sqlcipher = ["sqlx/sqlite-sqlcipher"]` (verify exact feature name; sqlx may require a custom build flag instead)
- `crates/argo-store/src/lib.rs` (≤ 480 LOC after) — `apply_encryption_pragma` helper called from connection-pool init
- `crates/argo-store/src/encryption.rs` (NEW, ≤ 200 LOC) — `KeySource` enum + `resolve_key` + redacted Debug impl (NEVER log the key)
- `crates/argo-store/src/encryption_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 tests: env-source happy / file-source happy / none-source / invalid-hex / file-missing / rekey-round-trip
- `crates/argo-cli/src/handlers/db_rekey.rs` (NEW, ≤ 150 LOC) — CLI subcommand
- `crates/argo-cli/src/cli_db_cmds.rs` — `Rekey` variant
- `argo-config/src/sections.rs` (≤ 480 LOC after) — `[security.encryption]` section parse
- `docs/skycot/RUNBOOK.md` — append "At-rest encryption" section

## Files you must NOT touch

- Any other store migration; the audit-chain hash logic; existing test fixtures (they continue to run unencrypted)

## Acceptance criteria

- [ ] `cargo build --features sqlcipher` succeeds; default build unchanged
- [ ] Key from env / file / none all work; `none` is the default
- [ ] `PRAGMA key` runs as the first statement on every fresh connection when key is set
- [ ] `argo db rekey` rotates the key in place; reads after rekey work
- [ ] Key value NEVER logged (Debug impl redacted; tracing spans excluded)
- [ ] RUNBOOK section warns: "losing the key = losing the data"
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (both with and without feature)
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-encryption-at-rest-sqlcipher`
- PR title: `store: SQLCipher feature-gated encryption-at-rest (mission-encryption-at-rest-sqlcipher)`

## Notes

- Feature-gated so the default build doesn't pull a heavier dep. Enterprise builds enable `--features sqlcipher`.
- Key derivation: accept raw hex (64 chars = 32 bytes / AES-256) OR a passphrase that gets KDF'd via Argon2id. Document both; default to passphrase for operator usability.
- The CLI `argo db rekey` is the operational seam — DO NOT add a programmatic rekey API (too easy to misuse).
