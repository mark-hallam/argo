# fix-workflows-dark-mode-card-white-on-white

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-dark-mode-card-white-on-white`.
**Source**: ux-e2e-audit-T2309MEL `whole-site-mvp-claims-coverage.md` F4 [P0] + F22 [P0] + F23 [P1] + F25 [P1].

🔴 **P0 — `/workflows`, `/workflows/templates`, and `/workflows/[verticalId]/[name]` cards render `background: rgb(255,255,255)` (literal white) AND text in `color: lab(96.5,…)` (off-white #f6f6f6) IN DARK MODE. Workflow names ("first_listing", "qualify_lead") and TRY pills are UNREADABLE. The workflow-detail toolbar has 4 buttons; 3 are white-on-white in dark mode. The theming system at the HTML root flips `data-theme="skycot-dark"` + `class="dark"` correctly — the card components just don't read the theme tokens.**

## Root cause

Per `project_multi_tenant_theming` (operator-locked): "All visual tokens as CSS custom properties." Workflow + workflow-templates card components were authored with **hardcoded** `bg-white` / `text-slate-100` Tailwind utilities (or hex-literal colors) instead of consuming the theme tokens (`var(--surface)`, `var(--ink)`, etc.) that the theme provider toggles on `[data-theme="dark"]`.

Audit evidence (DOM inspect):
- `/workflows` card: computed `background: rgb(255, 255, 255)` (hardcoded white) regardless of `data-theme`
- Card text: computed `color: lab(96.5 0.0001 -0.0119)` ≈ `#f6f6f6` (theme-aware off-white that flipped correctly)
- Net: white text on white background

The existing brief `fix-dark-mode-tokens-pricing-compare-docs-marketplace` covers FOUR pages (`/pricing`, `/compare`, `/docs`, `/for/marketplace`) but explicitly NOT `/workflows`, `/workflows/templates`, or `/workflows/{id}`. This brief fills the remaining gap.

## Findings this brief closes

- whole-site-mvp-claims-coverage F4 [P0] — `/workflows` + `/workflows/templates` cards white-on-white in dark mode
- whole-site-mvp-claims-coverage F22 [P0] — Same applies to workflow detail page
- whole-site-mvp-claims-coverage F23 [P1] — Workflow detail toolbar has 3-of-4 unreadable buttons in dark mode
- whole-site-mvp-claims-coverage F25 [P1] — Some cards have hardcoded white backgrounds not theme-responding (broader pattern)

## Fix recipe

1. **Enumerate affected files** with `rg` searches:
   ```bash
   rg "bg-white\|background:.*#fff\|background:.*rgb\(255" web/app/(app)/workflows/
   rg "text-slate-1\|text-white\|color:.*#fff" web/app/(app)/workflows/
   rg "bg-white\|background:.*#fff" web/components/skycot/workflows/
   ```

2. **Replace hardcoded with token references.** Per the project conventions:
   - `bg-white` → `bg-[var(--surface)]` (or Tailwind theme-token utility — verify the project's setup)
   - `text-slate-100`/`text-white` on a card body → `text-[var(--ink)]`
   - Hex literals like `#fff` / `rgb(255,...)` → `var(--surface)` / `var(--ink)` / `var(--ink-muted)`

3. **Target files (per audit + grep)**:
   - `web/app/(app)/workflows/page.tsx` + colocated `.module.css` / `<style jsx>` blocks
   - `web/app/(app)/workflows/templates/page.tsx` + colocated styles
   - `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` toolbar buttons (4 CTAs)
   - `web/components/skycot/workflows/WorkflowCard.tsx` (or equivalent)
   - `web/components/skycot/workflows/WorkflowToolbar.tsx` (or wherever the 4 buttons live)

4. **Verify "use client" preservation.** Per `feedback_styled_jsx_requires_use_client`, any page with `<style jsx>` blocks REQUIRES `"use client"` at the top. Do not remove it during the refactor; verify with `curl` post-edit that the page returns 200 not 500.

5. **Add a vitest spec** at `web/app/(app)/workflows/__tests__/dark-mode-contrast.test.tsx`:
   ```ts
   it("workflow card body text contrast vs surface is ≥ 4.5:1 in dark mode", () => {
     // Render with dark theme, query the card, measure computed colors,
     // assert WCAG AA contrast
   });
   ```

6. **Smoke test live**: open `/workflows` + `/workflows/templates` + `/workflows/post_listing` in dark mode (toggle via the existing theme picker), confirm all card names + button labels are readable.

## Acceptance criteria

- [ ] `/workflows`, `/workflows/templates`, `/workflows/[verticalId]/[name]` in dark mode: all workflow names, TRY pills, and toolbar button labels readable (≥ 4.5:1 contrast)
- [ ] `rg 'bg-white\|background:.*#fff\|background:.*rgb\(255' web/app/(app)/workflows/` returns no matches outside intentional light-mode-only blocks
- [ ] No `"use client"` directives removed during the refactor (per `feedback_styled_jsx_requires_use_client`)
- [ ] All three pages return HTTP 200 (not 500) post-edit
- [ ] New vitest dark-mode contrast spec green
- [ ] `pnpm --dir web exec tsc --noEmit` green
- [ ] `pnpm --dir web exec next build` green

## May-touch

- `web/app/(app)/workflows/page.tsx` + colocated styles (~15 LOC)
- `web/app/(app)/workflows/templates/page.tsx` + colocated styles (~15 LOC)
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` + toolbar block (~10 LOC)
- `web/components/skycot/workflows/WorkflowCard.tsx` (~10 LOC)
- `web/components/skycot/workflows/WorkflowToolbar.tsx` (or wherever the 4-CTA toolbar lives) (~10 LOC)
- New vitest spec (~50 LOC)

## Out of scope

- The 4 pages (`/pricing`, `/compare`, `/docs`, `/for/marketplace`) covered by `fix-dark-mode-tokens-pricing-compare-docs-marketplace`
- `/skills` card detail (covered separately if surfaces in the audit; otherwise polish)
- Theme provider / theme-toggle plumbing (already shipped + working; this brief is component-level only)

## Effort

S–M. ~60 LOC component edits + ~50 LOC test. ~1.5 hours. P0 because two pages the demo recording walks through render UNREADABLE in dark mode.
