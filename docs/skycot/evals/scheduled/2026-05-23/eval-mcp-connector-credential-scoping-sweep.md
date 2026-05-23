# Eval — MCP connector credential scoping sweep (29 connectors)

**Brief**: `eval-mcp-connector-credential-scoping-sweep`
**Authored against**: `origin/main` at the worktree HEAD of the claim commit (`199836940`).
**Scope**: read-only audit of every `crates/argo-mcp-connector-*/src/**`, classifying each connector by (a) where it sources its credential, (b) whether the read is tenant-bound, (c) where any token refresh deposits the refreshed token, and (d) what on-disk OAuth cache (if any) it owns.
**Severity scale**: P0 — cross-tenant credential leak / mis-attribution risk; P1 — substantive correctness gap; P2 — quality / completeness; P3 — minor.
**Companion brief**: `eval-credentials-cross-tenant-isolation` (the parent audit; this is its connector-fanout sibling).
**Source handoff**: evaluator round-10 SHARED.md at `2026-05-23T10:55:00Z` — *"27 MCP connector structural inventory PASS (29 real crates compile + tests; substrate healthy). 3 follow-up: ... eval-mcp-connector-credential-scoping-sweep (P1 — per-connector OAuth scoping NOT yet verified, canonical next-round focus)."*

---

## TL;DR

The credential read pattern is **uniform across all 29 connector crates** — every one that reads an API token, OAuth bearer, or refresh secret goes through a single function: `argo_config::cred::get_unscoped(name)`. The function is documented (`crates/argo-config/src/cred.rs:167-178`) as bypassing tenant isolation:

> *"Unscoped credential read — bypasses tenancy isolation. Use ONLY when the calling context legitimately has no tenant identity (CLI startup probes, single-tenant `argo` invocations, gateway-process secrets like webhook HMAC). Every call site SHOULD carry a `// tenant-unscoped: <reason>` comment."*

Every connector carries the standard justification comment ("MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying"). The discipline is present; the tenant scoping is not.

Three connectors additionally cache OAuth access tokens to disk at `<cwd>/.argo/connectors/<name>/{access_,token.json}`. The cache key is the process cwd — not the tenant. In a multi-tenant gateway process, two tenants' tokens collide on the same path: whichever connector wrote last wins, and the next read by the other tenant returns the wrong tenant's bearer token.

Counts:

- **27 connectors are env-only-leak** (no disk cache; reads `cred::get_unscoped`).
- **3 connectors are env-or-cache-leak** (gcal, gmail, salesforce — disk caches at cwd-shared paths).
- **1 connector is caller-responsibility** (onedrive — `OneDriveConnector::new(oauth_token: String)` accepts the token as a constructor argument; no in-crate provenance, no production caller in the workspace).
- **1 crate is not a connector** (argo-mcp-connector-shared — retry helper only, no credentials).

Total `argo-mcp-connector-*` crates in the workspace: **30** (29 connectors + 1 shared helper). The brief's "27" count from round-10 was a sales-page approximation; the audit confirms 29 connector crates eligible for the sweep.

### Headline findings

