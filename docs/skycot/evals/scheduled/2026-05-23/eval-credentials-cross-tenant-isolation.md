# Eval — `argo-credentials` cross-tenant isolation

**Brief**: `eval-credentials-cross-tenant-isolation`
**Authored against**: `origin/main` at the worktree HEAD of the claim commit (`432aac842`).
**Scope**: read-only audit of `crates/argo-credentials/src/**` and every workspace caller (`argo-config`, `argo-cli`, `argo-channels`, all `argo-mcp-connector-*`, `argo-store::workspace_keys`).
**Severity scale**: P0 — ship blocker / data integrity / cross-tenant leak; P1 — substantive correctness gap; P2 — quality / completeness; P3 — minor.
**Companion brief**: `eval-argo-agent-runner-turn-lifecycle` (same shape, different surface).

---

## TL;DR

`argo-credentials` itself is a small, well-scoped age-wrap helper crate (623 LOC across 4 files, no `unwrap` outside `#[cfg(test)]`, master-key newtype properly redacts on `Debug`). The crate is **tenant-blind by construction** — there is no `TenantId` parameter on any public surface, no SQL, no per-tenant key derivation.

That is **structurally fine** in SKYCOT's documented single-tenant-per-process / single-workspace-per-cwd model, where workspace path = tenant boundary by definition. It becomes a **🚨 P0 cross-tenant leak surface** the moment the gateway process serves multiple tenants from a single cwd — which is the architecture the `workspace_provider_key` SQL table (PRIMARY KEY `(tenant_id, workspace_id, provider)`) and the gateway-route handlers in `argo-cli/src/handlers/gateway/*` are already shaped for.

Three P0 findings, two P1, two P2. None of the P0s are inside `argo-credentials`'s own source; they live at the seam between `argo-credentials` and the rest of the system. The crate itself is being used the way it was designed; the design's tenant model is the problem.

---

## §1 — Surface inventory

`crates/argo-credentials/` is one `[lib]` crate, 623 LOC total:

| File | LOC | Role |
|---|---:|---|
| `src/lib.rs` | 227 | TOML doc + read/write/migrate primitives |
| `src/age_wrap.rs` | 45 | Per-value age encrypt/decrypt helpers |
| `src/master_key.rs` | 146 | `MasterKey` newtype + env/file resolution |
| `src/lib_tests.rs` | 205 | Unit tests (lib-internal) |

### Public surface

From `crates/argo-credentials/src/lib.rs:39-52`:

- `pub fn read_encrypted(path: &Path, key: &MasterKey) -> ArgoResult<CredentialMap>` (`lib.rs:92`) — load and decrypt a TOML file.
- `pub fn write_encrypted(path: &Path, credentials: &CredentialMap, key: &MasterKey) -> ArgoResult<()>` (`lib.rs:117`) — encrypt-then-write.
- `pub fn migrate_plaintext_in_place(path: &Path, key: &MasterKey) -> ArgoResult<MigrationReport>` (`lib.rs:141`) — wrap any plaintext entries in-place.
- `pub fn encrypt_value(plaintext: &str, key: &MasterKey) -> ArgoResult<String>` (`age_wrap.rs:24`) — wrap one value, return base64.
- `pub fn decrypt_value(blob_b64: &str, key: &MasterKey) -> ArgoResult<String>` (`age_wrap.rs:37`) — inverse.
- `pub struct MasterKey` (`master_key.rs:34`) — newtype over `age::x25519::Identity`. `Debug` is redacted (`master_key.rs:109`).
- `pub const MASTER_KEY_ENV` = `"ARGO_CREDENTIALS_MASTER_KEY"` (`master_key.rs:24`).
- `pub const MASTER_KEY_FILE_REL` = `".skycot/.master_key"` (`master_key.rs:27`).
- `MasterKey::from_age_secret`, `MasterKey::generate`, `MasterKey::resolve(Option<&Path>)`, `MasterKey::as_age_secret` (`master_key.rs:44-98`).
- `pub fn dirs_home_dir() -> Option<PathBuf>` (`master_key.rs:119`).

### What is NOT present

