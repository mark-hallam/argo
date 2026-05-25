# mission-genesis-spa-page-describe-and-watch

**Owner**: claude.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-spa-page-describe-and-watch`.
**Depends on**: `mission-genesis-gateway-route-streaming-api` MERGED.

🔴 **P0 MISSION CLAIM 3 SPA SURFACE — the page that turns plain English into a worker.**

Per Agent-5 audit: no SPA page fires Genesis. Add the missing surface so the investor demo can show: operator types role → live progress strip → preview generated worker → "Hire" button → worker appears in `/workers`.

## Scope

New page at `web/app/(app)/genesis/page.tsx` (or `web/app/(setup)/genesis/page.tsx` depending on whether it's in app-shell or onboarding-shell — worker decides based on UX flow). Layout:

1. **Hero input**: large textarea with placeholder "Describe the role in plain English. e.g. 'A customer success lead who triages tier-1 tickets, drafts response templates, and escalates refunds over $500'." 48px+ button "Generate worker".
2. **Live progress strip** during streaming: 5 phase steps with checkmarks as SSE events arrive:
   - 🟡 Drafting persona
   - 🟡 Compiling charter
   - 🟡 Generating skill bundle
   - 🟡 Validating eval seeds
   - ⏳ Ready
3. **Preview card** (on vertical_ready event): persona name + role + 3 sample actions + 2 sample duties + charter excerpt. Big "Hire <name>" CTA.
4. **Hire flow**: clicking Hire fires `POST /api/v1/workers/hire` with the generated vertical's manifest → returns worker_id → SPA navigates to `/workers/{id}`.
5. **Error states**: SSE error event renders a clear message + "Try again" + "Modify description" affordances.

## Acceptance criteria

- [ ] `/genesis` route renders + accepts free-text input.
- [ ] On submit, SSE stream from `/api/v1/genesis` is consumed live; progress strip updates per phase.
- [ ] On `vertical_ready`, preview card renders with the generated content.
- [ ] Hire button creates a real worker + navigates to `/workers/{id}` C2-personality detail.
- [ ] Cross-tenant: foreign tenant cannot see the generated vertical or the new worker.
- [ ] Brand-loud register per memory `project_skycot_brand_identity` (Audiowide hero + violet/cyan gradient + warm radial-blob bg).
- [ ] Function-vs-dressing: the worker is REAL (real backend creation, real charter, real persona). No mock fallback on this route.

## May-touch

- `web/app/(app)/genesis/page.tsx` or `web/app/(setup)/genesis/page.tsx` (NEW)
- `web/lib/api/genesis.ts` (NEW — SSE client)
- `web/components/skycot/genesis/HeroInput.tsx` (NEW)
- `web/components/skycot/genesis/ProgressStrip.tsx` (NEW)
- `web/components/skycot/genesis/PreviewCard.tsx` (NEW)

## Out of scope

- Gateway route (separate brief — must merge first)
- Onboarding integration (separate brief)
- Marketplace publishing of the generated vertical (separate future brief)

## Effort

~400-700 LOC (page + SSE client + 3 components + brand styling + a Playwright/Vitest test). ~3-5 hours.
