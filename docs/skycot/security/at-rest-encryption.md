# At-rest encryption — v1 default posture

> **TL;DR (v1, post-`mission-sqlcipher-4-default-flip`):** every
> `argo gateway` boot now opens the session store through the
> SQLCipher path: the workspace-default tenant key is resolved via
> `argo_config::cred::db_key::get_or_create_db_key` (env →
> OS keyring → generate-and-persist), `PRAGMA key` is applied
> before any other SQL runs, and the on-disk pages are
> SQLCipher-encrypted. The build feature that links libsqlite3
> against SQLCipher is `argo-store/bundled-sqlcipher`; without it
> the pragma is still issued but is a silent no-op.
>
> Full-disk encryption (FileVault on macOS, LUKS on Linux,
> BitLocker on Windows) remains a complementary defence — page-
> level encryption protects backups and snapshots that the FDE
> layer can't reach.

## What ships in v1

SKYCOT v1 produces two release surfaces:

1. **Default `cargo build --release --bin argo`** — links the
   pure-Rust `sqlx` SQLite driver. `PRAGMA key` is silently dropped
   and the on-disk pages stay plaintext. Use this build only when
   FDE is the operator's only required control.
2. **`cargo build --release --bin argo --features
   argo-store/bundled-sqlcipher`** — links libsqlite3 with the
   SQLCipher engine via `libsqlite3-sys`'s bundled-SQLCipher
   feature (mission-sqlcipher-1-libsqlite3-patch). `PRAGMA key` is
   load-bearing on this build; pages on disk are AES-256-CBC
   encrypted with HMAC-SHA512 page MACs. This is the v1 enterprise
   demo path.

In both builds the boot path is the same — `argo gateway start`
calls `argo_config::cred::db_key::get_or_create_db_key`,
`argo_store::encryption::base64_to_hex`, and
`SessionStore::open_with_hex_key` in sequence. The only difference
is whether the underlying libsqlite3 honours the pragma.

## Complementary control: full-disk encryption

Page-level SQLCipher encryption protects the file contents at rest;
full-disk encryption protects the on-disk free list, swap, and any
sibling files (WAL, SHM, journal) that the SQLCipher layer cannot
reach mid-write. Both controls should be on for an enterprise
deployment:

- **macOS** — FileVault. System Settings → Privacy & Security →
  FileVault → Turn On.
- **Linux** — LUKS on the root partition (`cryptsetup luksFormat`).
- **Windows** — BitLocker, Pro/Enterprise SKU.

A stock (non-SQLCipher) build with FileVault on is still acceptable
when the threat model is laptop theft / disposal only. A SQLCipher-
linked build with FileVault off is acceptable when the threat model
is the backup-snapshot leak. Enterprise tenants enable both.

## Boot-time posture

`argo gateway start` no longer prints an "encryption OFF" advisory
(`mission-sqlcipher-4-default-flip` removed the
`#1820`-era warning). The boot path always resolves a SQLCipher key
via `argo_config::cred::db_key::get_or_create_db_key(TenantId::nil())`
and applies `PRAGMA key` before any other SQL runs. Whether the
pragma is load-bearing depends on the build feature
(`argo-store/bundled-sqlcipher`); the boot code is identical
either way.

`[security.encryption] acknowledge_plaintext` is retained as a
parseable field for backward-compatible argo.toml; it no longer
suppresses anything because the warning is gone. `argo doctor`
continues to report the current state under the `db:` block —
`argo doctor --json` consumers gate on the
`db.encryption.state` field as before.

### Existing plaintext DBs

If an operator upgrades the binary without first running
`argo migrate encrypt --db <path>`, the next gateway boot will
attempt `PRAGMA key` against the plaintext file. On a SQLCipher-
linked build the first non-pragma query fails with `file is encrypted
or is not a database` — that is the deliberate fail-loud signal
that the operator should run the migrate CLI before retrying.
Auto-migration is intentionally NOT performed: data-integrity
beats convenience.

## Encryption-key resolution

The symmetric key passed to `PRAGMA key` is resolved per-tenant by
`argo_config::cred::db_key::get_or_create_db_key(tenant_id)`. PR 4 of
the `mission-sqlcipher-*` sequence (`-default-flip`) wires this into
the gateway boot path via `argo-cli`'s `open_store_at` helper —
argo-store itself does not depend on argo-config (it would create a
circular dep through argo-core), so callers compose the two primitives.
The wire-in point is `crates/argo-cli/src/handlers/gateway/boot.rs`:
the workspace-default tenant `TenantId(Uuid::nil())` is the stable
keyring account across boots.

Resolution precedence (fail-closed on malformed inputs):

1. **`SKYCOT_DB_KEY_BASE64`** env var — base64-encoded 32 bytes.
   Intended for Fly ephemeral containers and CI runners where no
   OS keyring is available. Wrong length or non-base64 input is a
   hard error, not a silent fallback.
2. **OS keyring entry**, service `skycot.db.encryption`, account
   `tenant-<uuid>`. macOS routes through Keychain via the
   `security-framework`-backed `keyring` crate; Linux routes through
   the Secret Service D-Bus API (libsecret / GNOME Keyring /
   KWallet). Windows is out of scope for v1 (see CLAUDE.md "v1.0
   strategic decisions").
3. **Generate-and-persist** — fresh 32 bytes from `OsRng`, written
   back to the keyring so subsequent calls retrieve the same value.

Per-tenant isolation is enforced by the account name: each tenant
gets a distinct keyring entry. Sharing one key across tenants would
be a cross-tenant confidentiality regression.

Operator inspection commands (macOS):
```
security find-generic-password -s skycot.db.encryption -a tenant-<uuid>
```

## Operator runbook (SQLCipher-linked builds)

The procedural runbook for *enabling* SQLCipher in a custom build
(key generation, rotation via `argo db rekey`, backup-with-key-
separation, etc.) lives in
[`docs/skycot/runbook/at-rest-encryption.md`](../runbook/at-rest-encryption.md).
Read that doc once the SQLCipher-linked build is in hand.

The BYOK / cloud-KMS surface (envelope encryption with AWS KMS,
GCP Cloud KMS, Azure Key Vault) is documented separately in
[`docs/skycot/runbook/byok-cloud-kms.md`](../runbook/byok-cloud-kms.md).

## Roadmap

Default-on page encryption shipped in the v1.0 SHIP-GATE 4-PR
sequence (`mission-sqlcipher-1-libsqlite3-patch` through
`mission-sqlcipher-4-default-flip`). Remaining future work:

- BYOK cloud-KMS support is documented in
  [`docs/skycot/runbook/byok-cloud-kms.md`](../runbook/byok-cloud-kms.md);
  the keyring path is the v1 default, BYOK is an enterprise upgrade.
- `argo db rekey` key-rotation runbook lives in
  [`docs/skycot/runbook/at-rest-encryption.md`](../runbook/at-rest-encryption.md).
- Windows OS-keyring backend is out of v1 scope per CLAUDE.md "v1.0
  strategic decisions" (macOS-first demo path).
