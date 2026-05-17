# mission-sqlcipher-3-migrate-cli — `argo migrate encrypt` upgrades existing unencrypted DBs (v1.0 SHIP-GATE, PR 3 of 4)

**Owner**: claude. **Sizing hint**: CLI subcommand + IO-heavy logic + backup discipline.
**Depends on**: `mission-sqlcipher-1-libsqlite3-patch` MERGED. Can run in parallel with `mission-sqlcipher-2-keyring-wire`.
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sqlcipher-3-migrate-cli`.
**Source**: 2026-05-18 audit Decision 4 — encryption-at-rest in v1.0. This is PR 3 of a 4-PR sequence.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — existing unencrypted DBs need a one-shot, idempotent upgrade path. Lose-data-on-migration is unacceptable.
- **Repeatability** (#3) — operator runbook must give a clear "how to encrypt my existing data" answer.

## Scope

P0 v1.0 SHIP-GATE. Add `argo migrate encrypt` CLI subcommand that converts an unencrypted SQLite database into a SQLCipher-encrypted one, with a guaranteed backup.

### Migration flow

```
argo migrate encrypt --db ~/.argo/session.db [--key-source <keyring|env>]
```

1. **Preflight checks**:
   - Source DB exists and is unencrypted (read `cipher_version()` PRAGMA — empty = unencrypted).
   - Free disk space ≥ 2× source DB size.
   - User has write permission to parent dir.
2. **Backup**:
   - Copy source DB to `~/.argo/backups/<timestamp>/<original-name>.unencrypted.db`.
   - Verify backup checksum matches source.
3. **Encrypt**:
   - Derive key via `argo_config::cred::db_key::get_or_create_db_key(tenant_id)` (or `SKYCOT_DB_KEY_BASE64` env var fallback).
   - Use SQLCipher's `ATTACH DATABASE ... AS encrypted KEY 'X';` + `SELECT sqlcipher_export('encrypted');` (canonical SQLCipher conversion pattern).
   - Write encrypted DB to `<original-path>.encrypted.tmp`.
4. **Verify**:
   - Open encrypted DB with derived key.
   - Run `PRAGMA cipher_version` — must return non-empty.
   - Compare row counts: source vs encrypted across every table.
   - If mismatch: fail loudly, leave source intact, delete `.tmp`.
5. **Atomic swap**:
   - `mv <original-path> <original-path>.pre-encrypt.bak`
   - `mv <encrypted.tmp> <original-path>`
   - On success: print "migration complete; remove .pre-encrypt.bak after verification."

### Tenant multiplicity

`SessionStore` shards per-tenant in the multi-tenant pool. CLI walks each tenant subdir and converts independently. Resumable: if the CLI is interrupted, re-running picks up where it left off (skips already-encrypted DBs).

### Dry-run flag

`--dry-run` prints what would happen without writing. Useful for documentation + audit.

LOC: ~500 (CLI dispatch + migration logic + backup + verification + tests).

## Files you may touch

- `crates/argo-cli/src/cli_subcommands_migrate.rs` (NEW or extend) — clap subcommand. ≤80 LOC.
- `crates/argo-cli/src/handlers/migrate.rs` (NEW) — dispatch. ≤30 LOC.
- `crates/argo-cli/src/handlers/migrate_encrypt.rs` (NEW) — main migration logic. ≤300 LOC.
- `crates/argo-cli/tests/migrate_encrypt_smoke.rs` (NEW) — E2E test: create unencrypted DB → migrate → reopen with key → verify rows. ≤120 LOC.
- `docs/skycot/operator/sqlcipher-migration.md` (NEW) — operator runbook. ≤100 LOC.
- `crates/argo-cli/src/cli_command_enum.rs` — register `Migrate` variant. ≤5 modified LOC.

## Files you must NOT touch

- `SessionStore::open` — still unencrypted-default until PR 4.
- The keyring module — read-only consumer (from PR 2).
- Existing unencrypted-DB tests — must not regress.

## Acceptance criteria

- [ ] `argo migrate encrypt --db <path>` converts an unencrypted DB to SQLCipher-encrypted in place.
- [ ] Backup at `~/.argo/backups/<timestamp>/` always exists post-run.
- [ ] Row counts match between source and encrypted across every table.
- [ ] Re-running on an already-encrypted DB is idempotent (no-op + clear message).
- [ ] `--dry-run` prints intended actions without writing.
- [ ] `crates/argo-cli/tests/migrate_encrypt_smoke.rs` passes.
- [ ] Operator runbook documents: how to run + how to verify + how to roll back.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-sqlcipher-3-migrate-cli`
- Title: `feat(cli): argo migrate encrypt — upgrade unencrypted SQLite to SQLCipher (mission-sqlcipher-3-migrate-cli) — PR 3/4`
- Worker verification: crate-scoped clippy + tests + manual smoke against a real demo DB.

## Hidden constraints

- **Never destroy data.** Backup is mandatory before any modification. If verification fails post-encrypt, source must remain pristine.
- **Atomic swap.** The CLI must not leave the user with a half-converted DB if interrupted.
- **Row-count check is necessary but not sufficient.** Pair with a per-table checksum (SHA-256 of canonical-ordered rows) for high-confidence integrity.
- **SQLCipher's `sqlcipher_export` PRAGMA is the canonical pattern** — do not hand-roll table-walks.
- **PR 4 is the wire-in point** for the default-flip + boot-warning removal.
