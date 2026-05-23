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
