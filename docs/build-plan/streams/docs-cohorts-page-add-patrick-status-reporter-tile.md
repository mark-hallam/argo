# Stream docs-cohorts-page-add-patrick-status-reporter-tile — add Patrick (M22 Status Reporter) tile to cohorts marketing page

**Owner**: claude (web/UX; ~40-80 LOC TSX + small CSS adjustment).
**Depends on**: #2259 MERGED (M22 Status Reporter pack live on main).
**Authored against**: origin/main `be9206ef2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-cohorts-page-add-patrick-status-reporter-tile`.

🟢 **P3 marketing alignment.** Source: round-51 evaluator finding, 2nd-consecutive carry-forward. PR #2259 shipped the M22 Status Reporter vertical pack (`Patrick` worker, Project & Ops cohort). The cohorts marketing page (`web/app/cohorts/page.tsx` or similar — the page that displays the 7-dept cohort tiles per the design-locked memory) does NOT yet show Patrick. Operator-locked design (project_cohorts_page_design_locked, 2026-05-19) places Project & Ops as the 7th dept tab with Peta (L2 Project Coordinator) as a lead tile — Patrick (L3 Status Reporter) is the second tile and is missing.

## Scope

Add Patrick's tile to the cohorts page in the Project & Ops dept tab. Tile content per the D3 magazine-tiles spec:

- Worker name: **Patrick**
- Level: **L3** (Status Reporter)
- Stack: **Linear · GitHub · Slack**
- Hero one-liner: written to match Peta's tone ("Coordinates work" / verb-first phrasing). Suggested: *"Reports work, status, and risks weekly across teams."*
- Avatar / portrait placeholder (reuse Peta's portrait style — generated illustration, magazine-feature aesthetic).
- "Great at" + "Escalates when" chips per the C2-personality pattern.

Two tile layouts to choose from per the D3 locked design (lead/stacked/compact):
- If Patrick goes in the LEAD slot, demote Peta to stacked.
- Default: add Patrick as the second STACKED tile next to Peta.

Pre-flight: grep `web/app/cohorts/` (and `web/app/` for any sibling cohort routes) to confirm the file structure. The exact path is operator-locked but the file may have moved — verify on current main before edits.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`web/app/cohorts/page.tsx`]

## Files you may touch

- The cohorts page TSX (find via `rg -nE "Project & Ops|Peta|Project Coordinator" web/app/`).
- A small fixture or constants file if cohort data is structured (e.g. `web/lib/cohorts-data.ts`).
- Avatar / image asset under `web/public/cohorts/` if a Patrick portrait needs to land.
- A Playwright snapshot test or Storybook fixture if those exist for cohorts.

## Tests

- Visual: a Playwright e2e snapshot covering the Project & Ops tab containing both Peta and Patrick tiles.
- Unit: any existing cohort-render test should still pass; extend with a Patrick presence assertion.

## Definition of done

- Patrick tile renders in Project & Ops dept tab in the D3 magazine-tiles layout.
- `pnpm test --filter web` green.
- PR body includes a screenshot of the updated cohorts page (Project & Ops tab).
- Patrick metadata (level, stack, hero one-liner) matches the M22 pack manifest fields.

Notes: this is marketing surface, not gateway behavior. Don't bundle with logic changes. Patrick's tile copy should be the operator's call — if uncertain about the hero line, PR-body the proposed text and ask before shipping.
