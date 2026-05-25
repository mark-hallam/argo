# fix-app-shell-wide-viewport-content-rail-1920

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-app-shell-wide-viewport-content-rail-1920`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F17 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🟢 **P2 — App shell at 1920 width wastes ~380px of empty horizontal margin on each side. Cards + sparklines on `/approvals` look adrift in the middle. Investor monitors are 1440-2560 in 2026.**

## Root cause

App shell at 1920 width: main content column appears constrained to `~max-w-6xl` (≈1152px) with empty grey margin on both sides. The demo recording may be on a wider display than the dev preview; on /approvals specifically the KPI strip + sparklines look adrift in the middle of the screen.

Touch sites: `web/components/skycot/AppShell.tsx` + per-page wrappers.

Per the locked design memories: the content rail should respond to viewport width up to a reasonable upper bound (1440 or 1600), not stay flat at 1152.

## Findings this brief closes

- whole-site-mvp-claims-coverage F17 [P2] — Wide-viewport (1920) on app shell wastes horizontal real-estate

## Fix recipe

1. **Bump the content rail width** in `web/components/skycot/AppShell.tsx`:
   - Current: likely `max-w-6xl` (1152px) or similar
   - Target: `max-w-[1440px]` or `max-w-[1600px]` (operator's call — 1440 is the safer pick)
   - Add a min-width inset to preserve breathing room at the edges.

2. **Audit per-page wrappers** for surfaces that set their own `max-w-*`:
   - `/approvals`, `/audit`, `/workers`, `/workflows`, `/skills`, `/team`, `/settings`, `/cohorts`
   - Either remove the per-page max-width (inherit from shell) OR align them all to the new shell rail.

3. **Verify the locked designs adapt**:
   - C2 trading-card deck (4-col grid) should grow to 5-6 cols at 1920.
   - KPI strip stays 5 tiles wide but tiles are larger.
   - D3 magazine layout (1 lead + 2 stacked + 4 compact) gains breathing room.
   - Audit table: rows fill the wider rail.

4. **Pin** with a Playwright spec at `web/tests/app-shell-wide-viewport.spec.ts`:
   - For each canonical surface, set viewport to 1920×1080, assert the content rail width is >= 1400.
   - Set viewport to 2560×1440, assert no degradation.

5. **Manual smoke**: open `/approvals` at 1920, 2560 — confirm cards don't look "adrift in the middle".

## Acceptance criteria

- [ ] App shell content rail expands to ~1440px (or operator-chosen target) at 1920 viewport.
- [ ] No more than 240px empty margin per side at 1920 (was: ~380px).
- [ ] Per-page wrappers don't override or shrink below the shell rail.
- [ ] Locked-design layouts adapt cleanly (C2 deck, D3 magazine, KPI strip).
- [ ] No visual regression at 1280, 1440 (the existing target viewports).

## May-touch

- `web/components/skycot/AppShell.tsx`
- Per-page wrapper components (multiple `web/app/(app)/<route>/page.tsx` or sibling components)
- `web/styles/app-shell.css` (if max-width is in CSS not Tailwind)
- `web/tests/app-shell-wide-viewport.spec.ts` (NEW)

## Out of scope

- Mobile/tablet responsiveness (separate brief)
- Theme toggle (separate brief `fix-spa-theme-toggle-broken-storage-key-mismatch`)
- Per-tenant theming (roadmap item)

## Effort

~30-100 LOC + visual regression sweep + 1 spec. ~1-2 hours.
