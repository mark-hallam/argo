# web-skills-mobile-and-search-dark-mode-polish

**Owner**: claude or codex (`either-pool`).
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-skills-mobile-and-search-dark-mode-polish`.

🟢 **P3 polish — two small Skills-page dark-mode + mobile defects.** Small brief. Skills is the best-in-class in-product surface today (locked A1 marketplace design); these two fixes are about extending the brand-consistency pass across dark mode + mobile.

## Why this brief exists

During the 2026-05-24 demo SWOT cold-walk in dark mode + mobile, two defects on the Skills page (`/skills`) surfaced:

1. **Search-bar background stays white in dark mode.** The hero "Every skill your team will ever need." search input is the signature pattern (reused per memory `project_skills_page_design_locked`). In dark mode, the page background switches to dark navy but the search input chrome retains a `background: white` fallback, producing a high-contrast bright white pill in the middle of the dark page.

2. **Mobile: count text "6 skills · 6 shown" overflows the search input.** At `<sm` widths, the search input takes the full width and the count text positioned to its right gets pushed outside the visible viewport, wrapping awkwardly. The `⌘K` keyboard hint is also lost.

## Source of truth

- `web/app/(app)/skills/page.tsx`
- `web/app/(app)/skills/page.css`
- Operator memory `project_skills_page_design_locked` (locked 2026-05-19) — the A1 marketplace pattern.
- Operator memory `project_brand_consistency_pass` (2026-05-19) — the D3 pass that says Audiowide + brand-gradient + ink theming should be uniform.

## Scope

### 1. Theme-aware search-input chrome

Change the search-input background from a hardcoded white to `var(--card)` (or `var(--background)`, whichever reads better against the page bg). Confirm by toggling `data-theme="skycot-dark"` on `<html>` in DevTools and the search input darkens with the page.

Same fix likely applies to the input's border and the "⌘K" pill background — audit any hardcoded color literal in `web/app/(app)/skills/page.css` and convert to CSS custom-property tokens.

### 2. Mobile: stack the count + ⌘K hint

At `< 640px` (Tailwind `sm` breakpoint), the right-rail metadata (`6 skills · 6 shown` + `⌘K`) should either:

- (a) Drop below the search input as a thin row, OR
- (b) Hide entirely on mobile (the count is informational; on a phone the user is scanning the cards anyway)

Recommend (a) — keep the count visible because it's a useful "this is a small set" signal, but stack it below to give the input the full width.

## Files you may touch

- `web/app/(app)/skills/page.tsx`
- `web/app/(app)/skills/page.css`

## Files you must NOT touch

- Any other `(app)/` page — extending this fix to other surfaces (the marketing search, audit search, etc.) is a separate brief if those surfaces exhibit the same pattern.
- The `BrandLoudHero` component — Skills uses its own hero treatment.

## Acceptance criteria

- [ ] `/skills` search-input background tracks the page theme (white in light, dark navy in dark)
- [ ] On `<sm` viewports the count + ⌘K hint render below the input, not clipped to the right
- [ ] No new CSS file > 500 LOC; no hardcoded color literals introduced (use the `--*` tokens)
- [ ] Mobile + desktop screenshots in the PR body, both light + dark mode (4 screenshots total)

## Verification

```bash
pnpm --dir web build
bash scripts/check-brief-freshness.sh web-skills-mobile-and-search-dark-mode-polish
```

Then manually, with the gateway booted demo-mode + SPA up:

```bash
open http://localhost:3000/skills
# - resize browser to 375px wide → count should drop below input
# - DevTools toggle prefers-color-scheme: dark → input bg darkens
```

## Cross-references

- 2026-05-24 demo SWOT cold-walk finding #N5.
- Operator memory `project_skills_page_design_locked` (locked 2026-05-19).
- Operator memory `project_brand_consistency_pass` (2026-05-19).
