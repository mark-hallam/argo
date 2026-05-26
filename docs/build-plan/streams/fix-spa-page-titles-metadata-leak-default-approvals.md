# fix-spa-page-titles-metadata-leak-default-approvals

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-page-titles-metadata-leak-default-approvals`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/whole-site-mvp-claims-coverage.md` F08 [P1] + F09 [P1] + F10 [P1] + F17 [P2].

🟠 **P1 — Multiple in-app pages leak `<title>Approvals · SKYCOT</title>` because they're client-components that can't export `metadata`. `/brand`, `/verticals`, `/orchestration`, `/retro`, `/billing` all show "Approvals" in the browser tab. `/maintenance` and `/sla` fall through to the root marketing default. `/auth/signup` uses a marketing-flavoured title while signin and verify use product titles.**

## Root cause

`web/app/(app)/layout.tsx:18-23` sets `title: { default: "Approvals", template: "%s · SKYCOT" }`. Pages declared with `"use client"` cannot export `metadata` per Next.js rules, so they inherit the default. The current state:

- `/brand` → `<title>Approvals · SKYCOT</title>` (wrong; should be Brand)
- `/verticals` → `<title>Approvals · SKYCOT</title>`
- `/orchestration` → `<title>Approvals · SKYCOT</title>`
- `/retro` → `<title>Approvals · SKYCOT</title>`
- `/billing` → `<title>Approvals · SKYCOT</title>`
- `/maintenance` → `<title>SKYCOT — Hire your first AI teammate</title>` (root default; outside (app) layout)
- `/sla` → `<title>SKYCOT — Hire your first AI teammate</title>` (same)
- `/auth/signup` → `<title>Hire your first AI teammate · SKYCOT</title>` (marketing-flavoured)
- `/auth/signin` → `<title>Sign in · SKYCOT</title>` (product-flavoured)
- `/auth/verify` → `<title>Verifying your magic link · SKYCOT</title>` (product-flavoured)

Two distinct mitigations needed: page-shim wrapper for the client pages, and explicit `metadata` for the marketing pages.

## Findings this brief closes

- whole-site-mvp-claims-coverage F08 [P1] — multiple in-app pages leak default "Approvals · SKYCOT"
- whole-site-mvp-claims-coverage F09 [P1] — `/auth/signup` title inconsistent with signin / verify
- whole-site-mvp-claims-coverage F10 [P1] — `/maintenance` + `/sla` fall through to root default
- whole-site-mvp-claims-coverage F17 [P2] — `/brand` leaks (sub-case of F08)

## Fix recipe

1. **For each client-component page** that needs a title, split into a server-component shim + a `*Client.tsx` child:
   ```tsx
   // web/app/(app)/brand/page.tsx (server component, NO "use client")
   import { BrandClient } from "./BrandClient";
   export const metadata = { title: "Brand" };
   export default function BrandPage() { return <BrandClient />; }
   ```
   Move the existing `"use client"` body into `BrandClient.tsx`.
2. **Apply to**: `/brand`, `/verticals`, `/orchestration`, `/retro`, `/billing`.
3. **For `/maintenance` and `/sla`**: declare `export const metadata = { title: "Maintenance · SKYCOT" }` (and same for `/sla`) at the page level.
4. **Decide auth title direction**: align `/auth/signup` to product-flavoured ("Sign up · SKYCOT") OR align all three to marketing-flavoured. Operator preference: product-flavoured for consistency with `signin` and `verify`. Update `web/app/auth/signup/page.tsx` metadata.
5. **Add a smoke test** that curls each page and asserts the `<title>` element contains the expected page-specific phrase.

## Acceptance criteria

- [ ] `/brand`, `/verticals`, `/orchestration`, `/retro`, `/billing` all carry page-specific titles (Brand / Verticals / Orchestration / Retro / Billing).
- [ ] `/maintenance` shows "Maintenance · SKYCOT"; `/sla` shows "SLA · SKYCOT".
- [ ] `/auth/signup` title aligns with signin/verify (product-flavoured).
- [ ] Web smoke test asserts each page's title.
- [ ] No regression on `/`, `/team`, `/workers`, etc.

## May-touch

- `web/app/(app)/brand/page.tsx` + `BrandClient.tsx` (NEW split)
- `web/app/(app)/verticals/page.tsx` + `VerticalsClient.tsx`
- `web/app/(app)/orchestration/page.tsx` + `OrchestrationClient.tsx`
- `web/app/(app)/retro/page.tsx` + `RetroClient.tsx`
- `web/app/(app)/billing/page.tsx` + `BillingClient.tsx`
- `web/app/maintenance/page.tsx` (add metadata)
- `web/app/sla/page.tsx` (add metadata)
- `web/app/auth/signup/page.tsx` (update metadata)
- `web/tests/page-titles-smoke.test.ts` (NEW)

## Out of scope

- Open-Graph card metadata (separate task; share cards are different from `<title>`)
- Marketing-page SEO sweep (separate brief for `/landing`, `/pricing`, etc.)

## Effort

~5 shim splits × ~15-25 LOC each + 3 metadata-only edits + smoke test. ~150-200 LOC total. ~2-3 hours mechanical.
