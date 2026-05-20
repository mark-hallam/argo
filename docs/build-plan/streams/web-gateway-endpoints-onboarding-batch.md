# web-gateway-endpoints-onboarding-batch — implement 3 onboarding endpoints

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-onboarding-batch`.
**Priority**: 🔴 **P0 — signup → first-hire path silently 404s as success**.
Wave-3 onboarding audit: 3 of 4 onboarding endpoints have no backend, the
hand-rolled fetch at `web/app/(setup)/onboarding/pick-vertical/page.tsx:74`
treats gateway 404 as success — so users reach step 5, click "Hire", and
the page just hangs.
**Sizing**: ~900-1200 LOC across argo-store (provider-keys table) + new
gateway handlers (3) + tests + web silent-404-bug fix.

---

## Why this brief exists

From wave-3 onboarding audit:

> Three of four onboarding endpoints have no backend. Only
> `/api/v1/onboarding/provision` exists in `crates/`; `/provider-key`,
> `/install-vertical`, `/first-hire` are unimplemented. First-hire
> has no fallback so users hard-stick on step 5.

The user-facing failure mode: brand-new account → magic-link verify →
provision → enter provider key (silent 404, "success" shown) → pick vertical
(silent 404, "success" shown) → first hire (silent 404, page hangs).

After this brief, all 3 endpoints exist and the silent-404 web bug is fixed.

---

## Pre-flight verification

```bash
# 1. Confirm only /provision exists today.
ls crates/argo-cli/src/handlers/gateway/onboarding_*.rs

# 2. Confirm the silent-404 bug location.
rg "fetch.*onboarding/first-hire" web/app/

# 3. Confirm provision handler pattern.
head -50 crates/argo-cli/src/handlers/gateway/onboarding_provision.rs

# 4. Confirm provider list — operator directive to source from svgl.app.
rg "providers.*anthropic\|anthropic.*openai" web/app/\(setup\)/

# 5. Confirm vertical manifest path.
ls verticals/ | head -10
```

---

## The 3 endpoints

### 1. POST /api/v1/onboarding/provider-key

Body:
```json
{
  "provider": "anthropic" | "openai" | "google" | "azure" | "aws_bedrock",
  "api_key": "sk-ant-..."
}
```

Returns:
```json
{
  "validated": true,
  "masked_key": "sk-ant-...abc1",
  "test_cost": 0.0001,
  "test_tokens": 5,
  "provider": "anthropic"
}
```

Server performs a 1-token validation call to the provider and reports the
cost + tokens of that test call. **Contract drift note (Wave-11 audit 2026-05-21)**:
web client `web/lib/onboarding/use-provider-key.ts:20-24` expects exactly
`{ validated, test_cost, test_tokens }` — match that shape, do NOT add or
rename fields without coordinating the web change.

### 2. POST /api/v1/onboarding/install-vertical

Body: `{ "vertical_id": "pr_checker" }`.

Returns:
```json
{
  "installed": true,
  "vertical_id": "pr_checker",
  "workflows_added": 3,
  "skills_added": 7
}
```

### 3. POST /api/v1/onboarding/first-hire

Body:
```json
{
  "description": "Maya — a GTM engineer who reviews PRs and writes outreach",
  "vertical_id": "pr_checker"
}
```

Returns:
```json
{
  "worker_id": "wkr_01HX...",
  "name": "Maya",
  "vertical_id": "pr_checker"
}
```

---

## Phase 1 — Provider-key storage

### 1a. Migration

**File**: `crates/argo-store/migrations/NNN__workspace_provider_keys.sql` (next slot)

```sql
-- Stream `web-gateway-endpoints-onboarding-batch` — encrypted-at-rest
-- LLM provider API keys per workspace. The encryption envelope uses
-- the existing SQLCipher page key when SQLCipher is on; otherwise
-- AES-GCM with a per-process key from `~/.argo/keystore.toml`.
--
-- One row per (tenant, provider). Re-keying the same provider
-- replaces the row.

