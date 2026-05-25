# fix-marketing-pages-theme-drift-landing-vs-compare

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-pages-theme-drift-landing-vs-compare`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F8 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟡 **P1 — `/landing` + `/customers` render light theme; `/compare` renders dark theme. Visible flash when navigating between them. Marketing surface is one cohort visually broken.**

## Root cause

- `/landing` and `/customers` render light theme (Audiowide-on-cream-paper, violet→pink gradient on the hero word).
- `/compare` renders dark theme (foreground text on near-black, gradient muted).
- Hover/click from a marketing CTA on `/compare` back to `/landing` causes a visible flash.

Per operator-locked memory `project_brand_consistency_pass` + `feedback_design_direction_friendly_enterprise_canva`: light is the operator preference for marketing surfaces.

Touch sites likely:
- `web/app/landing/page.css`
- `web/app/customers/page.css`
- `web/app/compare/page.css`
- The marketing layout wrapper (`web/app/(marketing)/layout.tsx` or similar)

## Findings this brief closes

- whole-site-mvp-claims-coverage F8 [P1] — Marketing nav cross-page theme drift (compare is dark, landing+customers are light)

## Fix recipe

1. **Audit each marketing page's theme**:
   - `/landing` → confirm light
   - `/customers` → confirm light
   - `/compare` → currently dark, switch to light
   - `/`, `/security`, `/sla`, `/help`, `/privacy`, `/terms` → confirm light

2. **Switch `/compare` to light**:
   - Touch site: `web/app/compare/page.css` (or `.tsx` if styles inline).
   - Change background from near-black to warm cream (matches `/landing`).
   - Change foreground text to `#1e1b4b` ink.
   - Keep the gradient on hero words — but on light bg (it's currently muted on dark).
   - Verify the comparison-table styling reads on light bg.

3. **Centralize theme at the marketing layout level**:
   - If marketing pages don't already share a layout, add `web/app/(marketing)/layout.tsx` that pins `data-theme="skycot-light"` (or whatever class names the FOUC script uses).
   - All marketing pages inherit from there; per-page CSS shouldn't override.

4. **Pin with a Playwright spec** at `web/tests/marketing-light-theme.spec.ts`:
   - For each marketing route, assert `<html>` carries the light theme class.
   - Navigate `/landing` → `/compare` → `/landing`; assert no theme flash (capture screenshots, compare bg colors).

## Acceptance criteria

- [ ] `/landing`, `/customers`, `/compare`, and all other marketing pages render in light theme.
- [ ] No visible theme flash navigating between marketing pages.
- [ ] Gradient + Audiowide hero treatments work on light bg consistently.
- [ ] Playwright spec passes.

## May-touch

- `web/app/compare/page.css` (or `.tsx`)
- `web/app/(marketing)/layout.tsx` (NEW if needed)
- `web/app/landing/page.css` (verify only)
- `web/app/customers/page.css` (verify only)
- `web/tests/marketing-light-theme.spec.ts` (NEW)

## Out of scope

- Auth pages dark→light (separate brief `fix-auth-pages-light-theme-to-match-wizard`)
- SPA theme toggle (separate brief `fix-spa-theme-toggle-broken-storage-key-mismatch`)
- Marketing nav fragment anchors (separate brief `fix-marketing-nav-fragment-anchors-and-dead-cta`)

## Effort

~40-100 LOC + 1 Playwright spec. ~1-2 hours.
