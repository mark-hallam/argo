# At-rest encryption

SKYCOT supports opt-in, page-level encryption of the workspace
SQLite database via [SQLCipher](https://www.zetetic.net/sqlcipher/).
The wire is feature-gated; stock builds ship pure-Rust `sqlx`,
which is not linked against SQLCipher and writes plaintext pages.
Enterprise builds enable the `sqlcipher` Cargo feature and link
against a SQLCipher-compiled `libsqlite3`; from that point onward
the `[security.encryption] key_source` config controls whether
fresh connections issue `PRAGMA key`.

## Configuring the key source

Set the key source in `argo.toml`:

```toml
[security.encryption]
# One of:
#   "none"                                 — plaintext (default)
#   "env:ARGO_DB_ENCRYPTION_KEY"           — raw hex key from env var
#   "file:/var/run/secrets/argo/db.key"    — raw hex key from a file
key_source = "env:ARGO_DB_ENCRYPTION_KEY"
```

The key value itself must be exactly **64 lowercase hex characters
(32 bytes / AES-256)**. Trailing whitespace is stripped before
validation. Non-hex characters or wrong-length values fail loudly
on startup — fail-closed is intentional.

The key value is **never logged**. `Debug` for `KeySource` only
ever prints `<redacted>`; tracing spans that capture configuration
intentionally exclude the resolved key.

## Enabling on a new workspace

1. Generate a fresh 256-bit key:
   ```bash
   openssl rand -hex 32 > /var/run/secrets/argo/db.key
   chmod 0400 /var/run/secrets/argo/db.key
   ```
2. Point `[security.encryption] key_source` at the env var or file.
3. Restart the gateway / re-bootstrap the workspace. On first
   write the SQLCipher-linked driver encrypts every page.

## Rotating the key (in place)

Use `argo db rekey --new-key-source <spec>`:

```bash
# Generate the new key in its destination first.
openssl rand -hex 32 > /var/run/secrets/argo/db.key.next
chmod 0400 /var/run/secrets/argo/db.key.next

# Rotate. The old key is loaded from the current
# [security.encryption] key_source; the new key from --new-key-source.
argo db rekey --new-key-source file:/var/run/secrets/argo/db.key.next

# Atomically swap the secret file once rekey succeeds, then update
# argo.toml's key_source pointer and restart the gateway so future
# connections use the new key.
mv /var/run/secrets/argo/db.key.next /var/run/secrets/argo/db.key
```

`argo db rekey` issues `PRAGMA rekey` in-place — it does **not**
itself rewrite `argo.toml`. The operator is responsible for the
config-flip + restart after a successful rotation.

## Backups and key custody

> **Losing the key means losing the data.** SQLCipher pages are
> AES-256 encrypted; there is no recovery, no support escalation,
> no vendor reset that can decrypt a page-encrypted database
> without the key. This is the same security posture as full-disk
> encryption — operator-managed key custody is the security model.

Back up the key alongside (but not co-located with) the database
backup. A safe operator pattern is:

- Key lives in your secrets manager (Vault, KMS, 1Password Business,
  AWS Secrets Manager, etc.) and is materialised to the host at boot.
- The on-disk file (`/var/run/secrets/argo/db.key`) is on a `tmpfs`
  mount so it never persists across reboots.
- The DB backup goes to S3 / GCS / Backblaze B2; the key never goes
  to the same destination.

## Operational notes

- `argo db rekey` is the only supported rotation surface. There is
  no programmatic rekey API — it is intentionally CLI-only to make
  rotation a deliberate operator action.
- One DB = one key in v1. Per-tenant keys, HSM integration, and
  FIPS-140-3 validation are out of scope; SQLCipher Commercial
  Edition is the procurement seam for FIPS, and per-tenant keys
  are tracked under `mission-byok-enterprise-tenants`.
- Stock (no-SQLCipher) builds accept the `sqlcipher` feature toggle
  and a configured `key_source`, but the underlying driver
  silently drops `PRAGMA key` — there is no encryption until the
  binary is rebuilt against a SQLCipher-linked `libsqlite3`.