CREATE TABLE IF NOT EXISTS workspace_provider_keys (
    tenant_id    TEXT      NOT NULL,
    provider     TEXT      NOT NULL,
    ciphertext   BLOB      NOT NULL,
    nonce        BLOB      NOT NULL,
    masked       TEXT      NOT NULL,
    created_at   INTEGER   NOT NULL,
    last_used_at INTEGER,
    PRIMARY KEY (tenant_id, provider)
);
```

### 1b. Store module

**File**: `crates/argo-store/src/workspace_provider_keys.rs` (new, ~180 LOC)

```rust
use crate::SessionStore;
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use argo_types::{ArgoError, ArgoResult, TenantId};
use rand::RngCore;

pub async fn store_key(
    store: &SessionStore,
    tenant: &TenantId,
    provider: &str,
    api_key: &str,
    encryption_key: &[u8; 32],
) -> ArgoResult<String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(encryption_key));
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, api_key.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("encrypt: {e}")))?;
    let masked = mask_key(api_key);
    let now = chrono::Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO workspace_provider_keys \
         (tenant_id, provider, ciphertext, nonce, masked, created_at) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(tenant_id, provider) DO UPDATE SET \
         ciphertext = excluded.ciphertext, \
         nonce = excluded.nonce, \
         masked = excluded.masked, \
         created_at = excluded.created_at"
    )
    .bind(tenant.to_string())
    .bind(provider)
    .bind(&ciphertext)
    .bind(&nonce_bytes[..])
    .bind(&masked)
    .bind(now)
    .execute(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("insert provider key: {e}")))?;
    Ok(masked)
}

pub async fn fetch_key(
    store: &SessionStore,
    tenant: &TenantId,
    provider: &str,
    encryption_key: &[u8; 32],
) -> ArgoResult<Option<String>> {
    /* SELECT ciphertext + nonce, AES-GCM decrypt, return plaintext.
     * Caller never logs the result. */
}

pub fn mask_key(api_key: &str) -> String {
    let len = api_key.len();
    if len < 8 { return "•".repeat(len); }
    format!("{}…{}", &api_key[..6], &api_key[len.saturating_sub(4)..])
}
```

**`encryption_key` source**: read from `~/.argo/keystore.toml` (existing).
If unset, generate + persist a fresh 32-byte key on first call. **NEVER
synthesize a default — log fatal and refuse to start** if keystore is
missing AND not writable.

---

## Phase 2 — Provider-key handler

**File**: `crates/argo-cli/src/handlers/gateway/onboarding_provider_key.rs` (new)

```rust
async fn provider_key_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    // 1. Auth (same pattern as onboarding_provision.rs).
    let principal = match resolve_principal_or_401(req) { Ok(p) => p, Err(r) => return r };
    let store = match store { Some(s) => s, None => return store_unavailable() };
    let body = body_from_request(req).unwrap_or_default();
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t, Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };

    // 2. Parse body.
    let opts: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v, Err(e) => return bad_request(&format!("invalid body: {e}")),
    };
    let provider = match opts.get("provider").and_then(|v| v.as_str()) {
        Some(p) if KNOWN_PROVIDERS.contains(&p) => p.to_string(),
        _ => return bad_request("provider must be one of: anthropic, openai, google, azure, aws_bedrock"),
    };
    let api_key = match opts.get("api_key").and_then(|v| v.as_str()) {
        Some(k) if !k.is_empty() => k.to_string(),
        _ => return bad_request("api_key must be a non-empty string"),
    };

    // 3. Validate the key with a 1-token test call.
    let (cost, tokens) = match validate_key(&provider, &api_key).await {
        Ok(r) => r,
        Err(e) => return bad_request(&format!("provider rejected key: {e}")),
    };

    // 4. Encrypt + persist.
    let encryption_key = match load_workspace_encryption_key().await {
        Ok(k) => k, Err(e) => return server_error(&format!("keystore: {e}")),
    };
    let masked = match argo_store::workspace_provider_keys::store_key(
        store, &tenant, &provider, &api_key, &encryption_key
    ).await {
        Ok(m) => m, Err(e) => return server_error(&format!("store key: {e}")),
    };

    // 5. Audit.
    let _ = sink.emit(AuditKind::ProviderKeyConfigured, json!({
        "actor": principal,
        "tenant_id": tenant.to_string(),
        "provider": provider,
        "masked_key": masked,
        "test_cost_usd": cost,
        "test_tokens": tokens,
    })).await;

    ("200 OK", json!({
        "validated": true,
        "masked_key": masked,
        "test_cost": cost,
        "test_tokens": tokens,
        "provider": provider,
    }).to_string())
}

