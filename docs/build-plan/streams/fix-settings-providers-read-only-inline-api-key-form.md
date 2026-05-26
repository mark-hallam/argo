# fix-settings-providers-read-only-inline-api-key-form

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-providers-read-only-inline-api-key-form`.
**Source**: ux-e2e-audit-T2309MEL `whole-site-mvp-claims-coverage.md` F16 [P1].

🟠 **P1 — `/settings/providers` is read-only. It shows "Primary provider: Anthropic / Fallback: Fail closed / Per-worker overrides: Audit logged" + a small "Review API keys" link routing to `/settings/api-keys`. There's no inline form to add or rotate a provider key. For the v1.0 enterprise positioning (`project_v1_strategic_decisions_2026_05_14`), operators need to swap from the Free-tier mock provider to their own Anthropic/OpenAI key BEFORE workers run real work. The current page makes that step a navigation away.**

## Root cause

`web/app/(app)/settings/providers/page.tsx` (verify exact path) renders provider configuration as a static readout. The actual editing flow lives at `/settings/api-keys`. So an operator on `/settings/providers` who wants to plug in their Anthropic key has to:

1. Read the provider readout
2. Click "Review API keys"
3. Land on a different page
4. Find their provider in a list
5. Add the key

That's 4 unnecessary clicks for the load-bearing setup action. Per the v1.0 enterprise GTM, the "I have a key, let me use it" path should be ONE click from `/settings/providers`.

The audit also notes: this affects Claim 3 (Platform builds workers) visibility — a buyer finishing the onboarding wizard with `provider=mock` (per `fix-onboarding-connect-provider-skip-mock-warning-callout`) needs an obvious in-product surface to upgrade later, and `/settings/providers` is the natural place.

## Findings this brief closes

- whole-site-mvp-claims-coverage F16 [P1] — `/settings/providers` is read-only; no inline add/rotate

## Fix recipe

1. **Inline the API-key form on `/settings/providers`.** Add a collapsible section per supported provider (Anthropic, OpenAI, Google, etc. — match the existing `/settings/api-keys` provider list):
   - Each row: provider name + status pill (configured / mock) + masked key preview if set + "Add key" or "Rotate" button
   - Clicking "Add key" expands an inline form: input field + Save + Cancel
   - Save calls the existing endpoint (`POST /api/v1/settings/provider-keys` or whatever `/settings/api-keys` uses today)
   - Success → status pill flips to "configured", masked preview updates, toast confirms

2. **Reuse existing components.** The `/settings/api-keys` page already has the form logic. Extract its core form into a shared `<ProviderKeyForm provider="anthropic" />` component, then mount instances inline on `/settings/providers`.

3. **Keep `/settings/api-keys` as the deep page** (for now). Don't delete it; it's still useful as the cross-provider list view. Add a "Manage all keys" link from `/settings/providers` to `/settings/api-keys` for parity.

4. **Audit row on save.** Per the project's audit-first claim, every key add/rotate writes an audit row (`provider.key.added` or `provider.key.rotated`) with the provider name + (last-4-of-key, masked) in the metadata. Verify the existing backend already does this; if not, add the audit emission.

5. **Vitest.** `web/app/(app)/settings/providers/__tests__/page.test.tsx`:
   - Renders with no keys configured → all providers show "Mock provider" status
   - Click "Add key" → form appears inline
   - Mock POST → success → status flips to "configured"

## Acceptance criteria

- [ ] `/settings/providers` shows an inline "Add key" or "Rotate" button next to each provider
- [ ] Clicking the button reveals an inline form on the same page (no navigation)
- [ ] Saving a key updates the page state + writes an audit row
- [ ] `/settings/api-keys` still works as a separate page (no regression)
- [ ] Vitest spec passes
- [ ] `pnpm --dir web exec tsc --noEmit` green
- [ ] Manual e2e: sign in, navigate to /settings/providers, add a fake Anthropic key, see it accepted + audit row visible on /audit

## May-touch

- `web/app/(app)/settings/providers/page.tsx` (~50 LOC additions for inline forms)
- `web/components/skycot/settings/ProviderKeyForm.tsx` (~80 LOC new shared component if extracted)
- `web/app/(app)/settings/api-keys/page.tsx` (~20 LOC refactor to consume shared component)
- `web/lib/api/provider-keys.ts` (verify exists; ~10 LOC if extending)
- `web/app/(app)/settings/providers/__tests__/page.test.tsx` (~60 LOC test)

## Out of scope

- Mock provider banner anywhere in the app shell (covered by `fix-mock-provider-persistent-banner-in-app-shell`)
- Onboarding-skip-mock-warning (covered by `fix-onboarding-connect-provider-skip-mock-warning-callout`)
- Per-worker provider override UI (the readout already says "Per-worker overrides: Audit logged" — that's a separate surface)
- Refactor of the locked Settings D-tabs design (`fix-settings-page-implement-d-top-tabs-design`)

## Effort

M. ~160 LOC + ~60 LOC test. ~2 hours. P1 because the load-bearing setup action (plug-in-your-own-key) is gated behind a multi-click trail.