- **No `TenantId` parameter anywhere** in the crate. `grep -rn 'TenantId\|tenant_id' crates/argo-credentials/` returns zero hits.
- **No SQL.** Storage is TOML file (`<workspace>/.argo/credentials.toml`).
- **No `Cargo.toml` dep on `argo-types`'s `TenantId`** — the crate only uses `argo_types::{ArgoError, ArgoResult}`.
- **No OAuth refresh primitive** in this crate. OAuth lives one layer up, per-connector.
- **No rotation or revocation primitive.** The CLI subcommand `cred clear` lives in `argo-config::cred::clear`; nothing in this crate knows about deletion.
- **No `Secret<String>` newtype on the decrypt boundary.** `decrypt_value` returns a raw `String`.

### Storage primitives

The on-disk shape (from `lib.rs:66-79`) is:

```toml
[credentials.SLACK_BOT_TOKEN]
value = "<base64-encoded-age-blob>"
```

Plus a legacy back-compat path for bare strings (`Plain(String)` variant of the `EncryptedEntry` enum). `migrate_plaintext_in_place` exists to walk a mixed file and rewrap every `Plain` entry. The file is written at mode `0600` on Unix (`lib.rs:191-199`).

### Resolver primitives

- `MasterKey::resolve(home_override)` (`master_key.rs:68`) — env first (`ARGO_CREDENTIALS_MASTER_KEY`), then `<home>/.skycot/.master_key`, else `ArgoError::Schema`. No silent default.
- The file is read with a `warn_loose_perms` check (`master_key.rs:126`); a `0644` master-key file emits a stderr warning but **does not refuse to load** — proceeding with a world-readable key. P3, see §7.

---

## §2 — SQL tenancy audit

**No SQL inside `argo-credentials`.** This section therefore covers the *related* SQL surfaces that store credentials downstream:

### `workspace_provider_key` table (slot 126)

Defined in `crates/argo-store/migrations/126__workspace_onboarding_keys.sql:14-28`:

```sql
CREATE TABLE IF NOT EXISTS workspace_provider_key (
    tenant_id        BLOB NOT NULL,
    workspace_id     BLOB NOT NULL,
    provider         TEXT NOT NULL,
    masked_key       TEXT NOT NULL,
    ciphertext_b64   TEXT NOT NULL,
    test_cost_micros INTEGER NOT NULL DEFAULT 0,
    test_tokens      INTEGER NOT NULL DEFAULT 0,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, provider)
);
```

PRIMARY KEY is `(tenant_id, workspace_id, provider)`. There is also `idx_workspace_provider_key_tenant`. Shape is correct.

Queries in `crates/argo-store/src/workspace_keys.rs`:

- `upsert_provider_key` (`workspace_keys.rs:98`) — binds `tenant_id` + `workspace_id` + `provider`. ✅ tenant-scoped.
- `get_provider_key` (`workspace_keys.rs:136`) — same bind. ✅ tenant-scoped. **Note**: this function has zero non-test callers (see §4 for impact).
- `record_vertical_install` (`workspace_keys.rs:170`) — same bind. ✅ tenant-scoped.
- `list_installed_verticals` (`workspace_keys.rs:205`) — same bind. ✅ tenant-scoped.

An existing `cross_tenant_isolation` test at `workspace_keys.rs:296` verifies tenant-A's row is invisible to tenant-B's `get_provider_key`. ✅ covered.

**§2 verdict**: SQL layer for stored provider keys is correctly tenant-scoped (write path) and tested (the one path that has a read primitive). The gap is upstream (handler + key derivation) and downstream (read path is dead code), not in the SQL.

---

## §3 — Encryption-at-rest verification

### Per-value wrapping

Every credential value is independently wrapped via `age::encrypt(&recipient, plaintext.as_bytes())` (`age_wrap.rs:24-29`) and base64-encoded for TOML safety. Wrapping uses an `x25519::Identity` — a 32-byte random secret with the matching public recipient derived on demand. This is solid; per-value wrapping prevents partial-file decrypt attacks and keeps TOML escaping orthogonal to ciphertext.

### Key derivation — the critical question

**There is no per-tenant key derivation.** The crate exposes one `MasterKey` type per process. `MasterKey::resolve` reads a single env var or a single fixed file path. This is **structurally workspace-scoped**, not tenant-scoped:

- One workspace dir → one `~/.skycot/.master_key` resolves the same key for every credential in that workspace.
- One process → one resolved `MasterKey` (see process-global static below).

In the gateway / multi-tenant deployment shape, that means **all tenants in the same process share one wrapping key**. The ciphertext column in `workspace_provider_key` is encrypted with the same key regardless of `tenant_id`.

