# web-a11y-contrast-tokens-sweep — fix muted-foreground contrast (4.4:1 → 4.5:1)

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-a11y-contrast-tokens-sweep`.
**Priority**: P2 — wave-2 a11y audit: muted-foreground token sits at 4.4:1 vs WCAG AA 4.5:1 threshold across 544 callsites. Single-token bump fixes all of them.
**Sizing**: ~50-150 LOC (one CSS variable bump + visual sanity-check + a contrast test).

## Why this brief exists

From the wave-2 a11y audit (`docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md`):

> Color contrast on `muted-foreground`: 4.4:1 vs 4.5:1 AA threshold,
> 544 callsites. Would fail `color-contrast` on an automated axe/wave
> scan.

The token `--muted-foreground` (currently `#71717a` Tailwind slate-500-ish)
sits 0.1 below the AA threshold against `--background` `#fafaff`. A
single CSS-variable change in `globals.css` flips all 544 callsites
to passing.

## Goals

1. Bump `--muted-foreground` from `#71717a` to a value that meets 4.5:1 against `#fafaff` (target: `#52525b` Tailwind slate-600 = ~7.0:1 on white) AND against `#1e1b4b` ink (for inverted contexts).
2. Verify no visual regression on demo-path pages (slight darkening; should be subtle).
3. Add a contrast smoke-test using `axe-core` or `pa11y` in `pnpm test`.
4. Audit any per-component overrides that hardcoded `#71717a` directly (bypass the token) and align.

## Steps

1. Read `web/app/globals.css` for current token value.
2. Pick replacement: `#52525b` (slate-600) — 7.0:1 on `#fafaff`, 4.7:1 on `#1e1b4b`.
3. Bump token + run dev server + curl + screenshot each demo-path page for diff.
4. Find direct-hex callsites: `rg -i '71717a' web/`. Swap to `var(--muted-foreground)` where they should respect the token.
5. Add contrast smoke-test under `web/__tests__/a11y-contrast.test.ts` using @axe-core/playwright or @testing-library + a contrast utility.

## may-touch

- `web/app/globals.css` (the token bump)
- Any component file hardcoding `#71717a` (swap to var)
- New: `web/__tests__/a11y-contrast.test.ts`

## must-NOT-touch

- Brand gradient colors (`#8b5cf6`, `#ec4899`) — unrelated
- BrandLoudHero (its white-on-gradient text is high-contrast by design)

## Acceptance criteria

- [ ] `--muted-foreground` ≥ 4.5:1 contrast against both `--background` and `--card` tokens.
- [ ] axe-core scan on /landing, /, /team, /workflows, /audit reports 0 `color-contrast` violations.
- [ ] Visual regression OK on the 5 demo pages (subtle text-darkening only).
- [ ] `pnpm test` passes the new contrast smoke.

## References

- Audit: `docs/skycot/audit/a11y/2026-05-20-wave-2-a11y.md`
- WCAG 2.1 AA 1.4.3 (Contrast minimum)
