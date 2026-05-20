# web-a11y-skip-to-content-link — global skip-to-content for keyboard + screen-reader users

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-a11y-skip-to-content-link`.
**Priority**: P2 — wave-2 a11y audit: `bypass` rule fails (no skip-link); keyboard users must Tab through every nav item before reaching content.
**Sizing**: ~80-150 LOC (component + insertion + per-page main-id wiring + tests).

## Why this brief exists

From the wave-2 a11y audit:

> Missing: skip-to-content link. WCAG 2.4.1 (Bypass Blocks) failure.
> Keyboard + screen-reader users must Tab through every nav item
> before reaching page content.

Standard pattern: visually-hidden anchor with `href="#main"` that
becomes visible on focus. Inserted as the first focusable element
in `<body>`. Every page's main content gets `id="main"` + `tabindex="-1"`.

## Goals

1. New `<SkipToContent>` component (visually hidden, focus-reveals).
2. Mount in `web/app/layout.tsx` so it's first focusable on every page.
3. Each route's main content gets `id="main"` and `tabindex="-1"` so the anchor target is real.
4. Brand-on visible state (Audiowide gradient pill on focus).

## Steps

1. Create `web/components/a11y/SkipToContent.tsx`.
2. Mount in root layout above `<AuthProvider>`.
3. For each route group (`(app)`, marketing pages, auth, hire, onboarding), wrap page content's outer `<main>` (or add one) with `id="main" tabindex="-1"`.
4. Test: Tab → skip-link visible → Enter → focus moves to main.

## may-touch

- New: `web/components/a11y/SkipToContent.tsx`
- `web/app/layout.tsx` (mount the skip-link)
- Each route group's layout.tsx (only to ensure a `<main id="main">` exists)
- `web/app/(app)/layout.tsx`, `web/app/auth/layout.tsx`, etc.
- New: `web/__tests__/skip-to-content.test.tsx`

## must-NOT-touch

- AppShell internal layout (which already has `<main>` — just ensure id="main" is set there)
- Per-page heading structures

## Acceptance criteria

- [ ] Skip link mounts on every page (visually hidden by default).
- [ ] First Tab from any page focuses the skip link (visibly).
- [ ] Pressing Enter on the link moves focus to the page's main content.
- [ ] axe-core scan reports 0 `bypass` violations.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md`
- WCAG 2.1 A 2.4.1 (Bypass Blocks)