### 🚨 P0 — Process-global ephemeral fallback (data loss + cross-tenant leak)

**Finding**: `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:180-183`:

```rust
fn process_master_key() -> &'static MasterKey {
    static KEY: OnceLock<MasterKey> = OnceLock::new();
    KEY.get_or_init(|| MasterKey::resolve(None).unwrap_or_else(|_| MasterKey::generate()))
}
```

Two compounding problems:

1. **`MasterKey::generate()` on resolution failure** produces a **random ephemeral key** that vanishes when the process restarts. Every provider key encrypted under that key becomes undecryptable forever. The handler doc-comment (`routes_onboarding.rs:21-23`) claims this is "intentional so the row never lands as plaintext", but the cost is silent data loss on restart — operators get a clean ciphertext column at write time and a broken `get_provider_key` flow on every subsequent boot.

2. **Process-global `OnceLock<MasterKey>`** means **one wrapping key for every tenant in the process**. If the gateway runs as a single multi-tenant process (the production architecture per the `workspace_provider_key` PK shape), an attacker who exfiltrates the master key (env, master-key file, or a process-memory dump) decrypts every tenant's provider keys at once. There is no "fail this tenant alone" cryptographic story.

**Severity**: 🚨 P0 — data loss on restart + cross-tenant blast radius on key compromise.

**Recommendation (fix brief)**: `mission-credentials-per-tenant-master-key` — derive a per-tenant subkey from a workspace HKDF context, store one wrapped tenant key per row, decrypt with the tenant-scoped derived key. Refuse to start the process if `MasterKey::resolve` fails — never silently fall back to `MasterKey::generate()`. Add a `MasterKey::resolve_or_fail_start()` constructor that handles this contract explicitly.

### 🟡 P3 — Master-key file `0644` loads with a warning

`master_key.rs:126-145` — `warn_loose_perms` emits a stderr line on `0644` but proceeds. Best-in-class is fail-closed on world-readable secrets. The same pattern exists for the credentials file itself (`lib.rs:203-227`).

**Severity**: P3 — quality of fail-closed posture.

**Recommendation**: `mission-credentials-fail-closed-on-loose-perms` — refuse to load when `mode & 0o077 != 0` unless an explicit `ARGO_CREDENTIALS_ALLOW_LOOSE_PERMS=1` env opt-in is set.

---

## §4 — API surface audit (caller walk)

The crate has 4 caller files in 2 caller crates, plus a downstream channel/connector layer that uses `argo-credentials` *transitively* through `argo-config::cred`.

### Direct callers (`grep -rln argo_credentials`)

1. `crates/argo-config/src/cred.rs` — wraps `argo-credentials` for the workspace-file backend (env > keyring > file resolution chain).
2. `crates/argo-cli/src/handlers/credentials_encrypt_all.rs` — `argo cred encrypt-all` CLI subcommand.
3. `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` — gateway onboarding handler, calls `encrypt_value` for provider API keys (see §3 P0).
4. `crates/argo-config/src/cred_encryption_tests.rs` — unit tests.

Every direct caller passes a `&MasterKey` correctly; none of them attempt to load by id without a key. ✅ direct API contract honored.

### Indirect callers (the hot path)

`argo-config::cred::get(name)` (`cred.rs:112`) is the de-facto runtime credential-read API:

```rust
pub fn get(key: &str) -> Result<String, VarError> {
    let cwd = std::env::current_dir().map_err(|_| VarError::NotPresent)?;
    get_from_workspace(&cwd, key)
}
```

It takes only the credential *key name*. Tenant scoping is implicit in `current_dir()`. There are **40+ callsites across `crates/argo-channels/` and `crates/argo-mcp-connector-*`**:

