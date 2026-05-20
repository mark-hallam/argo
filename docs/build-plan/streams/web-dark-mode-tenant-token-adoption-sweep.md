# web-dark-mode-tenant-token-adoption-sweep — replace hardcoded brand colors with tokens (PR-A + PR-B)

**Owner**: claude (split across 2 PRs to keep review tractable).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-dark-mode-tenant-token-adoption-sweep`.
**Priority**: P1 — wave-9 dark-mode audit found ~47 hardcoded color
references (`#fafaff` background, `#1e1b4b` ink, `#8b5cf6` brand, etc.)
that bypass the token system. Result: dark mode renders broken; tenant
white-labeling (operator-locked memory: per-tenant CSS-custom-property
themes) is impossible until the sweep lands.
**Sizing**: ~600-800 LOC across ~47 files. **MUST SPLIT INTO 2 PRs** —
PR-A (mechanical) + PR-B (per-callsite verified) to keep review
tractable.

---

## Why this brief exists

The token system shipped in `web/app/globals.css` already defines:
- `--background` / `--foreground` (page bg + ink)
- `--card` / `--card-foreground`
- `--primary` / `--primary-foreground` (brand gradient anchor)
- `--muted` / `--muted-foreground` (slate)
- `--border` / `--input` / `--ring`
- `--destructive` / `--success` / `--warning` / `--info`
- `--radius-{sm,md,lg}`

These are then theme-overridable via `[data-theme="dark"]` and (operator's
locked memory: `project_multi_tenant_theming.md`) per-tenant
`[data-tenant-theme="acme"]` etc.

But ~47 component files still write **`background: "#fafaff"`** or
**`color: "#1e1b4b"`** inline — bypassing the system entirely. Wave-9
dark-mode audit verdict: "all marketing pages render with white text on
white background in dark mode because the inline colors don't flip."

This brief sweeps the codebase to replace hardcoded brand colors with
their token equivalents, in two coordinated PRs.

---

## Pre-flight verification

```bash
# 1. Count the gap.
rg "#fafaff|#1e1b4b" web/ -l | wc -l         # expect ~47

# 2. Confirm token definitions are stable.
rg "^  --background:|^  --foreground:" web/app/globals.css

# 3. Confirm dark-mode override structure.
rg '\[data-theme="dark"\]' web/app/globals.css

# 4. Confirm there is NO ongoing parallel sweep in flight.
rg "web-dark-mode-tenant-token-adoption-sweep" docs/build-plan/BUILD_STATUS.md
```

---

## Color-to-token mapping

The full mapping for the 47-file sweep:

| Hardcoded literal | Replace with | Notes |
|-------------------|--------------|-------|
| `#fafaff` | `var(--background)` | page background (light) |
| `#1e1b4b` | `var(--foreground)` | primary text/ink |
| `#52525b` | `var(--muted-foreground)` | secondary text |
| `#71717a` | `var(--muted-foreground)` | tertiary text (collapse to muted) |
| `#8b5cf6` | `var(--primary)` | brand violet (gradient anchor) |
| `#ec4899` | `var(--gradient-end)` ← new token (see Phase 0) | brand pink |
| `#0891b2` | DELETE (legacy violet→cyan gradient — superseded per brand-consistency pass) | — |
| `#d4d4d8` | `var(--border)` | hairline border |
| `#e4e4e7` | `var(--input)` | input border |
| `#fafafa` | `var(--card)` | card background |
| `#dc2626` | `var(--destructive)` | error red |
| `#16a34a` | `var(--success)` | success green |
| `#f59e0b` | `var(--warning)` | warn amber |

**The gradient pair (`#8b5cf6 → #ec4899`) needs a new token** — Phase 0.

---

## Phase 0 — Add gradient-end token (one-line CSS edit)

**File**: `web/app/globals.css`

Add to `:root` block:
```css
  --gradient-end: #ec4899;
```

Add to `[data-theme="dark"]` block:
```css
  --gradient-end: #f472b6;   /* lifted for dark contrast */
```