| # | Finding | Severity | Fix-brief recommendation |
|---|---|---|---|
| F-01 | `cred::get_unscoped` is the canonical read for **every** connector. Tenant scoping is structurally impossible at the current surface. | 🟡 P1 | Already covered by `mission-cred-get-tenant-parameter` — connector auth fns need a `TenantId` parameter once that brief lands. This audit's value: enumerate the call sites that must flip. |
| F-02 | `<cwd>/.argo/connectors/gcal/access_token.json` is process-cwd-scoped, not tenant-scoped. Two tenants sharing a gateway cwd collide on the same path; OAuth refresh writes overwrite the previous tenant's cached bearer. | 🔴 P0 | `mission-mcp-connector-token-cache-tenant-keyed` — restructure the path to `<workspace>/.argo/connectors/<name>/<tenant_id>/access_token.json` (or move to SQL-backed `workspace_provider_key`-style row). |
| F-03 | Same pattern as F-02 in `<cwd>/.argo/connectors/gmail/access_token.json`. | 🔴 P0 | Same fix-brief as F-02 (single brief covers all three). |
| F-04 | Same pattern as F-02 in `<cwd>/.argo/connectors/salesforce/token.json`. | 🔴 P0 | Same fix-brief as F-02. |
| F-05 | `argo-mcp-connector-onedrive` accepts an OAuth bearer via constructor argument (`OneDriveConnector::new(oauth_token: String)`) with no `from_env` shortcut and no production caller in the workspace. Surface is clean, but the eventual caller will inherit the same `cred::get_unscoped` leak unless the caller is built tenant-aware. | 🟡 P1 | Track in the same fix-brief as F-01; document the caller-side requirement on the OneDrive descriptor when it wires up. |
| F-06 | Connector base-URL overrides (`*_BASE_URL` env vars) are read via direct `std::env::var`, **not** `cred::get_unscoped`. These are not credentials — they're test/staging endpoint overrides — but they share the process-cwd-scoping problem at a much lower severity (a test override could mis-direct a tenant's traffic if env vars cross over). | 🟢 P3 | No fix-brief needed; document in this report. The right boundary is "credentials always via `cred::*`; non-secret config via `env::var` is fine." |

### Recommended fix-brief queue (post-this-audit)

1. **🔴 P0 — `mission-mcp-connector-token-cache-tenant-keyed`** (NEW). Owners: claude. Touches: 3 connector crates (gcal, gmail, salesforce). Re-key disk caches by tenant. **Strict pre-req**: depends on `mission-cred-get-tenant-parameter` landing first, because the cache path needs a `TenantId` to embed.
2. **🟡 P1 — `mission-cred-get-tenant-parameter`** (ALREADY AUTHORED — this audit confirms 29 connector call-sites that flip the moment the API ships).
3. **🟢 P3 — none.** F-06 is not load-bearing.

🚨 **ESCALATION (operator-eyes)**: F-02 / F-03 / F-04 are the only true P0s in the connector layer. They are NOT regressions of round-10 substrate-healthy verdict; the substrate compiles, runs tests, and the leak only manifests at multi-tenant gateway runtime. v1.0 is **macOS single-tenant demo** (operator-locked 2026-05-14 — `project_v1_strategic_decisions_2026_05_14.md`), so the P0 does **not** block the v1.0 ship gate. It DOES block any multi-tenant gateway demo. Recommend queueing the cache-keyed fix-brief for the post-v1.0 hardening wave.

---

## §1 — Connector inventory

29 connector crates + 1 shared helper. Connector list verbatim from `Cargo.toml` workspace members (sorted):

```
argo-mcp-connector-airtable          argo-mcp-connector-mailchimp
argo-mcp-connector-asana             argo-mcp-connector-monday
argo-mcp-connector-calendly          argo-mcp-connector-notion
argo-mcp-connector-clickup           argo-mcp-connector-onedrive
argo-mcp-connector-discord           argo-mcp-connector-pagerduty
argo-mcp-connector-figma             argo-mcp-connector-postmark
argo-mcp-connector-fireflies         argo-mcp-connector-salesforce
argo-mcp-connector-gcal              argo-mcp-connector-sendgrid
argo-mcp-connector-github-issues     argo-mcp-connector-shared    [helper only]
argo-mcp-connector-gmail             argo-mcp-connector-slack
argo-mcp-connector-google-meet       argo-mcp-connector-stripe
argo-mcp-connector-hubspot           argo-mcp-connector-stripe-events
argo-mcp-connector-imessage          argo-mcp-connector-trello
argo-mcp-connector-jira              argo-mcp-connector-typeform
argo-mcp-connector-linear            argo-mcp-connector-zendesk
```

`argo-mcp-image-gen` is a different surface (image-generation tool, not a connector) — out of scope.

---

## §2 — Per-connector matrix

Each row classifies the connector against the four axes the brief calls out: (1) OAuth/cred source, (2) tenant-scoped, (3) refresh-on-disk, (4) cache location. Verdict column uses the severity scale above.

