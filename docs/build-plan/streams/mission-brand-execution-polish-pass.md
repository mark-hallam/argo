# mission-brand-execution-polish-pass — close brand-execution drift (P1 audit-finding)

**Owner**: claude. **Sizing hint**: judgment + cross-file consistency.
**Depends on**: `claude-brand-identity-system` MERGED (#1744).
**Authored against**: origin/main `fa849bbb` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-brand-execution-polish-pass`.
**Source**: 2026-05-17 claude conductor audit finding P1-2. Brand foundation is solid (7.5/10 per sub-agent audit). 3 specific execution drifts hold it back from 9+.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — brand-execution discipline IS the visible product.
- **Repeatability** (#3) — one motion vocabulary, one spacing scale, one responsive discipline.

## Scope

P1. Three drifts to close:

### Drift 1: Motion easing — replace `ease-out` with `ease-[var(--motion-ease)]`

DESIGN_TOKENS.md mandates `cubic-bezier(0.4, 0, 0.2, 1)` (`--motion-ease`) for all transitions. Today:

- `web/components/ui/Button.tsx:11` uses hardcoded `ease-out`.
- `web/components/skycot/audit/ReplayProgress.tsx:58` uses `duration-300 ease-out` (hardcoded both).

Plus: grep `web/components/` + `marketing/components/` for any other `ease-out`, `ease-in`, `ease-linear`, `transition-` without var. Replace with token references.

### Drift 2: Spacing grid — replace `gap-1.5` (6px, off-grid) with `gap-1` (4px) or `gap-2` (8px)

DESIGN_TOKENS.md specifies 8pt grid: space-1=4, space-2=8, space-3=12. 6px isn't a token.

Today: 8+ UI components use `gap-1.5`. Examples:
- `web/components/ui/Card.tsx:34,41`
- `web/components/ui/Breadcrumb.tsx:26,27`
- More via `grep -rE 'gap-1\.5|p-1\.5|m-1\.5' web/components marketing/components`.

For each: review visual intent + pick `gap-1` (tighter) or `gap-2` (looser). Document the choice in a single line comment if the rationale isn't obvious.

### Drift 3: SPA mobile-responsive parity with marketing

Marketing components use `sm:`/`md:` breakpoints extensively (33 responsive uses across LandingHero, Nav, KeyFact). SPA UI primitives (Card, Button, Dialog, Toast) rarely use them.

For each of the SPA UI primitives:
- `Card.tsx`, `Button.tsx`, `Dialog.tsx`, `Toast.tsx`, `Sheet.tsx`, `Drawer.tsx`, `Popover.tsx`
- Add `sm:` / `md:` variants where the component would visually cramp on mobile. Typical pattern: tighter padding + smaller text + stacked layout below `sm:`.
- Test in browser at 360px viewport — if the component renders cramped or overflows, fix.

LOC: ~150.

## Files you may touch

- `web/components/ui/Button.tsx` — motion-easing fix. ≤8 modified LOC.
- `web/components/ui/Card.tsx` — gap fix. ≤5 modified LOC.
- `web/components/ui/Breadcrumb.tsx` — gap fix. ≤5 modified LOC.
- `web/components/ui/Dialog.tsx`, `Sheet.tsx`, `Drawer.tsx`, `Popover.tsx`, `Toast.tsx` — mobile breakpoints. ≤30 modified LOC total.
- `web/components/skycot/audit/ReplayProgress.tsx` — motion fix. ≤5 modified LOC.
- Any other component grep surfaces — ≤50 modified LOC total.
- `web/__tests__/brand-execution-polish.test.tsx` (NEW) — assert no hardcoded ease-out / gap-1.5 anywhere via simple grep-style test. ≤40 LOC.

## Files you must NOT touch

- Brand tokens themselves (`web/app/globals.css` / `marketing/app/globals.css`) — read-only consumer.
- Component LOGIC (props, behavior, state) — pure style edits only.
- New components — out of scope.
- Marketing-site components — already audited as compliant.

## Acceptance criteria

- [ ] `rg -E 'ease-(out|in|linear)\b' web/components marketing/components` returns 0 matches (or all matches are inside `cn()` props that resolve to tokens).
- [ ] `rg -E '(gap|p|m|px|py|mx|my)-1\.5\b' web/components marketing/components` returns 0 matches.
- [ ] 7 SPA UI primitives have at least one `sm:` or `md:` responsive variant.
- [ ] Manual viewport check at 360px on 5 representative SPA pages — no cramping/overflow.
- [ ] `pnpm test && pnpm typecheck && pnpm lint` clean for web.
- [ ] Bundle delta ≤2KB gzipped (style-only changes).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-brand-execution-polish-pass`
- Title: `style(web): close brand drift — motion easing + spacing grid + mobile breakpoints (mission-brand-execution-polish-pass)`
- Worker verification: pnpm checks + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Visual changes are subjective.** Don't change spacing where the current value (even if off-grid) is visually correct — flag those with a comment + token alias instead of forcing a 4px change.
- **`tailwind.config.ts` is the right place to add `--motion-ease` as a Tailwind preset** — operator may have already done this; verify before adding raw `var(--motion-ease)` strings everywhere.
- **Lighthouse a11y must stay ≥95** on touched pages.
- **Closes audit finding P1-2.** When merged, brand-execution drift class is closed.
