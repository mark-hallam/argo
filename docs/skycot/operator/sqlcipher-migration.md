# SQLCipher migration runbook — `argo migrate encrypt`

Operator runbook for `mission-sqlcipher-3-migrate-cli`. Use this once
your build has been re-linked against SQLCipher
(`mission-sqlcipher-1-libsqlite3-patch`) and you have an encryption
key persisted in `$SKYCOT_DB_KEY_BASE64` (or, post-PR 4, the OS
keyring via `mission-sqlcipher-2-keyring-wire`).

## When to run

- Existing SKYCOT deployment with one or more unencrypted SQLite
  databases under `~/.argo/` (the default path is
  `~/.argo/session.db`).
- You have re-linked the `argo` binary against SQLCipher and want to
  flip existing data to encrypted-at-rest **before** PR 4
  (`mission-sqlcipher-4-default-flip`) flips `SessionStore::open` to
  require an encrypted DB by default.

If you are bootstrapping a fresh workspace, you do not need this
runbook — PR 4 will create new DBs encrypted from the first write.

## Prerequisites

1. `argo` binary built with SQLCipher (verify with
   `scripts/verify-sqlcipher-link.sh`).
2. Encryption key persisted somewhere stable:
   - Today: `SKYCOT_DB_KEY_BASE64` env var (base64-encoded 32 bytes).
     Generate with `openssl rand -base64 32` and store in your secret
     manager.
   - Post-PR 4: per-tenant OS keyring entry under service
     `skycot.db.encryption`.
3. Free disk space ≥ 2× the size of the DB you are converting. The
   CLI verifies this and refuses to run otherwise.
4. The `argo` daemon / gateway / any process holding a write lock on
   the DB must be stopped first. Use `argo daemon stop` then confirm
   with `lsof <db-path>`.

## Run

Dry-run first to see what the CLI plans:

```bash
argo migrate encrypt --db ~/.argo/session.db --dry-run --json
```

If the JSON envelope reports `dry_run: true` with a backup path
plan, proceed:

```bash
argo migrate encrypt --db ~/.argo/session.db --yes
```

For non-interactive contexts (CI, scripted upgrades), `--yes` skips
the confirmation prompt. The CLI emits one status line on success;
add `--json` for machine-parseable output.

## What the CLI does

1. Reads `cipher_version()` PRAGMA against a plaintext open. Empty /
   error ⇒ source is unencrypted; non-empty ⇒ already encrypted
   (no-op).
2. Copies the source to
   `~/.argo/backups/<UTC-timestamp>/<basename>.unencrypted.db` and
   verifies the SHA-256 matches.
3. Issues `ATTACH DATABASE '<tmp>' AS encrypted KEY "x'<hex>'";`
   then `SELECT sqlcipher_export('encrypted');`. This is the
   canonical SQLCipher conversion pattern — do not hand-roll a
   table-walk.
4. Reopens the encrypted file with the key, asserts
   `cipher_version()` is non-empty, walks every user table, and
   compares row counts against the source. A mismatch fails loudly
   and leaves the source untouched.
5. Renames the original to `<db>.pre-encrypt.bak` and the encrypted
   tmp file to the original path. The pre-encrypt backup is
   intentionally not deleted — you remove it manually after a
   verification pass.

## Verify

```bash
# 1. The DB now requires the key to open. This should fail.
sqlite3 ~/.argo/session.db .tables

# 2. With the key it should open and list the same tables as before.
sqlite3 ~/.argo/session.db <<SQL
PRAGMA key = "x'$(echo $SKYCOT_DB_KEY_BASE64 | base64 -d | xxd -p -c 64)'";
.tables
SQL

# 3. Spot-check row counts against the .pre-encrypt.bak.
sqlite3 ~/.argo/session.db.pre-encrypt.bak "SELECT COUNT(*) FROM messages;"
sqlite3 ~/.argo/session.db "PRAGMA key = ... ; SELECT COUNT(*) FROM messages;"

# 4. Start the daemon, run a smoke chat, confirm no errors.
argo daemon start
argo chat --provider mock --prompt "hello"
```

## Roll back

If anything looks wrong after the migration, roll back by restoring
the `.pre-encrypt.bak` file the CLI left in place:

```bash
# Stop everything that might be touching the DB.
argo daemon stop

# Replace the encrypted file with the pre-encrypt backup.
mv ~/.argo/session.db ~/.argo/session.db.rollback-quarantine
mv ~/.argo/session.db.pre-encrypt.bak ~/.argo/session.db

# Confirm the rollback is the plaintext file you started with.
sqlite3 ~/.argo/session.db .tables
```

Once you are confident the migration is durable, delete both the
`.pre-encrypt.bak` and the dated `~/.argo/backups/<ts>/` directory.

## Idempotency

Re-running `argo migrate encrypt` against an already-encrypted DB is
a no-op — the CLI detects `cipher_version()` is non-empty and prints
"already encrypted; no-op". This is safe to wire into bootstrap
scripts.

## Troubleshooting

- **`cipher_version() unavailable`** — the binary is not linked
  against SQLCipher. Re-run `scripts/verify-sqlcipher-link.sh` and
  confirm `mission-sqlcipher-1-libsqlite3-patch` is merged.
- **`row-count mismatch`** — `sqlcipher_export` failed silently
  (extremely rare). The source DB is untouched; investigate the
  encrypted tmp file (`.encrypted.tmp` adjacent to the source) and
  file a bug. Do **not** retry until the underlying issue is
  understood.
- **`need ≥N bytes free`** — clear space or move the DB to a larger
  volume. The 2× requirement is intentional: the encrypt step writes
  a full copy before the swap.