(Per-tenant overrides — `[data-tenant-theme="acme"]` — can swap `--gradient-end`
to that tenant's secondary brand color when white-label themes ship.)

Token note: `--primary` is the gradient START (#8b5cf6 violet, lifted to
#a78bfa in dark). The new `--gradient-end` is its sibling. Inline
gradients become:

```diff
- backgroundImage: "linear-gradient(135deg, #8b5cf6, #ec4899)"
+ backgroundImage: "linear-gradient(135deg, var(--primary), var(--gradient-end))"
```

---

## PR-A — Mechanical sweep (9 files, ~150 LOC delta)

PR-A handles the files where the replacement is 1:1 mechanical (the inline
color is ONLY used as a single value, never composited / conditional). The
9 files have only `background: "#fafaff"` and `color: "#1e1b4b"` as solid
values:

1. `web/app/help/page.tsx`
2. `web/app/offline/page.tsx`
3. `web/app/global-error.tsx`
4. `web/components/pwa/InstallPrompt.tsx`
5. `web/components/auth/WizardSkeleton.tsx`
6. `web/components/skycot/workflows/WorkflowBuilderSkeleton.tsx`
7. `web/components/workflows/triggers/shared.tsx`
8. `web/components/workflows/triggers/WebhookConfigForm.tsx`
9. `web/components/workflows/triggers/FormConfigForm.tsx`

### PR-A automation script

A reproducible mechanical sed sweep:

```bash
# Run from web/ root.
files=(
  "app/help/page.tsx"
  "app/offline/page.tsx"
  "app/global-error.tsx"
  "components/pwa/InstallPrompt.tsx"
  "components/auth/WizardSkeleton.tsx"
  "components/skycot/workflows/WorkflowBuilderSkeleton.tsx"
  "components/workflows/triggers/shared.tsx"
  "components/workflows/triggers/WebhookConfigForm.tsx"
  "components/workflows/triggers/FormConfigForm.tsx"
)
for f in "${files[@]}"; do
  # Mechanical replacements — only literal-form (in quotes).
  perl -i -pe '
    s/"#fafaff"/"var(--background)"/g;
    s/"#1e1b4b"/"var(--foreground)"/g;
    s/"#52525b"/"var(--muted-foreground)"/g;
    s/"#71717a"/"var(--muted-foreground)"/g;
    s/"#d4d4d8"/"var(--border)"/g;
    s/"#8b5cf6"/"var(--primary)"/g;
    s/"#ec4899"/"var(--gradient-end)"/g;
  ' "$f"
done
```

Then manual verification on each file: skim diff, confirm no false
positives.

### PR-A acceptance

- [ ] 9 files updated with mechanical replacements only.
- [ ] No conditional / composited colors touched (those go to PR-B).
- [ ] All 9 pages render correctly in light mode (visual diff).
- [ ] All 9 pages render correctly in dark mode (manually toggle
      `[data-theme="dark"]` on `<html>` and re-screenshot).
- [ ] `pnpm typecheck && pnpm lint` clean.
- [ ] Storybook stories (if any) for the 9 components still render.
- [ ] Bundle size delta is ~0 (CSS variable indirection is free).

PR-A is intentionally MERGED BEFORE PR-B starts. The two PRs touch
disjoint file sets so there's no merge conflict.

---

## PR-B — Per-callsite verified (~10-12 files, ~500 LOC delta)

PR-B handles files where the replacement isn't purely mechanical because
the inline color is used:
- In conditional expressions (`color: isError ? "#dc2626" : "#1e1b4b"`)
- In computed style objects (gradient stops, box-shadow rgba)
- In ternary class-name selectors
- In SVG fill attributes

These need per-callsite read + verify because a wrong substitution breaks
visual hierarchy.

### PR-B file list (per the wave-9 audit)

10. `web/app/pricing/page.tsx`
11. `web/app/compare/page.tsx`
12. `web/app/docs/page.tsx`
13. `web/app/for/_components/VerticalLandingShell.tsx`
14. `web/components/skycot/TeammateCard.tsx`
15. `web/components/skycot/worker/WorkerPersonalityCard.tsx`
16. `web/components/skycot/workers/EscalationMatrix.tsx`
17. `web/components/skycot/workers/TrustOverTimeChart.tsx`
18. `web/components/skycot/CommandPalette.tsx`
19. `web/components/workflows/triggers/TriggerConfigDrawer.tsx`

(Additional files discovered by `rg -l "#fafaff|#1e1b4b" web/` at brief-claim
time get added to this list if the count exceeds 12.)

### PR-B per-file pattern

For each file:

1. Read full file (Read tool, not bash sed).
2. Inventory every hex-color literal.
3. Classify each: mechanical (use perl) or conditional (replace by hand).
4. Update with token equivalents.
5. Light + dark visual diff (manual screenshot).
6. Commit.

**Example — `web/components/skycot/TeammateCard.tsx`** likely has:

```tsx
const tone = riskLevel === "high" ? "#dc2626" : "#1e1b4b";
// becomes:
const tone = riskLevel === "high" ? "var(--destructive)" : "var(--foreground)";
```

The conditional expression preserves; only the values change.

### Gradient handling

Most likely composite use: `linear-gradient(135deg, #8b5cf6, #ec4899)`.

Two equivalent rewrites:
```tsx
// Inline style:
style={{ backgroundImage: "linear-gradient(135deg, var(--primary), var(--gradient-end))" }}

// Or via Tailwind utility (preferred for new code, but inline OK for now):
className="bg-gradient-to-br from-primary to-[var(--gradient-end)]"
```

Pick the form that matches the existing convention in each file.

---

## Phase 4 — Add dark-mode theme toggle (operator-facing affordance)

After both PRs land, the theme toggle isn't useful unless the user can
flip it. The actual toggle UI is OUT OF SCOPE for THIS brief (sibling brief
`web-theme-toggle-ui` handles that). But add a CSS sanity-check:

**File**: `web/app/globals.css`

Confirm the dark-theme block defines all variables:

```css
[data-theme="dark"] {
  --background: #0a0a14;
  --foreground: #f8fafc;
  --card: #14141e;
  --card-foreground: #f8fafc;
  --muted: #1e1b29;
  --muted-foreground: #94a3b8;
  --border: #2a2438;
  --input: #2a2438;
  --primary: #a78bfa;       /* lifted from #8b5cf6 */
  --primary-foreground: #1e1b4b;
  --gradient-end: #f472b6;
  /* ... and so on for all tokens ... */
}
```

If any variable is undefined in the dark block, dark mode falls back to
the light value and the page is half-broken. **Test by adding `data-theme="dark"` to
`<html>` in the browser inspector and screenshotting every page.**

---

## Phase 5 — Visual regression test

**File**: `web/tests/dark-mode-token-coverage.spec.ts` (new)

Playwright E2E that:
1. Visits each migrated page in light mode → screenshot.
2. Toggles `<html data-theme="dark">` → screenshot.
3. Asserts no `rgb(255, 255, 255)` background appears in dark mode
   (specific check for the broken "white on white" pattern).

```ts
test.describe("dark-mode token coverage", () => {
  const pages = ["/help", "/offline", "/pricing", "/compare", "/docs",
                 "/workers/test-worker/overview", "/audit"];
  for (const path of pages) {
    test(`${path} renders correctly in dark mode`, async ({ page }) => {
      await page.goto(path);
      await page.evaluate(() => document.documentElement.dataset.theme = "dark");
      // No element should have rgb(255,255,255) background in dark mode.
      const whiteBacks = await page.evaluate(() => {
        const all = document.querySelectorAll("*");
        let count = 0;
        for (const el of all) {
          const bg = getComputedStyle(el).backgroundColor;
          if (bg === "rgb(255, 255, 255)") count++;
        }
        return count;
      });
      expect(whiteBacks).toBeLessThan(5);  // some legitimate whites (logos, hairlines)
    });
  }
});
```

---

## may-touch (PR-A)

- The 9 mechanical-sweep files listed above.
- `web/app/globals.css` — Phase 0 gradient-end token.

## may-touch (PR-B)

- The 10-12 per-callsite-verified files listed above.
- `web/tests/dark-mode-token-coverage.spec.ts` (new) — visual regression.

## must-NOT-touch

- The token DEFINITIONS in `globals.css` (Phase 0 adds ONE new token —
  `--gradient-end`. Otherwise definitions stay).
- The dark-theme override block (verify it's complete; don't refactor).
- Per-tenant theming infrastructure (out of scope; this brief enables it
  by removing the inline-color blockers).
- SVG icons with intrinsic colors (logos, lucide-react icons — those
  inherit `currentColor` and are already correct).

---

## Acceptance criteria

### PR-A
- [ ] All 9 PR-A files contain ZERO hardcoded `#fafaff` / `#1e1b4b` / `#52525b` / `#71717a` / `#d4d4d8` / `#8b5cf6` / `#ec4899` literals (verifiable: `rg "#fafaff\|#1e1b4b" <file>` returns nothing for each).
- [ ] Light mode pixel-diff: matches pre-merge screenshots within tolerance.
- [ ] Dark mode pixel-diff: pages no longer have white-on-white text.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

### PR-B
- [ ] Total `rg "#fafaff\|#1e1b4b" web/` matches drops from ~47 to 0.
- [ ] Conditional expressions preserved (e.g. `riskLevel === "high"`-style logic intact).
- [ ] All migrated pages render correctly in both light and dark mode.
- [ ] E2E `dark-mode-token-coverage.spec.ts` passes for all enumerated pages.
- [ ] No flicker / FOUC ("Flash of Unstyled Content") on initial mount (variables resolve before paint).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

### Combined post-merge
- [ ] Toggle `data-theme="dark"` on `<html>` in any page; observe sane dark theme.
- [ ] Per-tenant theming substrate ready: setting `data-tenant-theme="X"` on `<html>` is the future hook for white-label themes (no implementation yet, just the ability).

---

## Gotchas

1. **`var(--background)` in a string literal**: TypeScript doesn't validate
   CSS-var strings. A typo like `var(--bg)` (which would be `undefined`)
   would render as `currentColor` fallback — silently broken. Run the
   E2E spec to catch this.
2. **The gradient direction matters**: existing gradients use 135deg
   (top-left → bottom-right). Don't accidentally change to 90deg during
   the rewrite — that visibly reorients the brand gradient.
3. **CSS variable resolution at SSR time**: Next.js SSR can't compute CSS
   variables — they resolve at browser paint. Variables in inline-style
   objects work fine; variables in computed-string contexts (e.g.
   `tw-colors` plugin lookups) need extra care.
4. **Per-tenant theming hook**: after this brief, the operator can write
   `<html data-tenant-theme="acme">` and add `[data-tenant-theme="acme"] { ... }`
   block to override any token. THIS brief doesn't ship per-tenant UI —
   it removes the BLOCKER (inline colors) so future briefs can ship it.
5. **Tailwind class arbitrary values**: forms like `text-[#1e1b4b]` are
   also hardcoded and need to migrate to `text-foreground` or
   `text-[var(--foreground)]`. The grep `rg "#fafaff\|#1e1b4b"` catches
   these — verify both `style={...}` AND `className="..."` occurrences.
6. **The pricing / compare / landing pages may have many hex colors in
   one file (PR-B file 10)** — splitting per-file commits within PR-B is
   acceptable if the diff balloons. Group commits by visual region (hero
   / cards / footer) to keep review tractable.
7. **`#fafafa` vs `#fafaff`** — these are DIFFERENT colors (one is the
   neutral-50 gray, the other is the brand-shifted off-white). Make sure
   the replacement table preserves the distinction. `#fafafa` → `--card`;
   `#fafaff` → `--background`.

---

## References

- Audit: `docs/skycot/audit/dark-mode/2026-05-20-wave-9-dark-mode.md`
- Token contract: `docs/build-plan/streams/adr-frontend-design-language.md` §D3
- Operator memory: `project_multi_tenant_theming.md`
- Brand pass: `project_brand_consistency_pass.md`
- Sibling: `web-theme-toggle-ui` (not yet authored — operator UI to flip themes)
