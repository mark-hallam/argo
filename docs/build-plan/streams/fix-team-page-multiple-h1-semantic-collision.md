# fix-team-page-multiple-h1-semantic-collision

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-multiple-h1-semantic-collision`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/whole-site-mvp-claims-coverage.md` F16 [P2].

🟢 **P2 — `/team` page renders THREE `<h1>` elements: the nav-shell breadcrumb "Team", the brand-loud empty-state hero "Hire your first teammate.", and a third "Team" H1 lower in the document tree. Multiple H1s break semantic HTML and screen-reader tab order. Accessibility soft violation; not a recording blocker.**

## Root cause

Two surfaces both label themselves H1:

1. **App-shell breadcrumb** (`web/components/skycot/AppShell*.tsx`) renders the page title as `<h1 class="text-xl font-semibold tracking-tight">Team</h1>`.
2. **Page hero** (`web/app/(app)/team/page.tsx`) renders `<h1 class="skycot-brandloud-title">Hire your first teammate.</h1>` in the empty-state branch.
3. A third "Team" `<h1>` appears later in the document tree (likely a section header that was promoted to H1 by mistake).

For accessibility:

- Screen readers announce all H1s. Three H1s on one page produces three competing top-level landmarks.
- WCAG 2.4.10 + AXE rules flag multiple H1s as a soft violation.
- Tab-order semantic landmarks are confused.

For visual design:

- The shell label is correct and necessary.
- The brand-loud hero IS the page hero, so it WANTS to be H1.
- The third `<h1>` is the bug — it should be `<h2>` or `<h3>`.

## Findings this brief closes

- whole-site-mvp-claims-coverage F16 [P2] — `/team` page renders TWO (actually three) `<h1>` elements
- whole-site-mvp-claims-coverage F17 [P2] (partial) — `/settings/*` sub-pages have the same shell + page H1 collision (sibling brief `fix-settings-shell-double-h1-and-danger-zone-redirect` covers the settings case)

## Fix recipe

Two parts:

### Part 1 — demote the app-shell breadcrumb from H1 to H2 (or `role="banner"`)

`web/components/skycot/AppShell*.tsx` (verify exact file — the breadcrumb is likely in `AppShellTopBar.tsx` or `AppShellHeader.tsx`):

```tsx
// Before:
<h1 className="text-xl font-semibold tracking-tight">{pageTitle}</h1>

// After:
<h2 className="text-xl font-semibold tracking-tight" role="banner">{pageTitle}</h2>
```

The shell title becomes a secondary landmark. The page hero (Hire your first teammate / Skills / etc.) retains H1.

### Part 2 — find the third H1 on `/team`

Grep `web/app/(app)/team/page.tsx` for `<h1`. Demote any non-hero H1 to H2/H3. The duplicate "Team" label was likely added during a refactor.

### Part 3 — Playwright accessibility assertion

`web/playwright/accessibility/single-h1-per-page.spec.ts`: visit each in-app page (`/team`, `/skills`, `/workflows`, `/cohorts`, `/audit`, `/approvals`, `/workers/<id>/overview`). Assert exactly one `<h1>` per page. Catches future regressions.

## Acceptance criteria

- [ ] `/team` page has exactly one `<h1>` — the brand-loud hero "Hire your first teammate."
- [ ] The app-shell breadcrumb is `<h2>` (or another non-h1 landmark).
- [ ] No regression on `/skills`, `/workflows`, `/cohorts`, `/audit`, `/approvals`, `/workers/<id>` (each has exactly one H1).
- [ ] AXE-core accessibility audit on `/team` passes the "page-has-heading-one" rule.
- [ ] Playwright spec passes cold.

## May-touch

- `web/components/skycot/AppShell*.tsx` (demote shell H1 → H2)
- `web/app/(app)/team/page.tsx` (find + demote third H1)
- `web/playwright/accessibility/single-h1-per-page.spec.ts` (NEW)

## Out of scope

- `/settings/*` shell H1 collision (covered by `fix-settings-shell-double-h1-and-danger-zone-redirect`)
- WCAG full-audit (separate accessibility track)
- Visual restyle of the shell breadcrumb (purely semantic change here)

## Effort

~30-60 LOC + 1 Playwright spec. ~1-2 hours. P2 accessibility-soft + future-proofing.