| Crate | Credential keys |
|---|---|
| `argo-channels/src/slack.rs` | `SLACK_APPROVALS_BOT_TOKEN` |
| `argo-channels/src/teams.rs` | `cfg.client_secret_env` (e.g. Azure AD client secret) |
| `argo-channels/src/imessage.rs` | `BLUE_BUBBLES_WEBHOOK_SECRET`, env-named token |
| `argo-channels/src/discord.rs` | env-named bot token |
| `argo-channels/src/matrix.rs` | `cfg.access_token_env` |
| `argo-channels/src/whatsapp.rs` | env-named |
| `argo-channels/src/sms.rs` | Twilio credentials |
| `argo-channels/src/voice.rs` | voice provider creds (primary + fallback) |
| `argo-channels/src/push.rs` | `EXPO_ACCESS_TOKEN` |
| `argo-mcp-connector-airtable/src/auth.rs` | `AIRTABLE_PAT_ENV` |
| `argo-mcp-connector-asana/src/auth.rs` | `ASANA_PAT_ENV` |
| `argo-mcp-connector-calendly/src/auth.rs` | `CALENDLY_API_KEY_ENV` |
| `argo-mcp-connector-clickup/src/auth.rs` | `CLICKUP_TOKEN_ENV` |
| `argo-mcp-connector-discord/src/auth.rs` | `DISCORD_BOT_TOKEN_ENV` |
| `argo-mcp-connector-figma/src/auth.rs` | `FIGMA_TOKEN_ENV` |
| `argo-mcp-connector-fireflies/src/auth.rs` | `FIREFLIES_API_KEY_ENV` |
| `argo-mcp-connector-gcal/src/client.rs` | `GCAL_ACCESS_TOKEN`, OAuth refresh flow |
| `argo-mcp-connector-github-issues/src/auth.rs` | `GITHUB_TOKEN_ENV` |
| `argo-mcp-connector-gmail/src/auth.rs` | `ARGO_GMAIL_{CLIENT_ID,CLIENT_SECRET,REFRESH_TOKEN}` |
| `argo-mcp-connector-google-meet/src/auth.rs` | `GOOGLE_OAUTH_TOKEN_ENV` |
| `argo-mcp-connector-hubspot/src/client.rs` | `HUBSPOT_PRIVATE_APP_TOKEN` |
| `argo-mcp-connector-imessage/src/auth.rs` | env-named |
| `argo-mcp-connector-jira/src/auth.rs` | env-named |
| `argo-mcp-connector-linear/src/auth.rs` | `LINEAR_API_KEY_ENV` |
| `argo-mcp-connector-notion/src/client.rs` | `NOTION_INTEGRATION_TOKEN` (+ `NOTION_TEST_TOKEN`) |
| `argo-mcp-connector-mailchimp/src/auth.rs` | env-named |
| `argo-mcp-connector-monday/src/auth.rs` | `MONDAY_API_TOKEN_ENV` |
| `argo-mcp-connector-postmark/src/auth.rs` | `POSTMARK_SERVER_TOKEN_ENV` |
| `argo-mcp-connector-pagerduty/src/auth.rs` | `PAGERDUTY_API_KEY_ENV`, `PAGERDUTY_FROM_EMAIL_ENV` |
| `argo-mcp-connector-salesforce/src/auth.rs` | OAuth client-creds refresh |

### 🚨 P0 — `cred::get(name)` is tenant-blind, callers do not pass tenant

**Finding**: Every callsite above uses the **no-`tenant_id`** form. The dispatch goes:

1. `argo_config::cred::get("SLACK_APPROVALS_BOT_TOKEN")`
2. → `get_from_workspace(&std::env::current_dir(), "SLACK_APPROVALS_BOT_TOKEN")`
3. → `load_from_file(&workspace, "SLACK_APPROVALS_BOT_TOKEN")`
4. → reads `<workspace>/.argo/credentials.toml`

In the single-tenant-per-cwd model (operator runs `argo` from their workspace dir), this is fine — the workspace = tenant.

