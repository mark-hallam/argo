# mission-auth-pages-brand-mark — add SKYCOT logo + value-prop to auth pages (P2)

**Owner**: claude. **Sizing hint**: visual polish, no logic.
**Depends on**: none.
**Authored against**: origin/main `91f9d032` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-auth-pages-brand-mark`.
**Source**: 2026-05-18 browser UX audit — `/auth/signup` and `/auth/check-email` render a small form card alone in a vast empty page at 1920px. No SKYCOT logo, no brand context, no value reminder. Onboarding pages have the logo + progress dots; auth pages are missing both.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — auth is the first hands-on interaction. Brand mark + value prop sets confidence.

## Scope

P2. Add SKYCOT logo (top-left) + a value-prop sidebar (left half of the page) on `/auth/signup`, `/auth/check-email`, `/auth/signin`, and `/auth/verify` (the spinner / "verifying" state).

## What lands

1. Layout: two-column at large viewports (≥1024px); single-column at small.
2. Left column: SKYCOT logo + one of three rotating brand quotes ("Hire a teammate, not a tool." / "Memory that compounds." / "Audit by default.") + small illustration or product screenshot teaser.
3. Right column: the existing form, unchanged in function.
4. Mobile (<1024px): logo at top of single column; sidebar collapses; form below.

## Files you may touch

- `web/app/auth/signup/page.tsx` — wrap existing card in two-column layout. ≤40 modified LOC.
- `web/app/auth/check-email/page.tsx` — same. ≤40 modified LOC.
- `web/app/auth/signin/page.tsx` — same. ≤40 modified LOC.
- `web/app/auth/verify/page.tsx` — same. ≤40 modified LOC.
- `web/app/auth/layout.tsx` (NEW) — shared two-column layout component. ≤120 LOC.
- `web/components/skycot/AuthSidebar.tsx` (NEW) — logo + brand quote + teaser. ≤80 LOC.

## Files you must NOT touch

- Form logic / state / submit handlers — pure visual wrap.
- The brand quotes — copy is approved (matches marketing hero).
- The signup / verify / reset endpoints — gateway side untouched.

## Acceptance criteria

- [ ] At 1920x1080: SKYCOT logo top-left + brand quote in left column + form in right column.
- [ ] At 1024x768: same two-column layout, narrower.
- [ ] At 414x896 (mobile): single column, logo on top, form below.
- [ ] Brand quote rotates daily (deterministic — same quote per user-visit-day).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-auth-pages-brand-mark`
- Title: `feat(web): branded two-column layout for auth pages (mission-auth-pages-brand-mark)`
- Worker verification: viewport checks at 320, 414, 768, 1024, 1440, 1920px; auth flow still functions identically.

## Hidden constraints

- **Auth pages are the first hands-on touch.** They should feel as branded as the marketing landing.
- **The teaser image is optional** — a single brand quote is enough if no illustration is ready. Prefer to ship without a placeholder image than ship a low-quality one.
- **Closes a P2 visual-polish gap from the 2026-05-18 browser audit.**
