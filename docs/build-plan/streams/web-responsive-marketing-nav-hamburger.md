# web-responsive-marketing-nav-hamburger — hamburger nav for marketing pages below 900px

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-responsive-marketing-nav-hamburger`.
**Priority**: P2 — wave-2 responsive: marketing top-nav links disappear at <900px with NO hamburger replacement. A procurement reviewer on a phone is stuck.
**Sizing**: ~200-350 LOC (shared MarketingNav component + hamburger + slide-out sheet + tests).

## Why this brief exists

From the wave-2 responsive audit:

> Marketing nav links disappear at <900px on /landing/customers/pricing
> with **no hamburger replacement**. A procurement reviewer on a phone
> is stuck — can't navigate between marketing pages without typing URLs.

The authed (app) shell already has NavDrawer (works correctly on mobile).
Marketing pages have their own bespoke top-nav (Audiowide wordmark +
right-side link list) that simply hides links below 900px.

## Goals

1. New `<MarketingNav>` shared component used by /landing, /compare, /pricing, /customers, /changelog, /docs, /privacy, /terms, /security.
2. Desktop (≥900px): existing wordmark + link row.
3. Mobile (<900px): wordmark + hamburger icon that opens a slide-out sheet with the same links + close affordance.
4. Active-page indicator (the existing pattern, just moved into the sheet on mobile).
5. A11y: focus trap + Esc closes + first-link-focus on open.

## Steps

1. Inventory current marketing nav callsites: `rg 'Pricing.*GitHub.*Docs' web/app/` — they appear inline in each marketing page footer.
2. Extract the top + footer nav into `web/components/marketing/MarketingNav.tsx` + `MarketingFooter.tsx`.
3. Add hamburger + slide-out (use existing Radix Dialog or Sheet primitive if available).
4. Wire active-page detection via `usePathname`.
5. Test: desktop renders link row; mobile renders hamburger; click → sheet opens; Esc → closes.

## may-touch

- New: `web/components/marketing/MarketingNav.tsx`
- New: `web/components/marketing/MarketingFooter.tsx`
- Each marketing page replaces inline nav with `<MarketingNav />`
- Optional: `web/components/marketing/MobileNavSheet.tsx`

## must-NOT-touch

- AppShell (authed product nav — unrelated)
- Marketing page heroes / content (only nav extraction)

## Acceptance criteria

- [ ] At 375px / 768px, hamburger renders + works.
- [ ] At 900px+, link row renders.
- [ ] Active page highlighted in both desktop + mobile modes.
- [ ] Esc closes the mobile sheet; focus trap works; first link focused on open.
- [ ] All 9 marketing pages use the shared component (zero inline nav remaining).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/responsive/2026-05-20-wave-2-responsive.md`
- Pattern: `web/components/ui/NavDrawer.tsx` (the authed product's nav drawer; reuse the bottom-sheet pattern)
