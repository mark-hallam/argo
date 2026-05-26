# fix-workflow-risk-pill-hex-literals-route-through-brand-tokens

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-risk-pill-hex-literals-route-through-brand-tokens`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-4-and-5-claim-4-and-5.md` F12.

🟦 **P2 — `/workflows/page.tsx:36-39` hardcodes `RISK_COLOR = {"approval-required": "#f59e0b", autonomous: "#14b8a6"}` as raw hex literals. The locked brand identity (per memory `project_skycot_brand_identity` + `project_brand_consistency_pass`) uses a violet → pink gradient. The current amber/teal pills feel off-brand and disconnect the workflows page from the gradient cards on /workers.**

## Root cause

`web/app/(app)/workflows/page.tsx:36-39`:
```ts
const RISK_COLOR: Record<RiskLevel, string> = {
  "approval-required": "#f59e0b", // amber
  autonomous: "#14b8a6",          // teal
};
```

Both hex values bypass the CSS-variable token system (`--color-brand-violet`, `--color-brand-pink`, `--color-warning`, `--color-success`) declared by the brand tokens.

The brand color recipe per memory `project_brand_consistency_pass`:
- Primary gradient: `#8B5CF6` (violet) → `#0891B2` (cyan) — or violet → pink in D3 register
- Warm radial-blob background
- `#1e1b4b` ink
- Audiowide hero typography

Amber + teal don't appear anywhere else in the SPA's brand surface. The workflows page is a brand-medium register (per design memory) — should use brand-aligned warning + neutral tones, not arbitrary semantic hex.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F12 [P2] — Workflow viewer's risk pill uses bg-color HEX literals that ignore brand tokens

## Fix recipe

1. **Define brand-aligned risk tokens** in the global stylesheet (or `tailwind.config.ts`):
   ```css
   :root {
     --color-risk-approval: <amber-shifted-toward-brand-palette>;
     --color-risk-autonomous: <teal-shifted-toward-brand-palette>;
   }
   ```
   Suggested values (designer-confirm):
   - `--color-risk-approval`: `#d97706` (amber-600, slightly warmer to harmonize with the warm radial-blob bg)
   - `--color-risk-autonomous`: `#0891B2` (the brand cyan already in the gradient — same hue as "this is fine")

2. **Replace the literal map** in `web/app/(app)/workflows/page.tsx:36-39`:
   ```ts
   const RISK_COLOR: Record<RiskLevel, string> = {
     "approval-required": "var(--color-risk-approval)",
     autonomous: "var(--color-risk-autonomous)",
   };
   ```

3. **Sweep adjacent components** for the same anti-pattern:
   - `SubstrateChip.tsx` (per F5 brief `fix-workflow-substrate-chip-show-macos-browser-e2b.md`)
   - `WorkflowAuthorChip.tsx`
   - Anything in `web/components/skycot/workflows/` with a hardcoded hex.

4. **Vitest visual-snapshot** or component snapshot test for `/workflows` to catch a future regression to literal hex.

## Acceptance criteria

- [ ] No `#[0-9a-f]{6}` literals in `web/app/(app)/workflows/**` or `web/components/skycot/workflows/**`.
- [ ] Risk pills render with brand-aligned colors that visually harmonize with the gradient hero on /workers + /skills.
- [ ] Vitest pin asserts no literal hex in workflows components.
- [ ] Snapshot test or visual diff catches regression.

## May-touch

- `web/app/(app)/workflows/page.tsx` (lines 31-39)
- `web/app/globals.css` or `web/styles/tokens.css` (token declarations)
- `tailwind.config.ts` (extend palette if using Tailwind classes)
- Adjacent workflows components (sweep)
- Vitest test

## Out of scope

- Full brand-token migration across the whole SPA (out — file follow-ups for `/audit`, `/approvals`, `/team` if needed)
- Substrate chip styling (covered by `fix-workflow-substrate-chip-show-macos-browser-e2b.md`)
- Color-blindness accessibility audit (separate concern)

## Effort

XS — ~30-50 LOC. ~30-60 min.
