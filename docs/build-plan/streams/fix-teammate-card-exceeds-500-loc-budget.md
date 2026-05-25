# fix-teammate-card-exceeds-500-loc-budget

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-teammate-card-exceeds-500-loc-budget`.
**Source**: e2e-ux-audit Agent 2 finding F6 (`docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🟡 **P1 — `TeammateCard.tsx` is 582 LOC. Violates the 500-LOC CLAUDE.md invariant. Any change trips CI.**

## Root cause

`web/components/skycot/TeammateCard.tsx` is currently 582 lines per `wc -l`. CI invariant per `CLAUDE.md`:

> No file > 500 LOC

is currently violated. Confirmed via `wc -l web/components/skycot/TeammateCard.tsx` → 582.

The file is bloated by:
- `TRAITS_BY_NAME` map (~30 LOC)
- `LEVEL_BY_NAME` + `hashLevel` (~20 LOC)
- A large `<style jsx>` block (~150 LOC)
- The card render body itself (~300 LOC)

Note: `fix-worker-personality-and-level-derive-from-real-data` plans to DELETE `TRAITS_BY_NAME` + `LEVEL_BY_NAME` + `hashLevel` entirely (because they're load-bearing for the name-keyed personality bug). That brief alone might bring this file under 500 LOC. If shipped first, this brief becomes much smaller.

## Findings this brief closes

- Agent 2 F6 [P1] — `TeammateCard.tsx` is 582 LOC (exceeds the project's 500-LOC budget)

## Fix recipe

This brief sequences AFTER `fix-worker-personality-and-level-derive-from-real-data`. After that brief lands, check the new LOC count:

- If `wc -l TeammateCard.tsx` < 500, this brief is STALE-FIXED. Mark accordingly.
- If still > 500: extract the `<style jsx>` block to a sibling file:
  1. Create `web/components/skycot/TeammateCard.styles.tsx` exporting the style block as a component.
  2. Import + render in `TeammateCard.tsx`.
  3. Confirm visual parity with a screenshot diff.

Optional secondary extraction (if needed): split the render body into sub-components (`<TeammateCardHero>`, `<TeammateCardTraits>`, `<TeammateCardFooter>`) each in its own sibling file under `web/components/skycot/teammate-card/`.

## Acceptance criteria

- [ ] `wc -l web/components/skycot/TeammateCard.tsx` returns < 500.
- [ ] All sibling files created are also < 500 LOC.
- [ ] Visual parity with the pre-refactor card (screenshot diff in PR).
- [ ] No regression on the C2-holographic-showcase design memory `project_workers_page_design_locked`.
- [ ] CI loc-budget lint passes.

## May-touch

- `web/components/skycot/TeammateCard.tsx` (the main extraction)
- `web/components/skycot/TeammateCard.styles.tsx` (NEW)
- Possibly: `web/components/skycot/teammate-card/*.tsx` (NEW siblings if secondary extraction is needed)
- Snapshot tests

## Out of scope

- The name-keyed personality / level maps (separate brief: `fix-worker-personality-and-level-derive-from-real-data` — this brief sequences after it)
- Wider design refactor (locked per `project_workers_page_design_locked`)

## Effort

~50-200 LOC of extraction (or zero if the personality brief ships first). ~30-90 min.
