# fix-skills-library-chip-is-decorative-not-filter

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-skills-library-chip-is-decorative-not-filter`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — Every skill card on `/skills` shows a "LIBRARY" chip, but the chip isn't interactive — clicking does nothing. Operators expect chips next to filter affordances to filter the catalog.**

## Root cause

> Every skill card shows a `LIBRARY` tag, but the tag isn't a link/filter — clicking does nothing. Operator would expect this to filter the catalog to library skills, especially since the search bar offers "approval / invoice / candidate / deal / support / lead" pill-shortcuts but no scope filter.
> — `whole-site-mvp-claims-coverage.md` F07

The `LIBRARY` chip is currently a static label, not a filter token. Compare to the suggested-search pills above the catalog grid which ARE filters. Mixing interactive and decorative chips on the same surface is a confusing UX — the chip's affordance reads as a filter shortcut by sibling-association.

The strategic question is whether the catalog should have a Library/Marketplace/Mine filter dimension at all, or whether the chip should be re-labelled to something inert (e.g. "VERIFIED").

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F07 [P2] — /skills marketplace `LIBRARY` chip doesn't link anywhere

## Fix recipe

Two operator-decisional options:

### Option A — Make the chip a filter

1. In `web/app/(app)/skills/page.tsx`, add a `LibraryFilter` state next to the existing search filter.
2. Render the `LIBRARY` chip on each card as a clickable filter token (e.g. `<button onClick={() => setLibraryFilter("library")}>LIBRARY</button>`).
3. Add a "Library / Marketplace / Mine" segmented control above the catalog grid for explicit scope switching.
4. When `LibraryFilter !== "all"`, filter the rendered cards to that scope.

### Option B — Make the chip decorative (lower-risk)

1. Rename the chip to something that doesn't imply filter behaviour:
   - `VERIFIED` (matches the gateway's `trust_grade = "published_verified"` from the 7-skill boot-seed)
   - `LIBRARY · v0.1` (informational version stamp)
2. Style as a passive tag (`cursor: default`, no hover state).
3. Document in a code comment why this is non-interactive ("Library scope is the default; future scope filter belongs in the segmented control, not the per-card tag.").

Option B is the demo-day fix. Option A is the better product but requires UX time and a locked design call.

## Acceptance criteria

- [ ] The `LIBRARY` chip behaviour is consistent with operator expectation:
  - Option A: clicking filters the catalog.
  - Option B: chip is visibly decorative (`cursor: default`) and re-labelled.
- [ ] No regression in the search bar's try-pills (existing filter behaviour preserved).
- [ ] Component test at `web/app/(app)/skills/SkillCard.test.tsx` covers the chosen behaviour.
- [ ] PR body cites finding F07.

## May-touch

- `web/app/(app)/skills/page.tsx`
- `web/components/skycot/skills/SkillCard.tsx` (or wherever the chip is rendered)
- `web/app/(app)/skills/SkillCard.test.tsx` (NEW or extend)

## Out of scope

- Try-chips zero results (covered by `fix-skills-try-chips-zero-results-rewrite`)
- Skill detail install button (covered by `fix-skills-detail-install-button-and-flow`)
- Icon rotation semantics (covered by `fix-skills-marketplace-card-icon-rotation-semantic-name-map`)

## Effort

S — Option B: ~20-50 LOC + 1 test. ~1 hour. Option A: ~150-300 LOC + design alignment. ~4-8 hours.
