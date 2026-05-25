# fix-marketing-nav-fragment-anchors-and-dead-cta

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-nav-fragment-anchors-and-dead-cta`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) findings F9 + F11 + F14 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1/P2 — Marketing nav anchors (`#how`, `#features`) are relative-hash links that don't work from `/compare` or `/customers`. "Watch a 90-second demo" CTA has no destination wired. Hero teammate cards overlap with the SKYCOT logo block at 1440 width.**

## Root cause

**F14** (P2): `/landing` marketing nav links: `How it works` → `#how`, `Features` → `#features`. Pages `/customers` and `/compare` don't include those anchors. Clicking `How it works` from `/compare` won't jump correctly — it'll navigate to `/compare#how` then no-op.

Touch site: `web/components/marketing/MarketingNav.tsx`.

**F11** (P2): `/landing` shows two hero CTAs: "Hire your first teammate" → `/auth/signup` (works), and "Watch a 90-second demo" — clicking it does nothing visible (no modal, no scroll to a video). Public-facing dead link.

**F9** (P1): On the root marketing page at 1440 width, the floating worker-summary cards sit ON TOP of a gradient SKYCOT-logo block in the upper-right. The center card (`Jamal: 12 tickets resolved...`) is partly occluded by the logo block. z-index / positioning bug between two absolutely-positioned hero ornaments.

Combined as one brief because all three are marketing-surface polish on `/landing` + nav.

## Findings this brief closes

- whole-site-mvp-claims-coverage F9 [P1] — Hero teammate cards overlap with SKYCOT logo block on `/` at 1440
- whole-site-mvp-claims-coverage F11 [P2] — Marketing CTA "Watch a 90-second demo" has no destination wired
- whole-site-mvp-claims-coverage F14 [P2] — Marketing nav anchors are relative-hash links that don't work from `/compare` or `/customers`

## Fix recipe

**For F14**:
1. Touch site: `web/components/marketing/MarketingNav.tsx`.
2. Change anchor hrefs from `#how` / `#features` to absolute `/landing#how` / `/landing#features`.
3. Verify the target IDs (`<section id="how">`, `<section id="features">`) exist on `/landing`.

**For F11**:
- Pick ONE:
  - (a) Hide the CTA entirely until a real demo MP4 exists.
  - (b) Wire it to a placeholder modal: "Demo coming soon — book a 1:1" + Calendly link.
  - (c) Replace with "See how it works" → scrolls to `#how` section.
- Touch site: `web/app/landing/page.tsx`.
- Recommend (c) for a real action that doesn't lie.

**For F9**:
1. Touch site: `web/app/landing/page.tsx` + `web/app/landing/page.css` (or styles file).
2. Inspect the layered ornaments at 1440 width. Three options:
   - (a) Nudge the SKYCOT logo block's z-index below the worker cards.
   - (b) Offset the cards' grid columns so they don't share the same right edge.
   - (c) Shrink the SKYCOT logo block to occupy less right-side space.
3. Test at multiple widths: 1280, 1440, 1920, 2560. The fix should hold across all.

**Pin**:
- `web/tests/marketing-nav-anchors.spec.ts`: navigate `/compare`, click "How it works" in nav, assert URL becomes `/landing#how` AND the `#how` section is scrolled into view.
- Visual regression: capture `/landing` at 1440 width, assert no overlap.

## Acceptance criteria

- [ ] Marketing nav anchors work from any marketing page (absolute paths to `/landing`).
- [ ] "Watch a 90-second demo" CTA either does something real OR is removed.
- [ ] No visible overlap between hero teammate cards and the SKYCOT logo block at 1280/1440/1920/2560.
- [ ] Playwright spec passes.

## May-touch

- `web/components/marketing/MarketingNav.tsx`
- `web/app/landing/page.tsx`
- `web/app/landing/page.css`
- `web/tests/marketing-nav-anchors.spec.ts` (NEW)

## Out of scope

- Marketing theme drift (separate brief `fix-marketing-pages-theme-drift-landing-vs-compare`)
- Landing-page customer logo fabrication (separate brief `fix-landing-page-fictitious-customer-logos-and-testimonials`)
- Footer/legal links validation (separate task)

## Effort

~50-120 LOC + 1 Playwright spec. ~2 hours.
