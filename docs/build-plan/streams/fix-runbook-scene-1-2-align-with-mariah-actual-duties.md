# fix-runbook-scene-1-2-align-with-mariah-actual-duties

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-runbook-scene-1-2-align-with-mariah-actual-duties`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F5 [P0] + F6 [P1]).

🟠 **P1 (doc-only) — The v1.0 demo runbook narrates Scene 2 around duty IDs `morning_kickoff` / `listing_health_check` / `weekly_summary` that DO NOT EXIST anywhere in the SPA, and references "memory counts in the badge corners" that don't appear on any teammate card. A diligent operator reading the runbook will narrate one set of words while the camera captures a different set — instant credibility loss in the recording.**

## Root cause

Two narrative-vs-fixture mismatches the camera will catch:

1. **Duty IDs don't exist.** Runbook at `docs/skycot/demos/v1-mission-mvp-runbook.md:139-141` narrates 3 specific duty IDs (`morning_kickoff`, `listing_health_check`, `weekly_summary`) with associated cron strings. `rg -l 'morning_kickoff|listing_health_check|weekly_summary' ~/code/argo/web/` returns ZERO matches. Mariah's actual duties (from `web/lib/sample-worker-overview.ts:79-87`):
   - "List new inventory"
   - "Re-price stale listings against competitor data, daily 06:30"
   - "Respond to buyer questions"
   - "Flag items dropping below floor"

   Four duties, not three; different labels, different crons.

2. **"Memory counts in the badge corners" don't exist.** Runbook Scene 1 talk-track says "same names, same avatars, same memory counts in the badge corners". `TeammateCard` (`web/components/skycot/TeammateCard.tsx`) renders: level badge ("LEVEL 5"), status pill ("Waiting on you"), and traits chips — NO memory count visible anywhere on the card. `rg 'memory_count|memoryCount' web/components/skycot` returns nothing. If the operator points at "memory counts" on recording, the camera will catch them gesturing at nothing.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F5 [P0] — Runbook's duty IDs (`morning_kickoff` / `listing_health_check` / `weekly_summary`) do not exist anywhere
- `docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F6 [P1] — Scene 1 narration references "memory counts in the badge corners"; the badges don't exist

## Fix recipe

Operator-decisional (doc or code); recommend (a) for v1.0 timeline.

**(a) Edit the runbook to match what's there** (recommended — narrowest scope):

1. Open `docs/skycot/demos/v1-mission-mvp-runbook.md`. Rewrite Scene 2 step 1 to narrate against Mariah's actual 4 duties:
   > "Click into her Schedules card on the Overview tab. Four things she's responsible for — `List new inventory`, `Re-price stale listings`, `Respond to buyer questions`, `Flag items dropping below floor`."
2. Drop the "memory counts in the badge corners" phrase from Scene 1. Replace with what the card actually shows:
   > "Same names, same avatars, same trait chips — and notice the **level** badges. Mariah's at L5, Peta at L18."
3. Cross-reference the level badge / trait chips that DO render (per `TeammateCard.tsx:51-67` TRAITS_BY_NAME + LEVEL_BY_NAME — verify Peta is present after `fix-peta-personality-trait-level-lookup-coverage` lands).

**(b) Edit the seed to match the runbook** (more work, paired with `mission-demo-scene-2-duties-runnow-renders-from-seed`):

1. Rename Mariah's 4 duties in `web/lib/sample-worker-overview.ts:79-87` to `morning_kickoff` / `listing_health_check` / `weekly_summary` (drop one, rename three). Re-check `crates/argo-demo-seed/src/worker.rs` for the matching seed.
2. Add a `memory_count` field to `WorkerSummary` and render it in the badge corner on the `TeammateCard`.

For the v1.0 demo MP4 cut: pick (a). It's a doc edit; the camera-side story stays factually anchored. (b) is the v1.x richer-data path.

## Acceptance criteria

- [ ] `rg -l 'morning_kickoff|listing_health_check|weekly_summary' docs/skycot/demos/` returns zero matches (or, if (b) shipped, returns the seed file + the runbook).
- [ ] `rg -l 'memory count' docs/skycot/demos/v1-mission-mvp-runbook.md` returns zero matches (or returns the file but pointing at a card field that exists).
- [ ] An operator reading the runbook and looking at the rendered SPA at `/workers/<mariah-id>` sees the labels the runbook narrates.
- [ ] Pre-record dry-run with this brief shipped: operator can walk Scene 1 + 2 without pointing at non-existent UI.

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Option a — ~10 LOC of narration tweaks)
- `web/lib/sample-worker-overview.ts` (Option b — ~30 LOC duty renames)
- `crates/argo-demo-seed/src/worker.rs` (Option b — ~30 LOC matching seed renames)
- `web/components/skycot/TeammateCard.tsx` (Option b — ~10 LOC memory_count chip)

## Out of scope

- Adding a Schedules / Duties TAB to the worker-detail page (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end`)
- Run-now button on duty rows (covered by `mission-demo-scene-2-duties-runnow-renders-from-seed`)
- Seeding agent-execution audit rows (covered by `fix-demo-seed-emit-worker-hired-audit-rows`)

## Effort

XS for option (a) (~10 LOC doc edit, ~15 min). M for option (b) (~80 LOC across SPA + seed + new card field). Pick (a) for v1.0 demo cut.
