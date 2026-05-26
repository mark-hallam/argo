# fix-onboarding-provision-demo-mode-bypass

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-provision-demo-mode-bypass`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F5 [P0].

🔴 **P0 INVESTOR-DEMO-BLOCKER — In demo mode, clicking Continue on the wizard's Step 2 (`/onboarding/workspace`) renders, inside the form card, in red: `X-Argo-Principal header must carry a verified human:<email> value`. The investor narrative `landing → "Hire your first AI teammate" → wizard → working teammate in four minutes` cannot be walked end-to-end.**

## Root cause

The demo-mode top banner ("Demo mode — showing sample data. Sign in to see your own workspace.") implies that no sign-in is required and the wizard will work without credentials. But Step 2's Continue button fires `POST /api/v1/onboarding/provision` directly against the live gateway, and that route requires `X-Argo-Principal: human:<email>` because it actually creates a tenant + workspace row. The SPA does not short-circuit this call in demo mode, and the gateway's error envelope is rendered verbatim as the form-validation message.

Two layered failures:

1. **No demo-mode bypass on `/onboarding/provision`** — `web/lib/api/onboarding.ts` (the function backing the Continue button on `/onboarding/workspace`) calls `apiPost("/api/v1/onboarding/provision", ...)` without checking `isDemoMode()` first. Compare to other endpoints that have a sample-data fallback path when demo mode is on.
2. **Raw protocol-detail leak in the error copy** — `web/app/(setup)/onboarding/workspace/page.tsx` displays whatever string the gateway returns in the error envelope inside the card. The header name `X-Argo-Principal` is internal protocol detail that should never reach a human.

Reproduction (Chrome walk, 2026-05-26 20:07): navigate to `/landing` → click "Hire your first AI teammate" → arrive at `/auth/signup` (demo-mint), proceed to `/onboarding/welcome`, click Continue, arrive at `/onboarding/workspace`. Fill workspace="Acmeco Growth" + company="Acmeco Inc." + click Continue → red error inside the form card with the verbatim string above. Wizard cannot advance.

## Findings this brief closes

- chrome-live-walk-and-playwright F5 [P0] — Onboarding wizard Step 2 → Continue surfaces raw gateway error
- chrome-live-walk-and-playwright F5a [P0] — Demo mode bypass not propagated to wizard Step 2
- chrome-live-walk-and-playwright F5b [P0] — Error copy leaks `X-Argo-Principal` header name

## Fix recipe

1. **Add a demo-mode short-circuit branch in the provision client** (`web/lib/api/onboarding.ts` — locate the function called by the workspace-step Continue handler). When `isDemoMode()` returns true:
   - Skip the gateway round-trip
   - Generate a stable in-memory tenant_id from the workspace name (deterministic hash or UUID v7)
   - Persist to `sessionStorage` so subsequent wizard steps can read the demo tenant id
   - Return a synthetic success envelope shaped identically to the gateway's `{ tenant_id, workspace_name, company_name }` response

2. **Wrap the error envelope** so internal protocol detail can never leak into the form. In `web/app/(setup)/onboarding/workspace/page.tsx` (the Continue handler's catch block), replace direct render of `err.message` with:
   - If the error contains `X-Argo-Principal` or `principal` or any internal-protocol-name substring → render generic "We can't save this workspace in demo mode — sign in to continue" plus a Sign-in CTA
   - Otherwise render a generic "Something went wrong. Try again."

3. **Add a Playwright spec** at `web/playwright/onboarding/demo-mode-step-2-continue.spec.ts`:
   - Run with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`
   - Navigate the full wizard from `/auth/signup` (demo-mint) through Step 2 click-Continue
   - Assert URL advances to `/onboarding/connect-provider`
   - Assert no element on the page contains the string `X-Argo-Principal`

4. **Audit the other wizard steps** for the same failure mode. `connect-provider`, `pick-vertical`, `first-hire` likely each fire a gateway POST that needs the same demo-mode branch. Apply the same pattern to each.

## Acceptance criteria

- [ ] With `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, the wizard advances all 5 steps without any gateway-error banner.
- [ ] If the gateway DOES return an error, the rendered string never contains `X-Argo-Principal` or any other internal header name.
- [ ] New Playwright spec passes.
- [ ] An investor visiting `/landing` and walking through to "Hire" can reach `/workers/{id}` without seeing a red error banner anywhere.

## May-touch

- `web/lib/api/onboarding.ts` (add `isDemoMode()` branches)
- `web/app/(setup)/onboarding/workspace/page.tsx` (error wrapper)
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (audit + fix)
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (audit + fix)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (audit + fix)
- `web/playwright/onboarding/demo-mode-step-2-continue.spec.ts` (NEW)

## Out of scope

- Real provisioning behaviour when signed in (unchanged)
- Genesis invocation on first-hire (covered by `fix-onboarding-first-hire-invoke-genesis-plan`)
- Tenant resolver mismatch (covered by `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`)

## Effort

~100-200 LOC + 1 new spec. ~2-4 hours focused. P0 because this single bug makes the investor-pitch demo not walkable end-to-end in demo mode.
