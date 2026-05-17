# At-rest encryption — v1 limitation + compensating control

> **TL;DR (v1):** The default `cargo build --release --bin argo`
> writes **plaintext** SQLite pages to `~/.argo/db.sqlite`. A leaked
> backup = a full data leak. Enterprise / regulated tenants must
> either ship a SQLCipher-linked build (operator-opt-in) OR rely on
> full-disk encryption (FileVault on macOS, LUKS on Linux,
> BitLocker on Windows) as a compensating control.
>
> The default-on flip is tracked in
> [`mission-sqlcipher-default-v1x.md`](../../build-plan/streams/mission-sqlcipher-default-v1x.md).

## What ships in v1

SKYCOT v1 ships the pure-Rust `sqlx` SQLite driver. `sqlx` does not
link against SQLCipher; it speaks the unencrypted SQLite
on-disk format. Two consequences operators must understand:

1. **`PRAGMA key` is a silent no-op.** Setting
   `[security.encryption] key_source` does not encrypt anything in a
   stock build. The pragma is issued, the driver ignores it, and the
   pages on disk remain plaintext.
2. **`~/.argo/db.sqlite` is human-readable.** Anyone with read access
   to the file (backup, snapshot, lost laptop) can `strings` the
   contents and recover audit rows, message bodies, memory entries,
   and skill IR.

The `sqlcipher` Cargo feature exists as a marker for downstream
builds; enabling it on the stock dependency tree changes nothing.
Real encryption requires building against a SQLCipher-linked
`libsqlite3` (commercial license or self-built SQLCipher fork).

## v1 compensating control: full-disk encryption

For the v1 demo path (macOS-first, see CLAUDE.md "v1.0 strategic
decisions") the operator-facing recommendation is:

- **macOS** — turn on FileVault. System Settings → Privacy & Security
  → FileVault → Turn On. Pages on disk are then encrypted at the APFS
  layer; loss of the laptop without the unlock key = no data leak.
- **Linux** — LUKS on the root partition (`cryptsetup luksFormat`).
- **Windows** — BitLocker, Pro/Enterprise SKU.

Full-disk encryption is **not equivalent** to page-level encryption:
it does not protect against an attacker who has booted the host or
who can pull a backup snapshot out of an unencrypted destination.
It does, however, cover the dominant v1 threat model
(laptop theft / disposal / unencrypted bucket leak via mis-routed
filesystem-level backup).

## Boot-time advisory

`argo gateway start` prints a one-line warning on every boot when:

- `[security.encryption] key_source = "none"` (the default), AND
- `[security.encryption] acknowledge_plaintext = false` (the default)

```
argo gateway: ⚠️  at-rest encryption OFF (plaintext pages). v1 stock
build does not link SQLCipher. Set `[security.encryption]
acknowledge_plaintext = true` to suppress; see
docs/skycot/security/at-rest-encryption.md.
```

To suppress the warning once the operator has accepted the v1
limitation (e.g., FileVault is on and the deployment is single-
operator macOS):

```toml
[security.encryption]
acknowledge_plaintext = true
```

`argo doctor` always reports the current state under the `db:` block,
regardless of suppression — `argo doctor --json` consumers can gate
on the `db.encryption.state` field.

## Encryption-key resolution (post-`mission-sqlcipher-2-keyring-wire`)

Once a SQLCipher-linked build is in play (tracked in
`mission-sqlcipher-default-v1x`), the symmetric key passed to
`PRAGMA key` is resolved per-tenant by
`argo_config::cred::db_key::get_or_create_db_key(tenant_id)`. PR 4
of the `mission-sqlcipher-*` sequence wires this into
`SessionStore::open`; PR 2 (this) ships the API only.

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

Default-on page encryption (Path A — wire SQLCipher into the
default cargo build) is tracked in
[`mission-sqlcipher-default-v1x.md`](../../build-plan/streams/mission-sqlcipher-default-v1x.md).
The trade is a dependency on a SQLite-source/SQLCipher build (C
toolchain on the build host, ~2-3 min cold build cost) — acceptable
post-v1 once the first enterprise design partner is in flight.
