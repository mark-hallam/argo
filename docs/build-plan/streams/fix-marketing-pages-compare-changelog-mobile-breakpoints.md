# fix-marketing-pages-compare-changelog-mobile-breakpoints

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-marketing-pages-compare-changelog-mobile-breakpoints`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F16 (P2).

🟡 **P2 — `/compare` and `/changelog` ship a single `@media` breakpoint each (`web/app/compare/page.css`, `web/app/changelog/page.css`). Both pages were built desktop-first with a single mobile-breakpoint pass. The investor-demo recording stays at 1440 so this isn't a blocker, but customer-facing share links on phones (which is the largest source of marketing-page traffic by volume) will land on awkward layouts.**

## Root cause

`web/app/compare/page.css` and `web/app/changelog/page.css` each contain exactly one `@media` block, suggesting desktop-first build with one mobile pass. Common breakpoints (768 tablet, 480 small mobile) are missing. The layout collapses or scrolls horizontally at common social-share viewport sizes.

The locked design memos for `/compare` and `/changelog` don't specify breakpoints — this is a polish gap, not a design-language drift.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F16 [P2] — `/compare` and `/changelog` ship a single `@media` breakpoint each; behavior at 375 / 768 / 1920 unverified

## Fix recipe

1. Audit `web/app/compare/page.css` and `web/app/changelog/page.css`; identify the existing breakpoint and what it covers.
2. Add two additional breakpoints to each:
   - `@media (max-width: 768px)` — tablet portrait (stack columns, reduce hero font-size, simplify grids)
   - `@media (max-width: 480px)` — small mobile (single-column, reduce padding, smaller hero)
3. Manually verify each at 375 / 768 / 1024 / 1440 / 1920 (use Chrome devtools responsive mode).
4. (Optional) author a playwright snapshot test per breakpoint to lock the visual.

## Acceptance criteria

- [ ] `grep -c "@media" web/app/compare/page.css` returns 3 (or higher).
- [ ] `grep -c "@media" web/app/changelog/page.css` returns 3 (or higher).
- [ ] Manual visual check at 375 / 768 / 1440: no horizontal scroll, no overlapping text, no broken grids.
- [ ] (Optional) playwright snapshot tests pass.

## May-touch

- `web/app/compare/page.css` (~50 LOC additional `@media` blocks)
- `web/app/changelog/page.css` (~50 LOC additional `@media` blocks)
- `web/tests/e2e/marketing-mobile-snapshots.spec.ts` (optional NEW — ~60 LOC)

## Out of scope

- Other marketing pages' breakpoints (audit didn't flag them; verify separately if needed).
- The locked content / typography of either page.
- The hero animations on `/compare`.

## Effort

S — ~100-150 LOC across two CSS files + visual verification. ~2 hours. P2 because the camera demo stays at 1440, but mobile share-link traffic is the largest marketing-surface visitor cohort.
