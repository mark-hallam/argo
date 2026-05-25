# fix-settings-security-saml-status-failed-to-fetch

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-security-saml-status-failed-to-fetch`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F7 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1 — `/settings/security` SAML panel shows "Not configured" + inline red error text "Failed to fetch". Gateway endpoint isn't wired or SPA is calling a wrong URL.**

## Root cause

`/settings/security` renders the SSO (Single Sign-On) panel with a `Not configured` badge but inline error text `Failed to fetch`. The two CTAs (`Configure SAML`, `Download SP metadata XML`) still render. Likely either:
- (a) The gateway endpoint `/api/v1/settings/saml-status` (or similar) doesn't exist yet.
- (b) The SPA calls it via a path that doesn't match the gateway router.
- (c) CORS-blocked (separate root cause covered by `mission-gateway-cors-preflight`).

Whichever the cause, the visible result is a red "Failed to fetch" string on the security settings surface — looks broken to an investor doing a sanity-check tour of settings.

## Findings this brief closes

- whole-site-mvp-claims-coverage F7 [P1] — `/settings/security` SAML status panel shows "Failed to fetch"

## Fix recipe

1. **Diagnose the source of the error**:
   - Open DevTools network panel on `/settings/security`.
   - Identify the failing request URL.
   - Confirm whether it's 404 (endpoint doesn't exist), CORS-blocked, or 500 (handler error).

2. **If the endpoint doesn't exist (most likely)**:
   - Add `crates/argo-cli/src/handlers/gateway/routes_settings_saml_status_get.rs` (NEW).
   - Returns `{ "configured": false, "metadata_url": null }` for tenants without SAML config.
   - Returns `{ "configured": true, "metadata_url": "...", "last_verified_at": "..." }` for tenants with SAML config.

3. **If the path is wrong**:
   - Update `web/app/(app)/settings/security/page.tsx` or its data hook to use the canonical gateway path.

4. **Render a graceful fallback** instead of "Failed to fetch":
   - When the SAML status hook errors OR the endpoint returns 404 / network error, render `Not configured · Set up SAML SSO to connect your identity provider` with a single primary CTA.
   - Do NOT show the red "Failed to fetch" string to the user.

5. **Pin** with a test at `web/app/(app)/settings/security/page.test.tsx`:
   - Mock the SAML status hook to return network error; assert no "Failed to fetch" string in the rendered output.
   - Mock the hook to return `{configured: false}`; assert "Not configured" copy renders.
   - Mock the hook to return `{configured: true}`; assert configured copy renders.

## Acceptance criteria

- [ ] `/settings/security` does NOT display "Failed to fetch" in any state.
- [ ] On a tenant without SAML configured: renders "Not configured" with a Configure CTA.
- [ ] On a tenant with SAML configured: renders the configured state.
- [ ] Gateway endpoint returns a valid envelope OR the SPA handles the absence gracefully.
- [ ] Test pin passes.

## May-touch

- `web/app/(app)/settings/security/page.tsx`
- `web/lib/api/saml.ts` or equivalent SAML status hook
- `crates/argo-cli/src/handlers/gateway/routes_settings_saml_status_get.rs` (NEW, if endpoint missing)
- `web/app/(app)/settings/security/page.test.tsx` (NEW or extend)

## Out of scope

- Full SAML SSO implementation (separate briefs `mission-sso-saml-oidc-gateway`, `mission-saml-xml-signature-verify`)
- Settings D-top-tabs design (separate brief `fix-settings-page-implement-d-top-tabs-design`)
- Members hydration mismatch (separate brief `fix-settings-members-hydration-mismatch-unstyled`)

## Effort

~50-150 LOC (depending on whether the gateway endpoint needs creation). ~1-2 hours.
