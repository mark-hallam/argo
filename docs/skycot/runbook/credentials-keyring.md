# Operator runbook — OS keyring backend for credentials

Argo workspace credentials live in `<workspace>/.argo/credentials.toml`
(mode `0600` on Unix). For local development the file backend is the
default. Operators in regulated environments, on shared workstations,
or on machines that already harden secrets via the OS-native vault
(macOS Keychain, Linux Secret Service / `gnome-keyring`, Windows
Credential Manager) can move credentials into the OS keyring with no
behaviour change for runtime callers — env-var precedence is
preserved.

## What the keyring backend gives you

- Secrets stored under the OS-native vault, encrypted at rest with
  the user-account keychain (macOS) / Secret Service master key
  (Linux) / DPAPI (Windows).
- Per-workspace isolation: the keyring service name is `argo:<8-hex>`
  where the hex is a SHA-256 prefix of the canonical workspace path.
  Two `argo` installs on the same machine (e.g. `~/code/argo` and
  `~/Desktop/argo`) write to distinct keyring entries even when the
  credential keys are identical.
- A clean migration path that is atomic — if a single keyring write
  fails, every prior write is rolled back and the file is left
  untouched.

## Quickstart

```bash
# 1. (Optional) Inspect what would migrate.
argo cred migrate-to-keyring --dry-run

# 2. Move credentials into the OS keyring. On success, the
#    credentials.toml file is removed.
argo cred migrate-to-keyring

# 3. Switch the runtime backend to keyring.
argo cred backend keyring

# 4. Verify env-var resolution still works.
argo doctor   # or any command that exercises the provider stack
```

## Backend selector

`argo cred backend <kind>` writes to `<workspace>/argo.toml`:

```toml
[credentials]
backend = "auto"   # or "file" or "keyring"
```

| Value | Behaviour |
| --- | --- |
| `file` | Always read from `<workspace>/.argo/credentials.toml`. |
| `keyring` | Always read from the OS keyring. Missing keys return `NotPresent`. |
| `auto` | Keyring when available + populated, otherwise file. **Default.** |

Env-var precedence is unchanged for every backend: a credential
present in the process environment (e.g. `OPENAI_API_KEY` exported by
the shell) wins regardless of the configured backend.

## Verifying keyring entries

### macOS

```bash
# Replace <hash> with the 8-char service hash from `argo cred …` output
# or compute via `printf %s "$(realpath .)" | shasum -a 256 | cut -c1-16`.
security find-generic-password -s "argo:<hash>" -a OPENAI_API_KEY -g
```

### Linux (Secret Service)

```bash
secret-tool lookup service "argo:<hash>" account OPENAI_API_KEY
```

### Windows (PowerShell)

```powershell
# Stored under "Generic Credentials" with target = "argo:<hash>".
cmdkey /list:"argo:<hash>"
```

## Manual delete / re-add

If a credential gets corrupted or needs rotating outside Argo, delete
the keyring entry through the OS-native tool above and re-add it via
`argo cred set <KEY>` (after temporarily setting `argo cred backend
file`) followed by `argo cred migrate-to-keyring`. There is currently
no `argo cred set --backend keyring` shortcut — additive surface, a
later brief.

## Migrating BACK to file mode

```bash
argo cred backend file
# Re-create credentials.toml entries (the migrate command deletes the
# file on success; values now live only in the keyring).
argo cred set OPENAI_API_KEY <value>
# (Optional) Delete the keyring entries with the OS-native tool above.
```

## When the keyring is unavailable

`argo cred migrate-to-keyring` refuses to run with a clear message if
the platform keyring backend is not active — common on minimal Linux
containers and headless CI runners that lack a Secret Service daemon.

Remedies:

- Linux: install and start `gnome-keyring` or `KWallet`, or use the
  D-Bus Secret Service from your distro's package set.
- Headless CI: keep `backend = "file"` and rely on file-mode + ACL
  hardening. Keyring on CI tends to be more friction than security.

## Failure semantics

- **Partial write during migration:** every prior keyring write is
  rolled back; `credentials.toml` is left intact. Operator can retry
  after addressing the underlying error.
- **Keyring entry corruption in the hot path:** when `backend =
  "auto"`, a corrupt or missing keyring entry falls through to the
  file backend transparently. When `backend = "keyring"`, the caller
  sees `NotPresent` — same surface as a missing entry in `file` mode.
- **Multi-workspace collision:** none, by construction. Service-name
  hashing isolates each workspace.

## Why no encryption-at-rest for `credentials.toml`?

That is a separate brief (`mission-credentials-file-encryption`).
This runbook covers the keyring backend only. For HSM / Vault / AWS
Secrets Manager integration see the BYOK roadmap entry.
