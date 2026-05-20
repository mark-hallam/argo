# web-api-keys-service-accounts — /settings/api-keys page with cohort-scoped tokens

**Owner**: claude. **Sizing**: ~300-500 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-api-keys-service-accounts`.
**Priority**: P1 — today bearer-token access is a hidden dev path. Enterprise customers ask for "service account" tokens with cohort-scoped permissions so their CI / data pipeline / integration can call the SKYCOT API without using a human's session. This page surfaces it.

## Scope

A new page at `/settings/api-keys` for minting long-lived API tokens.

### 1. Page layout

- Brand-restrained register (matches the rest of `/settings/*`).
- Tab joins the existing `/settings` tab row alongside Members, Security, Billing, etc.
- Heading: "API keys" + 1-line subtitle ("Long-lived tokens for service accounts, CI, and integrations.").
- "Create API key" button top-right (gradient pill).
- Table of existing keys:
  - Name
  - Prefix (e.g. `sk_live_a4...`) — masked except last 4 chars
  - Scopes (cohort chips + role badge)
  - Created (relative time)
  - Last used (relative time, or "Never")
  - Expires (date or "Never")
  - Actions: Revoke (red destructive)
- Empty state: large illustration + "No API keys yet. Mint one to wire SKYCOT into your CI or integration." + CTA.

### 2. Create-key modal

Opens when "Create API key" is clicked. Fields:

- **Name** — text input (required), placeholder "CI pipeline" / "Data warehouse sync" etc.
- **Scopes** — checkbox grid:
  - Read workers / write workers
  - Read workflows / write workflows
  - Read audit / write audit
  - Read skills / write skills
  - Read members / write members (admin-only — gated)
  - Run workflows
- **Cohort scope** — multi-select of cohorts the key can act on (or "All cohorts" toggle). Same UX as the existing member-cohort-scope picker.
- **Expiry** — radio: Never / 30 days / 90 days / 1 year / Custom date.
- Submit button: "Create key".

### 3. Reveal-once flow

After submission:
- Modal swaps to a reveal panel showing the FULL token (e.g. `sk_live_a4b8...c2e1`).
- Big yellow warning banner: "Copy this token now. You won't see it again."
- "Copy to clipboard" button.
- "I've saved it — close" button (only this dismisses the modal).
- Closing without acknowledging: confirm dialog "Are you sure? You will lose access to this token."

### 4. Revoke flow

- Revoke button → confirm dialog "Revoke this key? Any system using it will stop working immediately."
- On confirm, gateway DELETE `/api/v1/api-keys/<id>` → row updates in place to show "Revoked" pill + grayed out.

### 5. API integration

- `web/lib/api/api-keys.ts` — `fetchApiKeys()`, `createApiKey()`, `revokeApiKey()`.
- `web/lib/hooks/use-api-keys.ts` — TanStack Query hook.
- Gateway endpoints: `GET /api/v1/api-keys`, `POST /api/v1/api-keys`, `DELETE /api/v1/api-keys/<id>`. If any are missing, the brief author files a follow-up gateway brief; web side authors against the documented contract regardless.

### 6. Audit integration

Every create / revoke action emits an audit entry with the actor + the key name (NOT the token itself). Surface "API key created: 'CI pipeline'" / "API key revoked: 'CI pipeline'" in the /audit feed.

## Acceptance criteria

- [ ] `/settings/api-keys` renders with the existing settings tab row + a new "API keys" tab.
- [ ] Existing keys list with masked prefix, scopes (cohort chips), created/last-used/expiry.
- [ ] Create-key modal collects name, scopes, cohort scope, expiry.
- [ ] After create, the modal reveals the full token ONCE with a copy button + yellow warning.
- [ ] Revoke action confirms then disables the key in-place.
- [ ] Empty state renders cleanly when no keys exist.
- [ ] Audit feed shows API key create / revoke entries.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Per-key rate limits (v1.x).
- IP allowlists (v1.x).
- OAuth client-credentials flow (v1.x — this is a static bearer-token brief).
- Rotation reminders or auto-rotation (v1.x).
- Gateway-side API key store implementation if missing (separate brief).
