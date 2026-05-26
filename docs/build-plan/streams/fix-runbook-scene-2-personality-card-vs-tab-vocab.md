# fix-runbook-scene-2-personality-card-vs-tab-vocab

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-runbook-scene-2-personality-card-vs-tab-vocab`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-1-and-2-claim-1-and-2.md` F10 [P2] + F4 [P2] (sibling: onboarding F5 / F1 tab-title).

🟢 **P2 — Doc-only fix. The v1.0 runbook narrates "click into her Personality tab" + "Schedules tab shows 3 duties". The implementation has Personality as a CARD on the Overview tab (per locked design memory `project_worker_detail_design_locked`), and Schedules is not a tab at all. The runbook talk-track needs to match what the SPA delivers; otherwise the camera narration says "Personality tab" and the operator clicks Overview.**

## Root cause

Two parallel sources of truth disagreed:

1. **Locked design** (memory `project_worker_detail_design_locked`, dated 2026-05-19): Worker detail is a LinkedIn-style profile. Personality is realised as `WorkerPersonalityCard` at the top of the Overview tab — NOT its own tab.
2. **Runbook** at `docs/skycot/demos/v1-mission-mvp-runbook.md` (current text): Scene 2 references "Personality tab" and "Schedules tab" as if both are tabs.

The actual tab list (`web/components/skycot/WorkerTabs.tsx:19-27`) is: Overview, Activity, Memory, Skills, Trust, Escalation, Settings. Personality is rendered inside Overview. Schedules doesn't exist (covered by sibling brief `fix-worker-detail-responsibilities-schedules-tab-end-to-end`).

This brief is the doc-only half. Until the Schedules tab actually exists, the runbook needs to describe what the camera can see.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F10 [P2] — Personality is a card on Overview, not its own tab; runbook says otherwise
- scene-1-and-2-claim-1-and-2 F4 [P2] (partial) — `/workers` → `/team` URL-flash; runbook says "land on `/workers`"

## Fix recipe

Edit `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 2 narration:

1. **Replace**: "click into Mariah's Personality tab" → "look at her Personality card on Overview".
2. **Replace**: "Schedules tab shows 3 duties" → either:
   - (a) "click Activity to see her past 3 runs" (if `fix-demo-seed-emit-worker-hired-audit-rows` has shipped),
   - (b) wait for `fix-worker-detail-responsibilities-schedules-tab-end-to-end` to add the real tab, then restore the Schedules narration, OR
   - (c) drive from `/workflows/<vert>/<name>` Run-now (per the sibling brief `fix-worker-detail-run-now-action-and-endpoint`'s Option B).
3. **Replace**: "land on `/workers`" → "land on `/team`" (until `fix-workers-redirect-to-team-url-flicker` lands).
4. **Add a runbook footnote**: "Tab labels and click-paths are pinned against `web/components/skycot/WorkerTabs.tsx` — if you rename a tab, re-check this scene."

## Acceptance criteria

- [ ] Runbook Scene 2 narration uses words that match the on-camera UI exactly.
- [ ] Footnote pinning to the tab source file is present.
- [ ] An operator with no SPA experience can read Scene 2 + sit in front of the demo + record without surprise.
- [ ] The next ux-audit-agent that drives the runbook does NOT find a vocab mismatch in Scene 2.

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 2 narration block — verify line range)

## Out of scope

- Adding a real Schedules tab (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end`)
- Run-now affordance on worker detail (covered by `fix-worker-detail-run-now-action-and-endpoint`)
- `/workers` → `/team` redirect (covered by `fix-workers-redirect-to-team-url-flicker`)

## Effort

~10-20 lines doc edit. ~15-30 min. P2 because the runbook is operator-facing and the demo recording depends on its accuracy.
