# fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F7 [P1]).

🟠 **P1 — `web/app/for/_data/verticals.ts` introduces 3 named workers in signature-workflow cards (Skylar in sales, Fen in finance, Mira in marketing) that do NOT exist anywhere else in the product. The canonical demo roster is Mariah / Peta / Priya / Jamal / Nori — a visitor reading `/for/sales` who clicks through to `/team` will NOT find Skylar. The vertical pages introduce 3 worker names the customer cannot find in the product.**

## Root cause

`web/app/for/_data/verticals.ts` includes `actor: "<Name>"` in the per-vertical signature-workflow definitions. Audit verification:

```
grep -nE 'actor: "[A-Z]' web/app/for/_data/verticals.ts
```

Returns Skylar (sales, lines 90-94), Fen (finance, lines 246-249), Mira (marketing, lines 298-301). Recruiting (Priya), Support (Jamal), Ops (Patrick) ARE canonical.

The canonical 5 per memory `project_pm_cohorts_peta_patrick` + Scene 1 lockup:
**Mariah / Peta / Priya / Jamal / Nori** (Patrick is also canonical as the second Project & Ops worker but not in the deck-of-5).

Same shape as `fix-workflow-card-roster-violations-rio-and-canonical-personas` (which fixed Rio in /workflows). This brief covers the parallel violation in /for/[vertical] data.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F7 [P1] — Marketing perimeter introduces non-canonical worker names (Skylar, Fen, Mira)

## Fix recipe

1. **Map the 3 names to canonical replacements**:

   | Non-canonical | Vertical | Suggested replacement | Rationale |
   |---|---|---|---|
   | Skylar | sales | Mariah | Mariah is the sales-shaped canonical; matches Scene-1 lockup |
   | Fen | finance | Nori | Nori is the closest fit on the canonical roster for finance work (per audit + memory) |
   | Mira | marketing | Peta or Mariah | Peta has the most marketing-adjacent shape (PM/comms); Mariah also works |

2. Edit `web/app/for/_data/verticals.ts`:
   - Line 90-94: Skylar → Mariah
   - Line 246-249: Fen → Nori
   - Line 298-301: Mira → Peta (or Mariah; operator pick)

3. Regression-grep across the whole repo for any other leak of these names into customer copy:

   ```bash
   rg -nE '\b(Skylar|Fen|Mira|Sam|Rio|Maya)\b' web/app web/components web/lib --type ts --type tsx
   ```

   Replace any remaining customer-visible occurrences. Internal test fixtures or comments can stay if uniquely identifiable as non-canonical.

4. Add a regression test that fails CI if any of those names appear in `web/app/(for|landing|customers|compare|pricing|auth|onboarding)/**`:

   ```ts
   // web/lib/__tests__/canonical-roster-only-in-customer-copy.test.ts
   const FORBIDDEN = ['Skylar', 'Fen', 'Mira', 'Sam', 'Rio', 'Maya'];
   // ... glob + readFile + assert no forbidden name appears
   ```

## Acceptance criteria

- [ ] `rg -nE '\b(Skylar|Fen|Mira)\b' web/app/for/` returns zero matches.
- [ ] `rg -nE '\b(Skylar|Fen|Mira)\b' web/app/` returns zero matches outside test directories.
- [ ] Every `actor` in `web/app/for/_data/verticals.ts` is one of the canonical 5 (Mariah / Peta / Priya / Jamal / Nori) or Patrick.
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run` no regressions.
- [ ] (Optional) regression lint shipped that prevents new occurrences.

## May-touch

- `web/app/for/_data/verticals.ts` (~10 LOC — 3 name swaps)
- Any other file the regression grep flags (low expected count)
- New `web/lib/__tests__/canonical-roster-only-in-customer-copy.test.ts` (~50 LOC, optional but recommended)

## Out of scope

- /workflows Rio violation (covered by `fix-workflow-card-roster-violations-rio-and-canonical-personas`)
- /team SAMPLE_TEAM (already canonical — verified during audit)
- Peta absence from workflow cards (covered by `fix-sample-workflows-add-peta-as-editor`)
- The cohort canonical mapping itself (operator-locked via `project_pm_cohorts_peta_patrick`)

## Effort

S. ~15 LOC + ~50 LOC test. ~45 min.
