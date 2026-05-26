# fix-docs-sub-routes-all-404

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-docs-sub-routes-all-404`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F1 [P0].

🔴 **P0 — `/docs` index advertises 10 sub-route cards (`/docs/getting-started`, `/docs/integrations`, `/docs/skills`, `/docs/workflows`, `/docs/api`, `/docs/audit`, `/docs/audit/trust-ratchet`, `/docs/getting-started/hire-first-teammate`, `/docs/integrations/slack-approvals`, `/docs/workflows/visual-canvas`). Every one of them returns HTTP 404. The investor / customer drill chain (`/landing` → "Docs" → article) is broken end-to-end.**

## Root cause

`~/code/argo/web/app/docs/` contains ONLY `page.tsx` — no sub-route directories. The 10 cards on the index page are `<Link>` elements pointing at routes that don't exist. Next.js returns 404 for all of them.

Verification:
- `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/docs` → 200
- `curl -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/docs/getting-started` → 404
- All 10 sub-routes confirmed 404 in the audit.

Per the function-vs-dressing rule: the camera in the POC demo doesn't follow these (the 6-scene runbook doesn't visit `/docs/*`), so it's not a hard demo blocker — but it IS a public-credibility break that any link recipient will hit. Operator may share the demo URL with an investor; investor opens a second tab, clicks Docs → article → 404 wall. That's exactly the credibility crack the audit flags.

Two policy options:
- (a) **Stub the 10 routes** with editorial-shell pages (each renders inside the docs shell, with placeholder content + a "request this article" CTA).
- (b) **Gate the 10 cards** behind a "Coming soon" overlay on the `/docs` index, link-stripping them until articles ship.

Option (a) is the cleaner long-term path; (b) ships in a single PR for the demo window.

## Findings this brief closes

- whole-site-mvp-claims-coverage F1 [P0] — All `/docs/*` sub-routes return 404

## Fix recipe

**Phase 1 — gate the cards (1-2 hours, ship for demo window)**:

1. On the `/docs` index page (`web/app/docs/page.tsx`), wrap each card link with a "Coming soon" badge for the 10 sub-routes that don't have a backing page yet.
2. Either disable the click (turn `<Link>` into a `<div>`) OR keep the link but route to a single `/docs/coming-soon` placeholder page.
3. Visual: greyed-out variant of the card + small "Coming soon" pill in the top-right corner.

**Phase 2 — stub the 10 articles (1-2 days, post-demo)**:

4. Create the directory tree:
   ```
   web/app/docs/
     layout.tsx                      (NEW — docs shell with breadcrumb + back-to-index)
     getting-started/
       page.tsx
       hire-first-teammate/
         page.tsx
     integrations/
       page.tsx
       slack-approvals/
         page.tsx
     skills/
       page.tsx
     workflows/
       page.tsx
       visual-canvas/
         page.tsx
     api/
       page.tsx
     audit/
       page.tsx
       trust-ratchet/
         page.tsx
   ```
5. Each `page.tsx` renders inside the docs shell with a `<h1>`, brief introduction paragraph, and a "request this article" CTA pointing at `mailto:hello@skycot.com?subject=Docs+request+<slug>`.
6. The actual article content can land later in MDX; the stubs make the chain navigable.

**Tests**:

7. Add a Playwright spec at `web/playwright/docs/all-sub-routes-resolve.spec.ts`:
   - For each of the 10 documented sub-routes, visit and assert HTTP 200.
   - Assert each renders a `<h1>` matching the card's title.

## Acceptance criteria

- [ ] Either (a) all 10 `/docs/*` sub-routes return 200 with stub article content, OR (b) the 10 cards on the `/docs` index are explicitly badged "Coming soon" and don't navigate anywhere broken.
- [ ] `curl` against each of the 10 routes either returns 200 OR is no longer linked from `/docs`.
- [ ] New Playwright spec passes.
- [ ] `/docs` index continues to render cleanly.

## May-touch

- `web/app/docs/page.tsx` (index — gate the cards if Phase 1)
- `web/app/docs/layout.tsx` (NEW if Phase 2)
- 10 new `page.tsx` files under `web/app/docs/<slug>/` (if Phase 2)
- `web/playwright/docs/all-sub-routes-resolve.spec.ts` (NEW)

## Out of scope

- `/docs` missing h1 (covered by `fix-docs-page-missing-h1-heading-hierarchy`).
- Real article content — placeholder stubs are sufficient.
- Translating docs into other languages.

## Effort

Phase 1: ~50-100 LOC, ~1-2 hours. Phase 2: ~500-800 LOC, ~1-2 days. P0 for Phase 1 (ship before any external demo link), P2 for Phase 2.
