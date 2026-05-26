# fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F5 [P1] + F6 [P1].

🟠 **P1 — Pick-vertical step shows hardcoded skill counts that don't match what the gateway installs. The wizard says marketplace-seller has 6 skills; gateway installs 2. Catalogue is client-side static; gateway `/api/v1/verticals` returns empty.**

## Root cause

`web/app/(setup)/onboarding/pick-vertical/page.tsx:24-74` hardcodes the 7-card vertical list including `skillCount: 5|6` per card. The gateway's `/api/v1/onboarding/install-vertical` response returns the actual count (`skills_added`). Live probe shows the card numbers disagree with the gateway's truth — most dramatically `marketplace-seller`: card says 6, gateway installs 2.

Underlying root cause: there is no gateway-side catalogue endpoint feeding the wizard. `/api/v1/verticals` returns `{"verticals":[]}` — the wizard cannot ask the platform what it knows, so it duplicates the catalogue in client code and drifts.

## DOM evidence

| Vertical | Card says | Gateway installs |
|---|---|---|
| gtm-engineer | 5 | 8 |
| marketing-intern | 5 | 5 |
| marketplace-seller | 6 | **2** |
| recruiter | 6 | 5 |
| sales-sdr | 5 | 5 |
| customer-support | 5 | 5 |
| finance-ar | 4 | 5 |

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P1] — Skill counts on vertical cards are hardcoded and don't match the gateway
- onboarding-hire-an-ai-teammate F6 [P1] — `/api/v1/verticals` returns empty; wizard pick-vertical is decoupled from gateway truth

## Fix recipe

1. **Populate `/api/v1/verticals`** from the registered vertical packs. Endpoint returns:
   ```json
   {
     "verticals": [
       {"id":"sales-sdr","display_name":"Sales SDR","skill_count":5,"workflow_count":2,"tagline":"...","color":"..."}
     ]
   }
   ```
   File: `crates/argo-cli/src/handlers/gateway/routes_verticals.rs` (verify path).
2. **Refactor `web/app/(setup)/onboarding/pick-vertical/page.tsx`** to fetch from the gateway via `useVerticals()` hook (new). Keep the hardcoded array as a fallback for the mock-offline path (`isDemoMode() && fetched.length === 0`).
3. **Tighten the type** in `web/lib/api/verticals.ts` (NEW or extend) so the card render uses `skill_count` + `workflow_count` from the gateway when available.
4. **Persist `verticalDisplayName` + `verticalSkillCount` to the draft store** so step 5 can render contextualised copy (separate brief `fix-onboarding-step5-vertical-name-and-install-summary-context` consumes this).
5. **Add a banner** in step 4 when gateway returns >0 verticals and the count differs from the seed: just trust the gateway truth. Until the catalogue is populated, render the seed with a small "v1 catalogue · 7 verticals" footer pill so the camera reads honest.

## Acceptance criteria

- [ ] `/api/v1/verticals` returns a non-empty list reflecting registered packs.
- [ ] `/onboarding/pick-vertical` fetches from the gateway and renders cards from the response.
- [ ] The displayed skill count on each card equals `skills_added` from the subsequent install-vertical response (parity test).
- [ ] Mock-offline fallback renders the existing hardcoded 7 cards.
- [ ] Web smoke test asserts pick-vertical renders without 500.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_verticals.rs` (populate)
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (fetch + render)
- `web/lib/api/verticals.ts` (NEW)
- `web/lib/onboarding/use-onboarding-draft.ts` (persist display name + skill count)
- `web/tests/onboarding-pick-vertical.test.ts` (NEW or extend)

## Out of scope

- Pack-load discovery (separate brief: `mission-gateway-add-api-v1-verticals-cors-mounted-discovery`)
- Step 5 hero copy contextualisation (separate brief: `fix-onboarding-step5-vertical-name-and-install-summary-context`)
- Vertical templates editing UI (post-MVP)

## Effort

~120-180 LOC + 1 gateway endpoint + 1 hook + 1 test. ~3-4 hours.
