# Credentials configuration — per-tenant scoping

Brief: `mission-cred-get-tenant-parameter` (P0 fix from `eval-credentials-cross-tenant-isolation` / #2080 §4).

## Background

Up to and including #2081, `argo_config::cred::get(name)` was tenant-blind: every worker reading a credential from `<workspace>/.argo/credentials.toml` saw the same root-level keys regardless of which tenant the worker belonged to. In a multi-tenant gateway this is a by-construction cross-tenant credential leak — `tenant_a`'s worker could ask for a Slack bot token and receive `tenant_b`'s value if the operator had stored either tenant's secret at the workspace root.

From this PR onward the file is structured per tenant.

## File layout

`<workspace>/.argo/credentials.toml`:

```toml
# Canonical per-tenant section (used by all production callsites).
[credentials.tenant."01975e4f-8a7a-7c10-9c44-3d4b6f93a5ee"]
SLACK_BOT_TOKEN = "xoxb-aaaa"
GITHUB_TOKEN    = "ghp_aaaa"

[credentials.tenant."01975e4f-9c12-7c10-9c44-3d4b6f93a5ee"]
SLACK_BOT_TOKEN = "xoxb-bbbb"

# Legacy root-level section. Read-only fallback during migration —
# every (tenant, key) hit here logs a `tracing::warn!` once.
[credentials]
ANTHROPIC_API_KEY = "sk-ant-..."
```

- **Per-tenant sections**: `[credentials.tenant."<TenantId>"]`. Tenant IDs are UUIDs; the wrapping quotes are required.
- **Legacy root-level section**: `[credentials]`. Read-only. New writes from `argo creds set --tenant <tid>` always go to the per-tenant section.

## Code-side API

`argo_config::cred` exposes two read paths:

| Function | Signature | Use when |
| --- | --- | --- |
| `get` | `fn get(tenant_id: &TenantId, key: &str) -> Result<String, VarError>` | The caller has tenant identity (worker context, request principal, audit row). **Default.** |
| `get_unscoped` | `fn get_unscoped(key: &str) -> Result<String, VarError>` | The caller legitimately has no tenant identity. Examples: CLI `argo doctor`, gateway startup probe, single-tenant local dev. Every call site MUST carry a `// tenant-unscoped: <reason>` comment. |

`get_with_backend(workspace, tenant_id: Option<&TenantId>, key, backend)` is the underlying primitive; `tenant_id = None` is the same as `get_unscoped`.

### Lookup precedence (tenant-scoped)

1. Process environment (`std::env::var(key)`).
2. The tenant's section: `[credentials.tenant."<tid>"]`.
3. The legacy root-level section, with a `tracing::warn!` (one warn per (tenant, key, backend) per process).

### Lookup precedence (unscoped)

1. Process environment.
2. The legacy root-level section only — per-tenant sections are **never** consulted by `get_unscoped`.

Emits a `tracing::warn!` on every unscoped call so the audit surface stays visible.

## Migrating a workspace

The credentials file is operator data; this codebase never edits it without explicit user action.

1. Identify the tenant ID(s) that own each credential. The web admin (`/settings/tenants`) lists tenant UUIDs.
2. Move keys from `[credentials]` to `[credentials.tenant."<tid>"]` by hand, or:
3. Use the migrate helper (separate brief — `argo creds migrate-to-tenant <tid>` moves the entire root-level section into the named tenant section, then empties the root).

While migrating, both shapes coexist. Reads through `get(tid, key)` consult the tenant section first and fall through to root with a warning. Once you have migrated every key, the warning stops firing.

## Operator runbook touchpoints

- **`tracing::warn! target=argo_config::cred backend=… message="credential found via legacy root-level fallback"`**: a tenant-scoped read fell through to the root section. Migrate the named key.
- **`tracing::warn! target=argo_config::cred message="tenant-unscoped credential read (caller bypassed tenancy isolation)"`**: a caller used `get_unscoped`. This is legitimate for some surfaces; audit if you see it under a request-handling path.

## Backends

The choice of file vs OS keyring (set in `argo.toml` `[credentials] backend = "file"|"keyring"|"auto"`) is orthogonal to tenant scoping. The keyring backend stores per-tenant entries under the key `tenant/<tid>/<KEY>` and falls back to the un-prefixed key for legacy entries.

## Encryption

Per-entry encryption (`mission-credentials-secret-newtype-at-decrypt`) is a separate concern from tenancy. Encrypted entries remain valid in either section shape:

```toml
[credentials.tenant."01975e4f-8a7a-7c10-9c44-3d4b6f93a5ee".SLACK_BOT_TOKEN]
value = "<age-encrypted base64>"
```

# Operator runbook — Per-tenant credential master keys

Brief: `mission-credentials-per-tenant-master-key` (P0 fix from `eval-credentials-cross-tenant-isolation` / #2080 §3).

SKYCOT wraps every persisted credential (database-backed and on-disk)
with [age](https://age-encryption.org/) encryption. Each tenant gets a
**distinct** master key derived from a single workspace root key via
HKDF-SHA256, so a leak of one tenant's key cannot decrypt another
tenant's data.

This runbook covers:

- The two-tier key hierarchy and where each key lives
- How to provision a workspace root key
- The lazy-migration path for credentials encrypted under the legacy
  process-global key

## Key hierarchy

```
SKYCOT_WORKSPACE_ROOT_KEY_BASE64   (env var; macOS Keychain on roadmap)
  │
  └─ HKDF-SHA256("argo-cred-master-v1", root, tenant_id) ──► per-tenant MasterKey
       │
       └─ age-encrypts that tenant's credential values at rest
```

The workspace root key is **32 bytes**, base64-encoded for transport.
HKDF derives a 32-byte secret per tenant from that root, which becomes
the `age::x25519::Identity` used to wrap individual credential values.
Per-tenant derivation runs at most once per tenant per process boot —
results are held in a `DashMap` cache keyed by `TenantId`.

## Why this exists

The previous implementation (process-wide
`OnceLock<MasterKey>` at
`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:180-183`)
used **one master key for every tenant**, with a `MasterKey::generate()`
fallback that produced an ephemeral random key when no key was
configured.

Two bugs, both surfaced in
`eval-credentials-cross-tenant-isolation` (round-9, PR #2080) §3 P0:

1. **Data loss on restart.** The ephemeral fallback was different on
   every boot — credentials wrapped by yesterday's process were
   permanently unreadable today.
2. **Cross-tenant blast radius.** A leak of the single master key
   compromised every tenant's credentials at once.

The per-tenant scheme fixes both: there is no ephemeral fallback
(fail-closed), and any leaked per-tenant key only decrypts that one
tenant's data.

## Provisioning a workspace root key

```bash
# Generate a 32-byte secret and base64-encode it.
openssl rand -base64 32

# Add it to ~/.argo/env.local.
cat <<'EOF' >> ~/.argo/env.local
export SKYCOT_WORKSPACE_ROOT_KEY_BASE64='<paste output here>'
EOF

# Verify the value decodes to exactly 32 bytes (44 base64 characters).
echo "$SKYCOT_WORKSPACE_ROOT_KEY_BASE64" | base64 -d | wc -c   # → 32
```

Restart the gateway after adding the variable; the resolver reads it
once at first credential wrap and caches the derivation result.

## Back-compat with single-key deployments

If `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` is unset, the resolver
deterministically derives a workspace root from whatever
`ARGO_CREDENTIALS_MASTER_KEY` produces. Single-tenant dev
environments that only set the legacy variable keep working
unchanged.

For any production deployment with more than one tenant, set
`SKYCOT_WORKSPACE_ROOT_KEY_BASE64` explicitly. The back-compat path is
a *convenience*, not a security recommendation — the workspace root
is the operator-facing rotation unit.

## Lazy migration of legacy ciphertexts

Credentials wrapped before this brief landed are encrypted under the
legacy process-global key (logical `key_version = 1`). The decrypt
path supports lazy migration via
[`argo_credentials::decrypt_value_with_fallback`]:

1. Try the per-tenant key (`key_version = 2`). If it succeeds, return.
2. If the per-tenant key fails the AEAD check, retry with the legacy
   global key.
3. If the legacy key succeeds, return the plaintext **plus** a
   `KeyVersion::Legacy` marker so the caller knows to re-encrypt under
   the per-tenant key and write back.

The `argo_config::cred` read path consumes that marker at
`crates/argo-config/src/cred.rs:92` — when a value comes back tagged
`KeyVersion::Legacy`, the credential is re-wrapped with the per-tenant
key, the TOML section is rewritten in place, and a one-shot
`tracing::warn!` fires per `(tenant_id, key)` so the migration is
observable in the operator log.

No operator action is required — every credential migrates on first
read after the upgrade. The migration is idempotent: re-running it
against an already-migrated row is a no-op.

If both keys fail, the decrypt errors loudly with an
`ArgoError::Provider` naming both failures — this is the
"corrupt blob" or "wrong key" failure mode, not a silent-empty
return.

## Rotating the workspace root

Rotating the root invalidates every existing per-tenant derivation.
Until per-tenant rotation tooling lands (tracked separately), the
operator-facing recipe is:

1. Generate a new root, but keep the OLD root configured for one
   release cycle as the fallback.
2. New writes use the new root's per-tenant keys.
3. Reads transparently fall back to the old root via the
   `decrypt_value_with_fallback` path described above, re-encrypting
   on first hit.
4. Once every row has been re-read and re-encrypted (sweep with a
   read-only crawler if you don't want to wait for natural turnover),
   remove the old root.

This is the same shape as the legacy-to-per-tenant migration —
versioned salts (`argo-cred-master-v1`) leave headroom for a
future `-v2` derivation without colliding with on-disk ciphertexts
still validating under `-v1`.

## Verifying isolation

After rotating, you can confirm cross-tenant isolation via the unit
test pinned in
`crates/argo-credentials/src/master_key_tests.rs::derive_for_two_tenants_yields_distinct_keys`:

```bash
cargo test -p argo-credentials master_key_tests
```

The test bootstraps two distinct tenants, derives a master key for
each, and asserts (a) the two keys serialize to different bech32
strings and (b) decrypting tenant A's payload with tenant B's key
fails with an AEAD authentication error. The decrypt-cross-tenant
assertion is the load-bearing invariant the brief fixes.
