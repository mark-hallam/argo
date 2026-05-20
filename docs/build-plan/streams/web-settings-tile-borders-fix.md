# web-settings-tile-borders-fix — settings landing tiles render without borders

**Owner**: claude. **Sizing**: ~10-30 LOC.
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-settings-tile-borders-fix`.
**Priority**: P3 — cosmetic. Demo audit Pass-12 (`d6cc0ca8`) shipped the /settings landing with 8 tile groups. The styled-jsx CSS is applied (typography + grid layout work) but the per-tile borders + backgrounds aren't rendering — probably a specificity loss to a parent CSS rule.

## Scope

Investigate why `.settings-tile { border: 1px solid var(--border); background: white; ... }` isn't visible in the rendered output. Likely fixes:
- Bump specificity: `.settings-stage .settings-tile { ... }`
- Or move to CSS modules instead of styled-jsx global
- Or add `!important` (last resort)

Verify by re-screenshotting /settings — tiles should have visible borders, white backgrounds, hover-lift effect.

## Acceptance criteria

- [ ] Each settings tile renders with a visible border and white background.
- [ ] Hover lifts the tile +2px and adds purple-glow shadow.
- [ ] Layout (2-col grid, 14px gap) unchanged.
- [ ] No other surface impacted.