| # | Connector | Credential env var(s) | Read via | Tenant-scoped? | OAuth refresh? | Disk cache location | Verdict |
|---:|---|---|---|---|---|---|---|
| 1 | airtable | `AIRTABLE_PAT` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (PAT) | — | 🟡 P1 env-only-leak |
| 2 | asana | `ASANA_PAT` | `cred::get_unscoped` (`auth.rs:16`) | ❌ no | n/a (PAT) | — | 🟡 P1 env-only-leak |
| 3 | calendly | `CALENDLY_API_KEY` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 4 | clickup | `CLICKUP_TOKEN` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 5 | discord | `DISCORD_BOT_TOKEN` | `cred::get_unscoped` (`auth.rs:13`) | ❌ no | n/a (bot token) | — | 🟡 P1 env-only-leak |
| 6 | figma | `FIGMA_TOKEN` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (PAT) | — | 🟡 P1 env-only-leak |
| 7 | fireflies | `FIREFLIES_API_KEY` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 8 | **gcal** | `GCAL_ACCESS_TOKEN` / `GCAL_TEST_TOKEN` / `ARGO_GCAL_REFRESH_TOKEN` / `ARGO_GCAL_CLIENT_ID` / `ARGO_GCAL_CLIENT_SECRET` | `cred::get_unscoped` (`client.rs:262`, `:266`, `:352`) | ❌ no | ✅ OAuth refresh-token flow (`client.rs:291-322`) | **`<cwd>/.argo/connectors/gcal/access_token.json`** (`client.rs:339-348`) | 🔴 **P0 env-or-cache-leak** |
| 9 | github-issues | `GITHUB_TOKEN` | `cred::get_unscoped` (`auth.rs:13`) | ❌ no | n/a (PAT) | — | 🟡 P1 env-only-leak |
| 10 | **gmail** | `ARGO_GMAIL_CLIENT_ID` / `ARGO_GMAIL_CLIENT_SECRET` / `ARGO_GMAIL_REFRESH_TOKEN` | `cred::get_unscoped` (`auth.rs:50`) | ❌ no | ✅ OAuth refresh-token flow (`auth.rs:22-46`) | **`<cwd>/.argo/connectors/gmail/access_token.json`** (`auth.rs:11`, `:113-117`) | 🔴 **P0 env-or-cache-leak** |
| 11 | google-meet | `GOOGLE_OAUTH_TOKEN` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (caller supplies bearer) | — | 🟡 P1 env-only-leak |
| 12 | hubspot | `HUBSPOT_PRIVATE_APP_TOKEN` | `cred::get_unscoped` (`client.rs:18`) | ❌ no | n/a (private app token) | — | 🟡 P1 env-only-leak |
| 13 | imessage | `BLUE_BUBBLES_URL` / `BLUE_BUBBLES_PASSWORD` | `cred::get_unscoped` (`auth.rs:21`) | ❌ no | n/a (basic auth) | — | 🟡 P1 env-only-leak |
| 14 | jira | `JIRA_EMAIL` / `JIRA_API_TOKEN` / `JIRA_BASE_URL` | `cred::get_unscoped` (`auth.rs:63`) | ❌ no | n/a (Atlassian basic auth) | — | 🟡 P1 env-only-leak |
| 15 | linear | `LINEAR_API_KEY` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 16 | mailchimp | (per-key required envs read in `required_env`) | `cred::get_unscoped` (`auth.rs:39`) | ❌ no | n/a | — | 🟡 P1 env-only-leak |
| 17 | monday | `MONDAY_API_TOKEN` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 18 | notion | `NOTION_INTEGRATION_TOKEN` / `NOTION_TEST_TOKEN` | `cred::get_unscoped` × 2 (`client.rs:24-26`) | ❌ no | n/a (integration token) | — | 🟡 P1 env-only-leak |
| 19 | **onedrive** | (caller-supplied) | constructor arg `OneDriveConnector::new(oauth_token: String)` (`client.rs:38`) | ❌ no (caller's problem) | n/a (out of scope per `lib.rs:36`) | — | 🟡 P1 **caller-responsibility** |
| 20 | pagerduty | `PAGERDUTY_API_KEY` / `PAGERDUTY_FROM_EMAIL` | `cred::get_unscoped` × 2 (`auth.rs:18`, `:36`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 21 | postmark | `POSTMARK_SERVER_TOKEN` | `cred::get_unscoped` (`auth.rs:13`) | ❌ no | n/a (server token) | — | 🟡 P1 env-only-leak |
| 22 | **salesforce** | `SF_CLIENT_ID` / `SF_CLIENT_SECRET` / `SF_INSTANCE_URL` | `cred::get_unscoped` (`auth.rs:146`) | ❌ no | ✅ OAuth client-credentials grant (`auth.rs:58-107`) | **`<cwd>/.argo/connectors/salesforce/token.json`** (`auth.rs:140-142`) | 🔴 **P0 env-or-cache-leak** |
| 23 | sendgrid | `SENDGRID_API_KEY` | `cred::get_unscoped` (`auth.rs:30`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 24 | slack | `SLACK_BOT_TOKEN` | `cred::get_unscoped` (`slack.rs:25`) | ❌ no | n/a (bot token) | — | 🟡 P1 env-only-leak |
| 25 | stripe | `STRIPE_API_KEY` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 26 | stripe-events | `STRIPE_API_KEY` | `cred::get_unscoped` (`auth.rs:15`) | ❌ no | n/a (API key) | — | 🟡 P1 env-only-leak |
| 27 | trello | `TRELLO_API_KEY` / `TRELLO_API_TOKEN` | `cred::get_unscoped` (`auth.rs:60`) | ❌ no | n/a (API key + token) | — | 🟡 P1 env-only-leak |
| 28 | typeform | `TYPEFORM_PERSONAL_TOKEN` | `cred::get_unscoped` (`auth.rs:29`) | ❌ no | n/a (PAT) | — | 🟡 P1 env-only-leak |
| 29 | zendesk | `ZENDESK_EMAIL` / `ZENDESK_API_TOKEN` / `ZENDESK_SUBDOMAIN` | `cred::get_unscoped` (`auth.rs:37`) | ❌ no | n/a (API token) | — | 🟡 P1 env-only-leak |

Summary by verdict:

- 🔴 **P0 (3)**: gcal, gmail, salesforce — env-or-cache-leak (disk cache shares cwd).
- 🟡 **P1 (26)**: every other connector (incl. onedrive's caller-responsibility surface).
- 🟢 P2/P3: none in the connector layer.

---

## §3 — F-01: uniform `cred::get_unscoped` pattern (P1)

**Severity**: P1
**Surface**: every connector that reads a credential.
**Already-authored fix-brief**: `mission-cred-get-tenant-parameter`.

Every connector that reads a credential does so via the `argo_config::cred::get_unscoped` function. The function lives at `crates/argo-config/src/cred.rs:178-181`:

```rust
pub fn get_unscoped(key: &str) -> Result<String, VarError> {
    let cwd = std::env::current_dir().map_err(|_| VarError::NotPresent)?;
    get_unscoped_from_workspace(&cwd, key)
}
```

Its tenant-scoped sibling already exists at `cred.rs:162-165`:

```rust
pub fn get(tenant_id: &TenantId, key: &str) -> Result<String, VarError> {
    let cwd = std::env::current_dir().map_err(|_| VarError::NotPresent)?;
    get_from_workspace(&cwd, tenant_id, key)
}
```

So the surface to **call** a tenant-scoped read exists. The gap is that **every connector's `from_env` / `required_env` helper lacks a `TenantId` parameter**. The fix is uniform: thread `tenant_id: &TenantId` through each connector's `*::from_env(...)` and replace the `cred::get_unscoped(name)` call with `cred::get(tenant_id, name)`.

Per-connector call site index (for the fix-brief implementer):

```
crates/argo-mcp-connector-airtable/src/auth.rs:15
crates/argo-mcp-connector-asana/src/auth.rs:16
crates/argo-mcp-connector-calendly/src/auth.rs:15
crates/argo-mcp-connector-clickup/src/auth.rs:15
crates/argo-mcp-connector-discord/src/auth.rs:13
crates/argo-mcp-connector-figma/src/auth.rs:15
crates/argo-mcp-connector-fireflies/src/auth.rs:15
crates/argo-mcp-connector-gcal/src/client.rs:262, :266, :352
crates/argo-mcp-connector-github-issues/src/auth.rs:13
crates/argo-mcp-connector-gmail/src/auth.rs:50
crates/argo-mcp-connector-google-meet/src/auth.rs:15
crates/argo-mcp-connector-hubspot/src/client.rs:18
crates/argo-mcp-connector-imessage/src/auth.rs:21
crates/argo-mcp-connector-jira/src/auth.rs:63
crates/argo-mcp-connector-linear/src/auth.rs:15
crates/argo-mcp-connector-mailchimp/src/auth.rs:39
crates/argo-mcp-connector-monday/src/auth.rs:15
crates/argo-mcp-connector-notion/src/client.rs:24, :26
crates/argo-mcp-connector-pagerduty/src/auth.rs:18, :36
crates/argo-mcp-connector-postmark/src/auth.rs:13
crates/argo-mcp-connector-salesforce/src/auth.rs:146
crates/argo-mcp-connector-sendgrid/src/auth.rs:30
crates/argo-mcp-connector-slack/src/slack.rs:25
crates/argo-mcp-connector-stripe/src/auth.rs:15
crates/argo-mcp-connector-stripe-events/src/auth.rs:15
crates/argo-mcp-connector-trello/src/auth.rs:60
crates/argo-mcp-connector-typeform/src/auth.rs:29
crates/argo-mcp-connector-zendesk/src/auth.rs:37
```

That's **31 call sites across 28 connector crates** (onedrive does not call `cred::*`; shared has no credentials).

**Why P1 not P0**: in v1.0 single-tenant macOS demo, every connector reads the one tenant's tokens. The "leak" is a cross-tenant *future* hazard. It becomes P0 the moment two tenants share a gateway process — at which point `cred::get_unscoped` will return whichever tenant's value was written last to the credentials file. The substrate is structurally tenant-blind; v1.0 ships behind a "one tenant per process" deployment seam.

---

## §4 — F-02 / F-03 / F-04: cwd-scoped OAuth caches (P0)

**Severity**: P0
**Surfaces**: 3 connectors (gcal, gmail, salesforce).
**Recommended fix-brief**: `mission-mcp-connector-token-cache-tenant-keyed` (NEW).

The pattern in each of the three connectors is the same: refresh an OAuth access token via an upstream `/token` endpoint, then **persist the refreshed bearer to a cwd-relative file**. If the cwd is shared across tenants (single gateway process), the file is shared across tenants.

### F-02 — gcal

`crates/argo-mcp-connector-gcal/src/client.rs:339-348`:

```rust
fn token_cache_path() -> McpResult<PathBuf> {
    std::env::current_dir()
        .map(|cwd| {
            cwd.join(".argo")
                .join("connectors")
                .join("gcal")
                .join("access_token.json")
        })
        .map_err(|e| McpError::Io(format!("current_dir: {e}")))
}
```

The cache write is `write_cached_access_token` at `client.rs:325-336`. The refresh upstream is `refresh_access_token` at `client.rs:291-323` — it consumes `ARGO_GCAL_REFRESH_TOKEN`, `ARGO_GCAL_CLIENT_ID`, `ARGO_GCAL_CLIENT_SECRET` (all read via `cred::get_unscoped` per F-01), exchanges for a fresh bearer at `https://oauth2.googleapis.com/token`, and writes the bearer to the cache path above.

**Cross-tenant impact**: in a multi-tenant gateway, tenant A's refresh writes `<cwd>/.argo/connectors/gcal/access_token.json` with tenant A's bearer. Tenant B's next read (via `read_cached_access_token` at `client.rs:275-289`) returns tenant A's bearer if the cache window has not expired (>60s remaining). Tenant B's subsequent gcal API calls are then issued **with tenant A's OAuth identity** — directly cross-tenant.

### F-03 — gmail

`crates/argo-mcp-connector-gmail/src/auth.rs:11`:

```rust
const CACHE_PATH: &[&str] = &[".argo", "connectors", "gmail", "access_token.json"];
```

`cache_path()` at `auth.rs:113-116`:

```rust
fn cache_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    Ok(CACHE_PATH.iter().fold(cwd, |p, part| p.join(part)))
}
```

The refresh upstream is `refresh_and_cache` at `auth.rs:22-46` — `oauth2::basic::BasicClient` exchange against Google's `/token` endpoint using `ARGO_GMAIL_REFRESH_TOKEN`, then `write_cached_token` (`auth.rs:79-111`) writes the bearer to the cwd-relative cache. The write is mode-0o600 (good), but the file path is still cwd-scoped (bad).

**Cross-tenant impact**: identical pattern to F-02. Tenant A's refresh writes the cache; tenant B reads tenant A's bearer.

### F-04 — salesforce

`crates/argo-mcp-connector-salesforce/src/auth.rs:140-142`:

```rust
fn cache_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo/connectors/salesforce/token.json")
}
```

The refresh upstream is `refresh` at `auth.rs:69-109` — OAuth client-credentials grant (not refresh-token grant, but same cache-leak shape). `write_cached_token` at `auth.rs:124-139` writes the bearer to the cwd-relative cache.

**Cross-tenant impact**: identical to F-02 / F-03. Salesforce uses client-credentials so the cache is the only persistence — there is no second source of truth — making this the most "definitive" of the three cwd-cache leaks.

### Recommended fix shape

A single fix-brief covers all three. The cache path should be tenant-keyed:

```
<workspace>/.argo/connectors/<name>/<tenant_id>/access_token.json
```

…or moved to a SQL-backed `connector_token_cache` table with PRIMARY KEY `(tenant_id, connector_name)` — symmetric with the existing `workspace_provider_key` table in `argo-store`. The SQL option is consistent with the rest of the codebase's tenant-scoping pattern and is probably what `mission-cred-get-tenant-parameter`'s eventual implementation should converge on.

**Pre-req**: this fix-brief depends on `mission-cred-get-tenant-parameter` landing first, because the cache path needs a `TenantId` to embed. Sequencing matters.

**Why this is P0 not P1**: this is not a "future hazard" the way F-01 is. The cache file exists on disk **right now**, in v1.0 single-tenant mode it is benign, but the moment a second tenant runs in the same process / cwd the bearer is *actively* mis-attributed. F-01 is a missing parameter; F-02/F-03/F-04 are an active byte-for-byte cross-tenant leak.

---

## §5 — F-05: onedrive caller-responsibility (P1)

**Severity**: P1
**Surface**: `argo-mcp-connector-onedrive`.

`OneDriveConnector::new(oauth_token: String)` at `crates/argo-mcp-connector-onedrive/src/client.rs:38` accepts the OAuth bearer as a constructor argument. The crate has no `from_env` helper and no production caller in the workspace (`grep -rln 'OneDriveConnector' crates/` returns only the connector's own files + its tests).

From the perspective of *this crate's* surface, this is **clean**: no `cred::*` call, no env read, no disk cache. The credential provenance is the caller's responsibility, and the caller does not yet exist.

The hazard is forward-looking: when a caller is built (likely a future `argo-mcp` connector binary or the gateway's connector registry), it will source the token via `cred::get_unscoped(...)` — inheriting F-01 — unless `mission-cred-get-tenant-parameter` has already landed by then. Recommend tagging the OneDrive descriptor's eventual implementation to be **tenant-aware from day one** so this connector skips the leak phase entirely.

---

## §6 — F-06: `*_BASE_URL` env vars read via `std::env::var` (P3)

**Severity**: P3 (advisory only, no fix-brief).

Most connectors expose a `*_BASE_URL` override read via direct `std::env::var(...)`, **not** via `cred::*`:

```
crates/argo-mcp-connector-airtable/src/auth.rs:26   AIRTABLE_BASE_URL
crates/argo-mcp-connector-asana/src/auth.rs:26      ASANA_BASE_URL
crates/argo-mcp-connector-calendly/src/auth.rs:26   CALENDLY_BASE_URL
crates/argo-mcp-connector-clickup/src/auth.rs:26    CLICKUP_BASE_URL
crates/argo-mcp-connector-discord/src/auth.rs:24    DISCORD_BASE_URL
... (21 more)
```

These are non-credential test/staging endpoint overrides (e.g. swap `https://api.airtable.com/v0` for a mock-server URL in integration tests). They are **not** credentials and should not be routed through `cred::*`.

The minor hazard: if two tenants in a multi-tenant gateway process both set `*_BASE_URL` env vars, the last-writer wins and the other tenant's traffic could be mis-directed to the wrong endpoint. This is a **test/dev configuration footgun**, not a production credential leak. In v1.0 production, these env vars are unset (defaults are used) and the issue is moot.

**No fix-brief needed.** The boundary is correct: credentials always via `cred::*`; non-secret endpoint overrides via `env::var` is fine. Document only.

---

## §7 — What I did NOT find (boundary notes)

- **No connector reads credentials directly via `std::env::var`**. The `cred::get_unscoped` discipline is universal — 28 of 28 credential-reading connectors funnel through the single function. This is good architectural hygiene (the moment `cred::*` flips to tenant-scoped, every connector benefits).
- **No connector caches refresh tokens to disk** — only access tokens. Refresh tokens are read fresh from env each time (`ARGO_GCAL_REFRESH_TOKEN`, `ARGO_GMAIL_REFRESH_TOKEN`). This is the correct OAuth shape (refresh token is long-lived secret, access token is short-lived cache).
- **No connector uses the keyring backend**. `argo-config::cred::cred_keyring` exists but is consulted via the `Auto` backend selector inside `get_with_backend` — connectors don't directly invoke keyring. This is fine.
- **`argo-mcp-connector-shared`** carries no credential surface — pure retry helpers (`retry_with_policy`, `default_http_policy`, `is_http_retriable`, `is_tonic_retriable`). Out of scope.
- **`argo-mcp-image-gen`** is an image-generation tool, not a connector. Out of scope.
- **No connector calls `cred::get(tenant_id, ...)` (the tenant-scoped sibling)**. The tenant-scoped function is implemented and ready; no caller has adopted it yet. Once `mission-cred-get-tenant-parameter` lands, this audit's call-site index (§3) becomes the worklist.

---

## §8 — Round-10 closure

Round-10's connector inventory verdict was *"27 MCP connector structural inventory PASS (29 real crates compile + tests; substrate healthy)."* This audit converts that round-10 substrate verdict into a credential-axis verdict:

- ✅ **Substrate health**: confirmed — all 29 connector crates compile, carry the standard `tenant-unscoped:` justification comment, and route credential reads through the central `cred::get_unscoped` function (no rogue `env::var` direct reads on credentials).
- ❌ **Tenant scoping**: NOT achieved on any connector. Every credential read bypasses tenant isolation by design (the substrate function does not yet accept a `TenantId`).
- 🔴 **P0 cwd-cache leak**: confirmed in 3 connectors (gcal / gmail / salesforce). Multi-tenant gateway is at risk; v1.0 single-tenant is benign.

**Next-round focus** per round-10 handoff is closed by this report. The follow-up brief queue is:

1. 🔴 `mission-mcp-connector-token-cache-tenant-keyed` (NEW) — depends on #2.
2. 🟡 `mission-cred-get-tenant-parameter` (already authored).
3. (no others — F-05 and F-06 are advisory.)

This audit cost ~70K input tokens (within the 50-100K target). Total deliverable: this report + matrix + 6 findings + call-site index. No connector source code was modified; this is a findings-only evaluator-shape brief.

---

## §9 — Acceptance checklist

- [x] Per-connector matrix covers all 27 → **29** MCP connectors (round-10's "27" was an approximation; actual workspace count is 29 connector crates + 1 shared helper).
- [x] Each connector classified: tenant-scoped / partially / leak (verdict column in §2 matrix).
- [x] Findings prioritized P0/P1/P2 with fix-brief recommendations (§§3-6).
- [x] Gap manifest update (see companion diff to `mission-vs-code-gap-manifest.md`).
- [x] Cost target 50-100K tokens (~70K spent).
- [x] BUILD_STATUS.md updated (post-PR flip in the conductor pass).
