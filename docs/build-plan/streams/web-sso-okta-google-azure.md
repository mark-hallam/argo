# web-sso-okta-google-azure — Enterprise SSO (OAuth + SAML) wired through

**Owner**: claude. **Sizing**: ~400-600 LOC + provider configs.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-sso-okta-google-azure`.
**Priority**: P1 — enterprise prospects ask "do you have SSO?" before anything else. The auth pages currently show three SSO buttons as "coming v1.x" placeholders. This brief wires them.

## Scope

The auth pages (`/signin`, `/signup`) already show three SSO buttons (Google / GitHub / Microsoft). They currently render as inert. This brief wires up real OAuth flows + adds a SAML configuration UI for enterprise IdPs (Okta + Azure AD).

### 1. OAuth providers — Google / GitHub / Microsoft

For each provider:
- Existing button on `/signin` and `/signup` becomes active.
- Click → starts the OAuth authorization-code flow via gateway `/api/v1/auth/oauth/<provider>/start` (gateway brief separate if endpoint missing; this brief authors the web side + the contract).
- On callback (`/auth/oauth/<provider>/callback`), the gateway exchanges the code + returns a session cookie → web redirects to `/` (or the saved `redirect_to` param).
- New-account path: if the email is unknown, auto-provision the user with the provider's verified email + default role (`user`) and route through the standard onboarding wizard (`/hire`).
- Existing-account path: log in directly, route to `/`.

### 2. SAML config UI in /settings

New tab in `/settings/security` (or extend the existing security tab):

- Section header: "Single Sign-On (SAML 2.0)"
- Status pill: "Not configured" / "Configured: Okta" / "Configured: Azure AD" / "Configured: Custom"
- Configure button → opens a modal with:
  - IdP entity ID (text input)
  - SSO URL (text input)
  - X.509 certificate (textarea, multi-line)
  - "Test connection" button
  - SP metadata download link (returns the SKYCOT service-provider metadata XML — gateway endpoint)
- Once configured:
  - Hide username/password form on `/signin` for this workspace's email domain (the workspace-level "SAML required" toggle controls this).
  - Show "Sign in with SSO" button instead → routes to the IdP.
- Toggle: "Allow non-SSO login for owners" (default off — owners can override during IdP outages).

### 3. SSO discovery

On `/signin`, after the user enters their email and hits Continue:
- Client probes `/api/v1/auth/sso/discover?email=<email>` for the workspace's SSO setting.
- If SAML enforced → "Sign in with SSO" button (routes to IdP) replaces the magic-link / password form.
- If OAuth-only (matched by domain) → highlight the relevant OAuth button.
- If no SSO → fall through to the standard magic-link flow.

### 4. Auto-provisioning rules

- OAuth: provision with `role=user`, no cohort scope, default workspace = the one matching the email domain (or create a new workspace if none).
- SAML: provision with the role + cohort scope claims sent in the assertion (read from configured attribute mappings — the IdP setup modal lets the operator map IdP attributes to SKYCOT role + cohort).

### Files

- `web/app/(auth)/signin/page.tsx` — wire the SSO buttons + discovery probe.
- `web/app/(auth)/signup/page.tsx` — same.
- `web/app/(auth)/auth/oauth/[provider]/callback/page.tsx` — new callback handler page.
- `web/app/(app)/settings/security/page.tsx` — add SAML config section (or new tab).
- `web/components/auth/SsoButtons.tsx` — extract existing buttons + wire onClick.
- `web/lib/api/sso.ts` — client helpers (start OAuth, discover SSO, configure SAML, test SAML).

## Acceptance criteria

- [ ] Google SSO button on /signin and /signup starts the OAuth flow and completes login.
- [ ] GitHub SSO button works end-to-end.
- [ ] Microsoft SSO button works end-to-end.
- [ ] New-account OAuth path auto-provisions + routes through /hire.
- [ ] Existing-account OAuth path logs in + routes to /.
- [ ] /settings/security shows SAML config section with status pill.
- [ ] Configure modal accepts IdP entity ID + SSO URL + X.509 cert.
- [ ] Test-connection button reports success/failure against the IdP.
- [ ] SP metadata download link returns valid XML.
- [ ] SSO discovery on /signin matches workspace domain → shows the right button.
- [ ] "Allow non-SSO login for owners" toggle persists + works.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- OIDC discovery (auto-config from `.well-known/openid-configuration`) — v1.x.
- SCIM provisioning (auto-create/disable users via IdP push) — v1.x.
- IdP-initiated SSO (vs SP-initiated) — v1.x.
- Per-cohort SAML attribute mapping UI beyond a simple JSON mapping textarea — v1.x.
- Backend OAuth/SAML implementation if missing (separate gateway brief — this is web-side wiring + contract).