In the **gateway / multi-tenant process model** (the architecture documented by `workspace_provider_key`'s `(tenant_id, workspace_id, provider)` PK), every tenant's worker turn executes inside the gateway process. The gateway does NOT `set_current_dir` per tenant (a `grep -rn set_current_dir crates/` shows only test fixtures and one non-gateway handler use). All channel sends and all connector tool calls therefore read the *same* `cwd/.argo/credentials.toml` regardless of which tenant the calling worker belongs to.

The cross-tenant leak vector:

- Operator A configures `SLACK_APPROVALS_BOT_TOKEN=xoxb-A` in the gateway's cwd credentials file.
- Operator B is provisioned later, in the same gateway.
- Tenant B's worker hits the Slack channel → reads `cwd/.argo/credentials.toml` → gets tenant A's bot token → posts approval requests into tenant A's Slack workspace.

**Severity**: 🚨 P0 — by-construction cross-tenant credential leak in any multi-tenant gateway deployment. This is the exact pattern auditor T21-39 §4 P1 #2 flagged.

**Recommendation (fix brief)**: `mission-cred-get-tenant-parameter` — change the signature of the credential-read API to require a `TenantId`:

```rust
pub fn get(tenant: TenantId, key: &str) -> ArgoResult<String>;
```

The implementation route: read from `workspace_provider_key` (or a sister `workspace_credential` table for non-LLM-provider keys) scoped to `(tenant, key)`, decrypt with a per-tenant master key (see §3 P0 fix), fall back to process-env only when explicitly allowed by tenant policy. Audit every callsite in the channels + connectors tables above; this is a workspace-wide refactor.

### 🟡 P2 — `get_provider_key` is wired in storage but has no production reader

`crates/argo-store/src/workspace_keys.rs:136` defines `get_provider_key`, but `grep -rn 'get_provider_key\b' crates/` shows only doc references and tests. The handler at `routes_onboarding.rs:100` writes `workspace_provider_key` rows; nothing reads them in the runtime path. Worker turns calling Anthropic/OpenAI must therefore be getting the provider API key from somewhere else (process env, most likely) — the per-tenant provider key in the DB is decorative until a reader wires in.

**Severity**: 🟡 P2 — feature-completeness gap, not a leak. But it means the *one* tenant-scoped credential path the system has shipped is dead code from the consumer end.

**Recommendation (fix brief)**: `mission-provider-key-runtime-read-wire` — add the read path from `workspace_provider_key` into the provider-router credential resolution. Combine with the §3 P0 per-tenant-key fix.

---

## §5 — OAuth refresh flow

There is no centralized OAuth refresh primitive in `argo-credentials` itself. Each connector implements its own refresh in `crates/argo-mcp-connector-*/src/auth.rs` (or `client.rs`). Sampled the two most-developed flows:

### Gmail (`argo-mcp-connector-gmail/src/auth.rs`)

- Refresh inputs: `ARGO_GMAIL_CLIENT_ID`, `ARGO_GMAIL_CLIENT_SECRET`, `ARGO_GMAIL_REFRESH_TOKEN` — all read via `argo_config::cred::get(name)`.
- Refresh trigger: `access_token_from_env()` checks the disk cache; if empty/expired, calls `refresh_and_cache()`.
- Refresh write-back: `<cwd>/.argo/connectors/gmail/access_token.json` (mode `0600` on Unix). Cache path uses `std::env::current_dir()` (`auth.rs:113`).
- No tenant binding on the cache path.
- No audit emit on refresh.

### gcal (`argo-mcp-connector-gcal/src/client.rs:289-319`)

Same shape — env-resolved refresh token, file-cache access token, no tenant in path.

### 🚨 P0 — OAuth access-token cache is process-cwd shared

**Finding**: `gmail/src/auth.rs:113` (and the gcal analog) writes the refreshed access token to `<cwd>/.argo/connectors/gmail/access_token.json`. In the multi-tenant gateway process, all tenants share that cache.

Vector identical to §4 P0:

- Tenant A's Gmail-using worker fires → refresh runs → access token written to shared cache.
- Tenant B's Gmail-using worker fires (with potentially their own different refresh token configured) → reads the shared cache → gets tenant A's access token → reads from tenant A's mailbox.

If both tenants happen to have the same `ARGO_GMAIL_REFRESH_TOKEN` source (single global env), all access tokens point at the same mailbox anyway — but that's the single-tenant fallback case. The dangerous case is when the gateway is supposed to be running per-tenant credentials (which is the future architecture per §3 + §4 fixes); the disk cache is then the surviving leak surface even after the env-var problem is solved.

**Severity**: 🚨 P0 — cross-tenant access-token leak via shared filesystem cache.

**Recommendation (fix brief)**: `mission-oauth-cache-per-tenant-scope` — change the cache path to `<state-root>/connectors/<connector>/<tenant_id>/access_token.json` (or similar). Caller must thread `TenantId` to `access_token_from_env()`; default `cwd` form is for single-tenant ops only and must be removed once `cred::get` itself takes a `TenantId` (§4 P0 fix).

### 🟡 P1 — Refresh is not atomic with audit emission

No connector's refresh flow calls into `argo-audit` to emit an audit row when an OAuth refresh happens. From a forensic standpoint that means: if tenant A's refresh token is exfiltrated and used by an attacker to mint new access tokens, there is no audit trail of the refresh events. The token-cache file is the only record, and it is mutable.

**Severity**: 🟡 P1 — observability gap, not a direct leak. Compliance-relevant (SOC 2 / GDPR audit-log expectations).

**Recommendation (fix brief)**: `mission-oauth-refresh-audit-emit` — emit `AuditKind::CredentialOAuthRefresh` rows from every connector refresh, scoped to the tenant whose worker triggered the refresh. Atomic with the cache write or roll back the cache.

---

## §6 — Rotation + revocation

### Rotation

- **Operator-action only.** No background rotator.
- `argo cred set <KEY> <VALUE>` writes a new value to `<workspace>/.argo/credentials.toml` via `argo-config::cred::set` (`cred.rs:236`).
- `argo cred encrypt-all` migrates the file from plaintext to wrapped form (`credentials_encrypt_all.rs`).
- The OS keyring backend (`argo-config::cred_keyring`) supports rotation via `keyring_set`, but rotation across both backends is not coordinated.

### Revocation

- `argo cred clear <KEY>` removes the row (`cred.rs:247`).
- `argo cred clear-all` removes everything (`cred.rs:259`).
- No keyring-aware "revoke everywhere" command — `clear` only touches the file backend.

### Propagation to in-flight worker turns

- **None.** Workers re-read on each turn boot via `argo_config::cred::get(name)`; there is no in-memory cache to invalidate, but there is also no notification primitive saying "tenant A's Slack token was revoked at T, terminate in-flight Slack sends for tenant A."
- In-flight HTTP requests carrying the soon-to-be-revoked token will complete normally and may post into the (now untrusted) destination.
- The OAuth access-token disk cache (§5) is *not* invalidated when the underlying refresh token is rotated.

### 🟡 P2 — No revocation propagation contract

**Severity**: 🟡 P2 — quality / completeness. Hard to exploit, but matters for compliance and breach response.

**Recommendation (fix brief)**: `mission-credential-revoke-propagation` — emit `AuditKind::CredentialRevoked` on every `cred clear` / `keyring_delete`; add a tenant-scoped "credential generation counter" that workers re-check before consuming a cached token. Cache invalidation is hard but achievable inside the gateway process.

---

## §7 — Decryption boundary

### Where decrypt happens

- `argo_credentials::decrypt_value(blob, &MasterKey) -> ArgoResult<String>` (`age_wrap.rs:37`) is the only decrypt primitive.
- `argo_config::cred::flatten_document` (`cred.rs:208-230`) calls it during `cred::load`; the decrypted plaintext lands in a `BTreeMap<String, String>` returned to the caller.
- `cred::get_with_backend` returns a raw `String` (`cred.rs:143-167`).

### The shape of the plaintext post-decrypt

- Plaintext leaves the crate as `String`, not `Secret<String>` (or any `secrecy`-crate newtype).
- The `MasterKey` newtype itself is well-protected — `Debug` is redacted (`master_key.rs:109`), and the inner age `Identity` keeps the secret in a `SecretString` (zeroized on drop).
- The decrypted *credential value* enjoys none of that protection. A panic message holding a captured credential, a `tracing::debug!("got token: {token}")` line, or a future `serde_json::to_string(&CredentialMap)` accidentally landing in a log will leak.

### 🟡 P1 — Plaintext credentials are raw `String`, not `Secret<String>`

**Severity**: 🟡 P1 — exfiltration surface via log lines, panic messages, and debug formatters. Real-world incidents in similar shape happen routinely — they don't take a clever exploit, just one debug-format `{:?}` on a `BTreeMap<String, String>` containing a credential.

**Recommendation (fix brief)**: `mission-credentials-secret-newtype-at-decrypt` — change `decrypt_value` (and downstream consumers) to return `secrecy::SecretString`. `argo_config::cred::get` should also return `SecretString`. Callers (channels, connectors) opt into `ExposeSecret` at the exact HTTP-body / DB-bind point, not before. This is a large refactor (40+ callsites) but the only structural fix.

---

## Summary table

| # | Severity | Section | Finding | Fix brief slug |
|---|:---:|:---:|---|---|
| 1 | 🚨 P0 | §3 | Process-global `OnceLock<MasterKey>` with ephemeral `MasterKey::generate()` fallback at `routes_onboarding_helpers.rs:180-183`. Data loss on restart + cross-tenant blast radius on key compromise. | `mission-credentials-per-tenant-master-key` |
| 2 | 🚨 P0 | §4 | `argo_config::cred::get(name)` is tenant-blind. 40+ connector + channel callsites read the gateway's shared `cwd/.argo/credentials.toml`. By-construction cross-tenant leak in multi-tenant gateway. | `mission-cred-get-tenant-parameter` |
| 3 | 🚨 P0 | §5 | OAuth access-token disk caches at `<cwd>/.argo/connectors/<name>/access_token.json` are process-cwd shared (gmail + gcal confirmed; pattern likely repeats elsewhere). | `mission-oauth-cache-per-tenant-scope` |
| 4 | 🟡 P1 | §5 | OAuth refresh emits no audit row. No forensic trail of refresh events. | `mission-oauth-refresh-audit-emit` |
| 5 | 🟡 P1 | §7 | `decrypt_value` returns raw `String`; plaintext flows out of the crate without a `Secret<String>` newtype. Log / panic / debug-format exfil surface. | `mission-credentials-secret-newtype-at-decrypt` |
| 6 | 🟡 P2 | §6 | No revocation propagation to in-flight worker turns. `cred clear` removes the row but emits no signal. | `mission-credential-revoke-propagation` |
| 7 | 🟡 P2 | §4 | `get_provider_key` is dead code from the read side — `workspace_provider_key` rows are written by the onboarding handler but never read by the runtime. | `mission-provider-key-runtime-read-wire` |
| 8 | 🟢 P3 | §3 | Master-key file `0644` loads with a warning instead of failing closed. | `mission-credentials-fail-closed-on-loose-perms` |

---

## 🚨 Escalation summary

Three P0s, all at the seam between `argo-credentials` and the rest of the system, not inside the crate's own source. They share a root cause: **the credential-resolution API was designed for the single-tenant-per-cwd model and is being deployed inside a multi-tenant gateway process**. The SQL layer (`workspace_provider_key`) has the right shape; the runtime read path has not caught up. Until findings 1 + 2 + 3 are fixed, **the multi-tenant gateway is structurally a credential-sharing process** for every channel, every MCP connector, and every OAuth refresh that runs inside it.

The single-tenant CLI mode (operator runs `argo` in their workspace dir) is unaffected — that is the model `argo-credentials` was actually designed for, and within that scope the crate is clean.

---

## Negative-coverage tests

Two `#[ignore]`'d FINDINGS-ONLY tests added at `crates/argo-credentials/tests/eval_findings_cross_tenant.rs`:

1. `decrypt_returns_raw_string_not_secret_newtype` — codifies §7 P1. Removed once `decrypt_value` returns `SecretString`.
2. `master_key_resolve_does_not_take_tenant_id` — codifies §3 P0 + §4 P0. Removed once `MasterKey::resolve_for_tenant(TenantId)` (or equivalent per-tenant key derivation) exists.

Both are pure compile-time / API-shape assertions inside `argo-credentials`'s own dev-deps; they are `#[ignore]`'d so they don't fail the suite, but they will compile-fail (red) the moment a fix-brief lands the API change — converting them into automatic verification gates.

A third test — "cross-process two-tenant credential leak" — would be the right shape per the brief's "anti-theater" guidance, but it requires a runnable gateway harness and would touch crates outside this brief's may-touch list. Logged as a follow-up: `eval-credentials-multi-tenant-gateway-leak-harness`.

---

## Authoring trace

- Read 4 source files (623 LOC) in `argo-credentials/src/`.
- Walked 4 caller crates: `argo-config`, `argo-cli`, `argo-channels`, `argo-mcp-connector-*` (sampled ~10 of 27).
- Read 1 SQL migration (`migrations/126__workspace_onboarding_keys.sql`) + 1 storage module (`argo-store::workspace_keys`, 350 LOC).
- Tooling: `rg`, `find`, `wc -l`. No tests run — read-only audit.
- Token budget: well under 100K (read-only single-crate audit, 27-connector sample, 1 SQL slot).

Auditor citation per brief: T21-39 §4 P1 #2 — "`argo-credentials` stores OAuth + API keys; cross-tenant leak here breaks every customer's third-party tokens. Pattern from #2068 6-handler existence-leak + #2051 audit-export tenant-scope: cross-tenant work has been shipping P1-level value per audit, and credentials is unaudited so the surface is unknown." Evaluator round-8 manifest row reproduced verbatim at `mission-vs-code-gap-manifest.md:110`.
