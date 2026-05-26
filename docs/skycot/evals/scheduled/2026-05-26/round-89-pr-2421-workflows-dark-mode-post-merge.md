# Round 89 — PR #2421 workflows dark-mode contrast (POST-MERGE)

- **Date**: 2026-05-26 (UTC 15:13Z) / 2026-05-27 01:13 AEDT
- **Round**: 89
- **Pillar**: UX-AUDIT-T2309MEL F4/F22/F23/F25 — white-on-white workflows surface in dark mode
- **Verdict**: 🟢 **POST-MERGE PASS** (P1 — recording-impact)

## Setup

Merge commit: `1ad6de30a2b7aad7b5c239351081d9b54b14fd1e` on `origin/main`. +145 / −13 across 5 files. PR #2421 closes UX-AUDIT findings F4 + F22 + F23 + F25.

```
git show 1ad6de30a -- 'web/app/(app)/workflows/page.css' \
    'web/app/(app)/workflows/templates/page.css' \
    'web/app/(app)/workflows/[verticalId]/[name]/page.css' \
    'web/app/(app)/workflows/page.tsx' \
    'web/app/(app)/workflows/__tests__/dark-mode-contrast.test.tsx'
```

## Observed

- **Production diff is a 1-for-1 substitution**: every `background: white;` in the in-scope `page.css` / `page.tsx` files becomes `background: var(--card, #ffffff);`. The `#ffffff` fallback is pixel-identical to the prior literal `white` keyword, so **light-mode rendering is byte-stable**.
- **Coverage** (13 sites):
  - `workflows/page.css`: `.workflows-pin-card`, `.workflows-secondary-btn`, `.workflows-try-chip`, `.workflow-card` (+ 1 `border` ad-hoc → `var(--border)`).
  - `workflows/templates/page.css`: `.templates-pin-card`, `.templates-chip`, `.template-card`, and the empty-state panel.
  - `workflows/[verticalId]/[name]/page.css`: `.wfd-card`, `.wfd-btn`, `.wfd-step-body`.
  - `workflows/page.tsx:202`: 1 inline-style `background: "white"` (toolbar pagination button) → `var(--card, #ffffff)`.
- **Theme contract holds**: per `project_multi_tenant_theming` (operator-locked memory: "all visual tokens as CSS custom properties"), `--card` is toggled by the theme provider between `oklch(1.00 ...)` light and `oklch(0.205 ...)` dark on `[data-theme="skycot-dark"]` / `.dark`. The fix puts these 13 surfaces on the same wire as `workflows/build/page.tsx` + the earlier `fix-dark-mode-tokens-pricing-compare-docs-marketplace` brief.
- **New test** (`__tests__/dark-mode-contrast.test.tsx`, 132 LOC): 5 assertions.
  1. No `background: white;` (CSS) or `background: "white"` (TSX) literal remains in the 6 in-scope source files.
  2. `.workflow-card` uses `var(--card)`.
  3. `.template-card` uses `var(--card)`.
  4. `.wfd-btn` uses `var(--card)`.
  5. **Computed WCAG-AA contrast** between dark `--card` and dark `--foreground` ≥ 4.5:1 — parses the `.dark, [data-theme="skycot-dark"]` block out of `web/app/globals.css`, extracts the two `oklch(...)` tokens, converts via the official OKLab → linear-sRGB matrix, applies WCAG 2.1 relative-luminance, asserts the ratio.

## Anti-theater check

The static-contract assertions (1–4) are tautological-by-design — they read the production CSS file and assert the production change is present. That's fine for regression-pin purposes (a future "background: white;" insert breaks the build) but doesn't prove the rendered pixels meet contrast. The **5th assertion is the load-bearing one**: it computes real WCAG-AA contrast from the actual `globals.css` token values, so a future drift in either `--card` or `--foreground` for dark mode that drops contrast below 4.5:1 would fail this test even with zero changes to the workflow page sources. The OKLab → sRGB matrix and relative-luminance formulae match the WCAG 2.1 / CSS-Color-4 specs character-for-character. **Dispositive**.

Vitest can't render computed styles (`css: false`) — the test file documents this limitation in its header comment, not concealed.

## Gap / brief candidates

None. **87th anti-theater verification this week.**
