# fix-landing-hero-mobile-stack-and-tablet-nav

**Owner**: codex or claude.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-landing-hero-mobile-stack-and-tablet-nav`.
**Source**: ux-e2e-audit-T2309MEL `whole-site-mvp-claims-coverage.md` F26 [P2] + F27 [P3].

🟢 **P2 — `/landing` at 375×812 (iPhone SE / standard mobile) renders "Hire your first teammate" + "See how it works" CTAs side-by-side at ~50% width each. Both wrap text to 2 lines and look cramped. At 768 (tablet) the primary nav (How it works / Features / Compare / Roster / Pricing / Docs / Changelog) collapses behind a hamburger — reasonable but conservative; the 7 items fit at 768 with a smaller font.**

## Root cause

The `/landing` hero CTA pair is probably using a Tailwind flex-row layout that doesn't break to flex-col below a certain viewport width. At 375px viewport, the two CTAs share ~344px (after padding) — each gets ~160px which forces 2-line wrapping on both labels.

The tablet nav breakpoint is probably tuned for a "narrow tablet" / "lightweight chrome" use case but the 7-item nav probably fits at 768 with the right font-size — verify against current Inter sizing.

## Findings this brief closes

- whole-site-mvp-claims-coverage F26 [P2] — Mobile hero CTAs stack tightly side-by-side at 375 width
- whole-site-mvp-claims-coverage F27 [P3] — Tablet (768) collapses to hamburger nav

## Fix recipe

### Mobile CTA stack (F26)

1. **Find the hero CTA pair** in `web/app/landing/page.tsx`. Look for the `<div>` wrapping both "Hire your first teammate" + "See how it works" buttons.

2. **Add a breakpoint stack.** Replace the current flex utility with:
   ```tsx
   className="flex flex-col gap-3 sm:flex-row sm:gap-4"
   ```
   (or whatever Tailwind / brand-token equivalent the project uses). At ≤640px the buttons stack vertically; ≥640px they sit side-by-side.

3. **Verify button min-width**. Each button at full-width on mobile should display its label on a SINGLE line. Check at 320px, 375px, 414px viewports.

### Tablet nav (F27)

1. **Measure current breakpoint.** Find the AppShell / marketing header (`web/components/skycot/marketing/MarketingHeader.tsx` or equivalent) and check the `md:` / `lg:` breakpoint at which the hamburger appears vs full nav.

2. **Test fit at 768.** Render the full nav (7 items) at 768 with the current Inter sizing. If it fits without overflow, bump the breakpoint down (e.g. `lg:` → `md:`). If it doesn't fit, reduce the nav font-size at the 768-1024 range OR keep the hamburger (this is P3 — operator polish call).

3. **Visual check at standard tablet sizes**: 768, 820, 1024.

## Acceptance criteria

- [ ] `/landing` at 375 width: hero CTAs stack vertically, each on a single label line
- [ ] `/landing` at 414 width: same — single-line labels, stacked
- [ ] `/landing` at 640+ width: CTAs side-by-side (unchanged from current)
- [ ] `/landing` at 768 width (tablet): EITHER full nav fits without overflow OR hamburger persists with explicit operator-OK
- [ ] `pnpm --dir web exec next build` green
- [ ] Visual smoke: open `/landing` in Chrome dev tools mobile emulator at 375 / 414 / 640 / 768 / 1024 / 1440

## May-touch

- `web/app/landing/page.tsx` (~5 LOC hero CTA wrapper class)
- `web/components/skycot/marketing/MarketingHeader.tsx` (~5 LOC breakpoint adjustment, if F27 fix taken)
- Vitest viewport spec (optional, ~30 LOC)

## Out of scope

- Marketing CTA copy (covered by `fix-marketing-cta-teammate-vs-worker-vocab-drift`)
- Onboarding wizard mobile (covered by `fix-onboarding-wizard-mobile-viewport-top-bar-clipping`)
- App-shell wide-viewport (`fix-app-shell-wide-viewport-content-rail-1920`)
- Dark mode on marketing pages (`fix-dark-mode-tokens-pricing-compare-docs-marketplace`)

## Effort

XS–S. ~10 LOC class edits. ~20 min. P2 polish — the v1.0 demo MP4 records on desktop, so mobile breakage only affects investor side-tab clicks from their phone.
