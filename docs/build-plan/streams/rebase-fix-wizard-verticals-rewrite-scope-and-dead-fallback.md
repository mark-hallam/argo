# rebase-fix-wizard-verticals-rewrite-scope-and-dead-fallback

Authored against: origin/main `0c7cff50f5` on 2026-05-29
Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-wizard-verticals-rewrite-scope-and-dead-fallback

## Why this brief exists

PR #2502 (214 LOC, 2026-05-27) targeted F5 [P1] + whole-site F10 [P1]: `web/lib/api/verticals.ts::fetchVerticals()` calls bare `/verticals` which is NOT proxied through `web/next.config.ts` (which only rewrites `/api/v1/*`). In dev, the SPA hits its own origin, gets an HTML 200, JSON-parse raises, catch returns the 5-entry `SEED_VERTICALS`. Camera-visible result: wizard step 4 reads "v1 catalogue (offline · 5 verticals)" — implying the platform is offline when in fact the gateway is alive with 30+ packs loaded.

Original PR's approach: add new `GET /api/v1/verticals/catalog` gateway endpoint + update SPA to call it via the existing `/api/v1/*` rewrite + delete dead `FALLBACK_VERTICALS` const + drop "offline · N" eyebrow qualifier.

The bug is STILL load-bearing — verified:
- `web/lib/api/verticals.ts:231` still calls `apiGet<VerticalsEnvelope>("/verticals")` (bare path, not proxied).
- `web/next.config.ts:36-40` rewrites only `/api/v1/:path*`.

But context has shifted: **#2456** already addressed an adjacent gateway-side bug — `/api/v1/verticals` now reads the on-disk pack source directly (via `argo_vertical::load_dir`) and returns `{verticals: [...]}` with the canonical SPA row shape. So adding a NEW `/api/v1/verticals/catalog` route may now be redundant — the SPA could simply call the existing `/api/v1/verticals` (which IS proxied) and get the catalog directly.

**Simpler re-implementation path**: change `fetchVerticals` to call `/api/v1/verticals` (currently the path used elsewhere in the file for per-vertical detail), drop the bare `/verticals` route, delete the dead `FALLBACK_VERTICALS` const, drop the "offline · N" eyebrow. No new gateway endpoint required. Smaller surface, smaller LOC.

The previously-locked semantic split (legacy `/verticals` = catalog vs `/api/v1/verticals` = per-tenant install) collapsed after #2456 — both now read from `load_dir`, so the SPA can use either, and the rewrite-only path is correct.

PR #2502 was closed for re-author.

## Source of truth

- Original brief: `docs/build-plan/streams/fix-wizard-verticals-rewrite-scope-and-dead-fallback.md`
- Closed PR: #2502
- Current main shape:
  - `web/lib/api/verticals.ts:226-242` (bare `/verticals` call, dev fallback to SEED_VERTICALS).
  - `web/next.config.ts:36-40` (only `/api/v1/:path*` proxied).
  - `crates/argo-cli/src/handlers/gateway/routes_verticals.rs` — after #2456, `/api/v1/verticals` reads pack catalog from disk.

## Acceptance

Same as original brief F5 + F10 acceptance:
- `/onboarding/pick-vertical` renders one card per gateway-loaded vertical (≥7 once Recruiter + Marketplace Seller packs are loaded).
- Eyebrow no longer reads "offline · N verticals" when the gateway is alive.
- `fetchVerticals` requests a path that IS proxied through the Next rewrite (`/api/v1/*`).
- Dead `FALLBACK_VERTICALS` const + branch deleted.
- Vitest pin asserts gateway-fed N-row catalogue + correct eyebrow string.

**Recommended path: change `fetchVerticals` to call `/api/v1/verticals`** (existing route, post-#2456 reads pack catalog directly). No new gateway endpoint needed. If integration with `workflow_definition_index` for tenant-install view is desired later, that's a separate concern.

## Boundary

- May touch: `web/lib/api/verticals.ts`, `web/app/(setup)/onboarding/pick-vertical/page.tsx`, the test files in the same directory.
- May skip the gateway-side changes from the original brief (option a) since #2456 already wires `/api/v1/verticals` to the pack catalog.

## Verification

- `npx vitest run app/(setup)/onboarding/pick-vertical`
- `npx tsc --noEmit`
- Manual: `curl http://127.0.0.1:7400/api/v1/verticals` returns ≥7 verticals (verified per #2456 acceptance).
