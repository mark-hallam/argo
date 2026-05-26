# fix-wizard-verticals-rewrite-scope-and-dead-fallback

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-wizard-verticals-rewrite-scope-and-dead-fallback`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/onboarding-hire-an-ai-teammate.md` F5; whole-site F10.

🟡 **P1 — `/onboarding/pick-vertical` shows 5 hardcoded SEED verticals instead of the ~30 the gateway has loaded, because `fetchVerticals` calls `/verticals` (bare path) which is NOT in `next.config.ts`'s rewrite list. The browser fetches the SPA HTML at `/verticals`, JSON-parse fails, the catch falls through to the 5-entry `SEED_VERTICALS`. A 7-entry `FALLBACK_VERTICALS` in the page is dead code (never reached because the seed always returns ≥1 row).**

## Root cause

Two compounding defects on the same path:

1. **Path mismatch.** `web/lib/api/verticals.ts:226-243` (`fetchVerticals`) requests `/verticals`. `web/next.config.ts:30-34` only rewrites `/api/v1/*` → gateway. So the SPA fetches `/verticals` against the Next dev origin (3000), gets the SPA's HTML 200 page back, `response.json()` fails on `<!doctype html>`, the catch returns `SEED_VERTICALS` (5 entries).

2. **Dead code.** `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158` declares a 7-entry `FALLBACK_VERTICALS` array (adds Recruiter + Marketplace Seller) for the "`useVerticals()` returned empty" branch. But `useVerticals` ALWAYS returns ≥1 row (the 5-entry SEED_VERTICALS), so this code path never runs. The 2 extra verticals (Recruiter + Marketplace Seller) are invisible to the operator even though both fallback arrays were intended to surface them.

3. **Gateway envelope mismatch (whole-site F10).** Live `curl http://127.0.0.1:7400/api/v1/verticals` returns `{"verticals":[]}` (zero loaded) while the legacy `/verticals` (no `/api/v1`) returns `{"loaded":[...], "errors":[], "ok":true}` with 30+ entries. Even if the SPA's path is fixed, the `/api/v1` envelope is empty for the SPA's resolved tenant.

Camera-visible result: wizard step 4 shows 5 cards labelled "v1 catalogue (offline · 5 verticals)" — implying the platform is offline, when in fact the gateway is alive and has 30+ verticals it can install.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P1] — Wizard offers 5 vertical templates instead of ~30 the gateway has; "offline" label is misleading; FALLBACK_VERTICALS is dead code
- whole-site-mvp-claims-coverage F10 [P1] — `/api/v1/verticals` returns empty `{"verticals":[]}` while the page renders 6 fixture verticals

## Fix recipe

Two options; pick (a) for the simplest cut.

**(a) Move the gateway endpoint into the `/api/v1` namespace and proxy through next.config (preferred — narrowest scope).**

1. Confirm or add a gateway route `GET /api/v1/verticals/catalog` returning `{verticals: [...], total_count: N}` (matching the legacy `/verticals` `loaded` array). Sibling route to existing `/api/v1/verticals/<id>`.
2. Update `fetchVerticals` in `web/lib/api/verticals.ts:226-243` to call `apiGet('/api/v1/verticals/catalog')` (which goes through the existing `/api/v1/*` rewrite).
3. Confirm response shape mapping (`loaded` → `verticals` if needed).
4. Delete the 7-entry `FALLBACK_VERTICALS` array at `web/app/(setup)/onboarding/pick-vertical/page.tsx:108-158`.
5. Drop the "offline · 5" label from the page header; the catalogue is online.

**(b) Extend `next.config.ts` to rewrite `/verticals` (and any other gateway-side legacy paths).**

1. Add `{ source: '/verticals', destination: 'http://127.0.0.1:7400/verticals' }` to `next.config.ts:30-34`.
2. Same step 4 + 5 as above.
3. Note: this widens the rewrite surface; not preferred for security review.

**Gateway-side fix (separate but coupled).** If `GET /api/v1/verticals` returns `{"verticals":[]}` for the SPA-resolved tenant, the boot-seed isn't materializing the catalog. Cross-reference brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md` for the upstream defect. The pick-vertical step depends on that seed being non-empty.

## Acceptance criteria

- [ ] `/onboarding/pick-vertical` renders ≥7 vertical cards (Recruiter + Marketplace Seller visible).
- [ ] Eyebrow no longer reads "v1 catalogue (offline · 5 verticals)" when the gateway is alive.
- [ ] DevTools network shows a successful `200` for the verticals catalog fetch through the Next proxy, with non-empty `verticals` array.
- [ ] The dead `FALLBACK_VERTICALS` array is deleted (or wired through a real `useVerticals.empty` branch).
- [ ] Vitest pin: when `useVerticals` returns 30 rows, the page renders 30 cards (or paginates if a max is set).

## May-touch

- `web/lib/api/verticals.ts` (fetch path)
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (delete dead fallback + header label)
- `web/next.config.ts` (option b only)
- `crates/argo-cli/src/handlers/gateway/routes_verticals*.rs` (add catalog endpoint if not present)

## Out of scope

- Gateway-side boot-seed for `/api/v1/verticals` returning empty (separate brief — see `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution.md`)
- Card name wrapping ("Finance — Accounts Receivable" wraps onto 3 lines) — separate brief `fix-pick-vertical-card-name-wrap-finance-ar.md`

## Effort

S — ~30-80 LOC. ~30-90 min.