const KNOWN_PROVIDERS: &[&str] = &["anthropic", "openai", "google", "azure", "aws_bedrock"];

async fn validate_key(provider: &str, api_key: &str) -> ArgoResult<(f64, u64)> {
    // Issue a 1-token test request to the provider. Returns (cost, tokens).
    // Wrap the existing argo-provider crate's ProviderClient.
    let client = argo_provider::Client::from_provider_name(provider, api_key)?;
    let response = client.completion("test", 1).await
        .map_err(|e| ArgoError::Provider(format!("test call: {e}")))?;
    Ok((response.cost_usd, response.tokens_used))
}
```

Add `AuditKind::ProviderKeyConfigured` (4-site recipe). Label:
`"provider_key.configured"`.

---

## Phase 3 — Install-vertical handler

**File**: `crates/argo-cli/src/handlers/gateway/onboarding_install_vertical.rs` (new)

The `argo-vertical` crate has `parse_vertical(toml)` and `parse_workflow(toml)`
loaders. Verticals live in `verticals/<id>/` with `vertical.toml` +
`workflows/*.workflow.toml` + `skills/*.skill.md`.

```rust
async fn install_vertical_response(req: &str, store: ...) -> (&'static str, String) {
    let opts: serde_json::Value = serde_json::from_str(&body)?;
    let vertical_id = opts.get("vertical_id").and_then(|v| v.as_str())
        .ok_or_else(|| bad_request("vertical_id required"))?;

    // 1. Find the vertical pack on disk.
    let pack_dir = workspace_root().join("verticals").join(vertical_id);
    if !pack_dir.exists() {
        return not_found(&format!("vertical '{vertical_id}' not in catalog"));
    }

    // 2. Parse + insert workflows.
    let mut workflows_added = 0;
    for entry in std::fs::read_dir(pack_dir.join("workflows"))? {
        let path = entry?.path();
        if path.extension().map(|x| x == "toml").unwrap_or(false) {
            let toml = std::fs::read_to_string(&path)?;
            let parsed = argo_vertical::parse_workflow(&toml)?;
            argo_store::workflow_definition::create(
                store.pool(), tenant.clone(), vertical_id,
                &parsed.name, &toml, None, &principal
            ).await?;
            workflows_added += 1;
        }
    }

    // 3. Parse + insert skills (similar pattern).
    let mut skills_added = 0;
    // ... bulk-insert into skill table ...

    // 4. Audit row.
    let _ = sink.emit(AuditKind::VerticalInstalled, json!({
        "actor": principal,
        "tenant_id": tenant.to_string(),
        "vertical_id": vertical_id,
        "workflows_added": workflows_added,
        "skills_added": skills_added,
    })).await;

    ("200 OK", json!({
        "installed": true,
        "vertical_id": vertical_id,
        "workflows_added": workflows_added,
        "skills_added": skills_added,
    }).to_string())
}
```

Add `AuditKind::VerticalInstalled` (4-site recipe). Label: `"vertical.installed"`.

---

## Phase 4 — First-hire handler

**File**: `crates/argo-cli/src/handlers/gateway/onboarding_first_hire.rs` (new)

This is the thinnest of the three — delegate to the existing
`routes_workers_hire::hire_worker(...)` logic which already creates a worker
row. The onboarding wrapper just:

1. Parses the description.
2. Calls an LLM (small + cheap, hardcoded `claude-haiku` or similar) to
   distill `{ name, primary_duty, vertical_id }` from the description.
3. Calls the existing `hire_worker(...)` with the distilled fields.
4. Returns `{ worker_id, name, vertical_id }`.

```rust
async fn first_hire_response(req: &str, store: ...) -> (&'static str, String) {
    let opts: FirstHireRequest = serde_json::from_str(&body)?;

    // 1. LLM-distill name + duty (1-2 second call).
    let distilled = match distill_worker_from_description(&opts.description).await {
        Ok(d) => d, Err(e) => return server_error(&format!("distill: {e}")),
    };

    // 2. Delegate to hire-worker handler.
    let hire_body = json!({
        "vertical_id": opts.vertical_id,
        "name": distilled.name,
        "duties": distilled.duties,
    }).to_string();
    let (status, body) = super::routes_workers_hire::hire_response(req, store).await;
    if status != "200 OK" && status != "201 Created" {
        return (status, body);
    }

    // 3. Parse worker_id from hire response.
    let hire_resp: serde_json::Value = serde_json::from_str(&body)?;
    let worker_id = hire_resp.get("worker_id").and_then(|v| v.as_str())
        .unwrap_or_default().to_string();

    // 4. Audit (the hire handler already emits WorkerHired — don't double-emit).
    ("201 Created", json!({
        "worker_id": worker_id,
        "name": distilled.name,
        "vertical_id": opts.vertical_id,
    }).to_string())
}
```

---

## Phase 5 — Fix the silent-404 web bug

**File**: `web/app/(setup)/onboarding/pick-vertical/page.tsx` (and sibling
`first-hire/page.tsx`)

Find the hand-rolled fetches at line ~74. Replace with the proper `apiPost`
helper from `web/lib/api/client.ts`:

```diff
- const res = await fetch("/api/v1/onboarding/first-hire", {
-   method: "POST",
-   headers: { "content-type": "application/json" },
-   body: JSON.stringify({ description, vertical_id }),
- });
- // BUG: response.ok is true for 404, treats as success.
- router.push("/workers");
+ try {
+   const result = await apiPost<{ worker_id: string; name: string }>(
+     "/api/v1/onboarding/first-hire",
+     { description, vertical_id }
+   );
+   router.push(`/workers/${result.worker_id}/overview`);
+ } catch (e) {
+   if (e instanceof ApiError) {
+     setError(`Couldn't create worker: ${e.message}`);
+   } else {
+     setError("Couldn't reach gateway. Check your network and try again.");
+   }
+ }
```

`apiPost` correctly throws on non-2xx; the silent-404 bug was in
`response.ok` being lenient.

---

## Phase 6 — Logos directive

Operator memory: `feedback_logos_from_svgl_app.md` — all provider logos
on `pick-vertical` / `provider-key` pages MUST come from https://svgl.app/.

Download SVGs to:
- `web/public/logos/anthropic.svg`
- `web/public/logos/openai.svg`
- `web/public/logos/google.svg`
- `web/public/logos/azure.svg`
- `web/public/logos/aws.svg`

Use in the provider picker:
```tsx
<Image src="/logos/anthropic.svg" alt="Anthropic" width={48} height={48} />
```

---

## Phase 7 — Tests

`crates/argo-cli/src/handlers/gateway/onboarding_provider_key_tests.rs` (new):
1. `valid_key_returns_200_with_validation_data`
2. `invalid_provider_returns_400`
3. `empty_key_returns_400`
4. `provider_rejection_returns_400_with_message`
5. `key_persisted_encrypted_round_trip`
6. `re_storing_same_provider_overwrites_not_duplicates`
7. `mask_key_format_six_dot_four`

`crates/argo-cli/src/handlers/gateway/onboarding_install_vertical_tests.rs`:
8. `install_known_vertical_seeds_workflows_and_skills`
9. `install_unknown_vertical_returns_404`
10. `install_idempotent_re_install_is_no_op` (or returns "already installed")

`crates/argo-cli/src/handlers/gateway/onboarding_first_hire_tests.rs`:
11. `first_hire_creates_worker_row`
12. `first_hire_emits_worker_hired_audit`

---

## may-touch

- `crates/argo-store/migrations/NNN__workspace_provider_keys.sql` (new).
- `crates/argo-store/src/migrations/vNN.rs` (new).
- `crates/argo-store/src/workspace_provider_keys.rs` (new) + tests.
- `crates/argo-store/Cargo.toml` — add `aes-gcm` workspace dep if missing.
- `crates/argo-cli/src/handlers/gateway/onboarding_provider_key.rs` (new) + tests.
- `crates/argo-cli/src/handlers/gateway/onboarding_install_vertical.rs` (new) + tests.
- `crates/argo-cli/src/handlers/gateway/onboarding_first_hire.rs` (new) + tests.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — wire 3 new arms.
- `crates/argo-audit/src/event.rs` — add 2 variants (`ProviderKeyConfigured`, `VerticalInstalled`).
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` — replace hand-rolled fetch with `apiPost`.
- `web/app/(setup)/onboarding/first-hire/page.tsx` — same.
- `web/public/logos/{anthropic,openai,google,azure,aws}.svg` — download from svgl.app.

## must-NOT-touch

- `/api/v1/onboarding/provision` — already shipped + stable.
- Vertical-pack TOML format / parser (`argo-vertical`).
- Existing hire-worker logic (`routes_workers_hire`) — delegate only.
- `~/.argo/keystore.toml` schema — extend additively if needed.

---

## Acceptance criteria

- [ ] `POST /api/v1/onboarding/provider-key` validates the key with a real provider call and returns `{ validated, masked_key, test_cost, test_tokens, provider }`.
- [ ] Wrong key returns 400 with provider's error message.
- [ ] Key is stored encrypted; SELECT on the table shows ciphertext only.
- [ ] `POST /api/v1/onboarding/install-vertical` seeds workflows + skills from the on-disk pack.
- [ ] Unknown vertical returns 404.
- [ ] `POST /api/v1/onboarding/first-hire` creates a real worker visible at `/workers/<id>/overview`.
- [ ] Web `pick-vertical` and `first-hire` pages no longer treat 404 as success — error surfaces in UI.
- [ ] Provider logos appear from `/logos/<name>.svg` (downloaded from svgl.app).
- [ ] Cross-tenant requests return 404.
- [ ] Audit rows `provider_key.configured` and `vertical.installed` visible in /audit.
- [ ] 12+ tests pass.
- [ ] `cargo clippy -p argo-store -p argo-cli -p argo-audit --all-targets -- -D warnings` clean.
- [ ] E2E: brand-new account → provision → enter key → pick vertical → first-hire → land on `/workers/<id>/overview`. No silent fall-throughs.

---

## Gotchas

1. **Provider validation call cost**: a 1-token completion costs ~$0.00001
   — fine. But repeated invalid keys could rack up cost from a malicious
   actor hammering POST /provider-key. **Rate-limit** at the gateway: max
   10 attempts per tenant per hour. Use the existing rate-limit
   middleware if present, else add a simple in-memory counter (acceptable
   for v1 single-process gateway).
2. **AES-GCM nonce reuse is catastrophic**: every encryption MUST use a
   fresh random 12-byte nonce. The `rand::rngs::OsRng.fill_bytes(...)` call
   in the brief is correct. NEVER use a deterministic nonce.
3. **Keystore file permissions**: `~/.argo/keystore.toml` must be `0600`
   (operator read+write only). Boot fails if perms are wider.
4. **Install-vertical idempotency**: re-installing should NOT duplicate
   workflows / skills. The store-level `create` calls already deduplicate
   on `(tenant, vertical, name)` — but the brief should also return a
   distinct response when nothing was added: `{ installed: true, idempotent: true, workflows_added: 0, skills_added: 0 }`.
5. **First-hire LLM-distill failure**: if the LLM call fails (rate-limit /
   provider error / 30s timeout), do NOT block the hire — fall back to a
   sensible default name derived from the description's first noun. Surface
   the failure as a friendly hint: "we couldn't auto-name your worker — they'll
   start as 'New Worker', rename anytime."
6. **Web `apiPost` already handles auth + cookies** — don't add bearer
   headers manually in the rewrite, just call `apiPost(url, body)`.
7. **Contract drift watch**: the `use-provider-key.ts:20-24` shape is
   load-bearing. If a future brief renames `test_cost` to `cost_usd`, the
   server response shape MUST update in lockstep. The Wave-11 audit
   amendment in the prior version of this brief flagged this; preserve it.

---

## References

- Audit: `docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md`
- Wave-11 contract-drift amendment: `docs/skycot/audit/api-wiring/2026-05-21-wave-11-contract-drift.md`
- Shipped provision handler: `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs`
- Operator memory: `feedback_logos_from_svgl_app.md` (svgl.app directive)
- Existing hire handler: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`
- Sibling: `web-onboarding-f1-wizard-shell`
