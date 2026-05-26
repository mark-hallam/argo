# UX audit — Scenes 1 + 2 (Claims 1 + 2)

**Date**: 2026-05-27 00:15 AEST
**Scope**: `/workers` deck (Mariah / Peta / Priya / Jamal / Nori), `/workers/{id}` overview + tabs, the duties / "schedules" surface, the "Run now" affordance, the audit-row landing beat.
**Environment**: dev server (web port 3000), gateway port 7400, both UP at audit start. `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`.
**Method**: Claude Preview MCP for initial page-load + screenshots; preview hung partway, so most findings come from direct source-code reading + curl against the gateway + the runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`). Per the function-vs-dressing rule, anything the camera *clicks into* must be real; decorative numbers can stay theatrical.

---

## TL;DR

Scenes 1 + 2 are **not recordable as written** in the runbook. Three demo-stopping problems: (1) the entire `/team` (a.k.a. `/workers`) roster is mock data because the gateway has no `/api/v1/workers` endpoint, so the "stop the gateway and the same 5 workers come back" beat is theatre the camera sees; (2) there is no "Duties" or "Schedules" tab on the worker-detail page — duties live inline on the Overview tab and the named duties (`morning_kickoff`, `listing_health_check`, `weekly_summary`) don't exist in the SPA; (3) there is no "Run now" button on any duty row, and the gateway audit table has zero `worker_duty.fired` rows ever recorded. Total findings: 13 (P0 × 5, P1 × 3, P2 × 4, P3 × 1).

## Headline blockers

1. **F1 [P0]** — `/workers` deck is 100% mock data; "stop / restart gateway" beat is theatre. Scene 1.
2. **F2 [P0]** — No "Duties" or "Schedules" tab exists on the worker-detail page; Scene 2 step 1 is unrecordable.
3. **F3 [P0]** — No "Run now" affordance on any duty row; Scene 2 step 2 is unrecordable.
4. **F4 [P0]** — Gateway has zero `worker_duty.fired` audit rows; Scene 2 pass cue can never appear.
5. **F5 [P0]** — Runbook's specific duty IDs (`morning_kickoff` / `listing_health_check` / `weekly_summary`) do not exist anywhere in the SPA fixture.

---

## All findings

### F1 [P0] — `/workers` roster is mock data; "stop / restart gateway" beat is pure theatre

The Scene 1 narration ("I'm going to stop the gateway, restart it, and they're all still here. Same names. Same memory.") implies the workers come from the gateway and persist across a restart. They don't. The gateway returns `404 Not Found` on `/api/v1/workers`; the SPA falls through to `SAMPLE_TEAM` (`web/lib/sample-team.ts`), which is hard-coded mock data. The "MOCK-OFFLINE" badge is even visible on-camera next to the "Team" heading. Per the function-vs-dressing rule, this is theatre the camera sees: the demo asserts persistence as a *proof of claim 1*, but no persistence is actually happening.

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}` HTTP 404
- Open `/team` in the SPA → 5 workers appear with a visible "MOCK-OFFLINE" pill
- Stop the gateway, refresh — same 5 workers, because they never came from the gateway in the first place

Files:
- `~/code/argo/web/lib/api/team.ts:58-99` (fallback logic — `mock-bypass` in dev, `mock-offline` on 404)
- `~/code/argo/web/lib/sample-team.ts` (the 5 hard-coded workers)

Suggested fix: either land the gateway's `/api/v1/workers` endpoint with seeded data so the restart actually re-loads from disk (real), OR rewrite Scene 1 narration to drop the persistence claim and describe the workers as the canonical demo cast — but the latter loses the claim-1 proof.

---

### F2 [P0] — There is no "Duties" or "Schedules" tab on the worker-detail page

Scene 2 step 1: "click into her **Duties** tab. Three duties listed…". The worker-detail page has these tabs only: Overview, Activity, Memory, Skills, Trust, Escalation, Settings (`WORKER_TAB_IDS` in `web/components/skycot/WorkerTabs.tsx:19`). No Duties tab, no Schedules tab. Duties render inline on the Overview tab as `<DutiesCard>`. The operator can't click into a tab that doesn't exist.

Bonus footgun: typing `/workers/<id>/duties` returns a 200 page but the body renders the worker-shell's not-found boundary — not a crash, but visually broken.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001` (Mariah) → the tab strip has 7 tabs, none labelled Duties or Schedules
- Curl `http://127.0.0.1:3000/workers/018f67d0-0000-7000-8000-000000000001/duties` → title `· duties` but body renders not-found content

Files:
- `~/code/argo/web/components/skycot/WorkerTabs.tsx:19-39` (tab IDs + labels)
- `~/code/argo/web/components/skycot/worker/WorkerOverviewContent.tsx:151-159` (DutiesCard on Overview)
- `~/code/argo/web/app/(app)/workers/[workerId]/[tab]/page.tsx` (catch-all that calls `notFound()`)

Suggested fix: either re-script Scene 2 to say "scroll down to her Schedules card on Overview" (cheap; matches what's there), or add a real Schedules tab. Re-script is the right v1.0 call.

---

### F3 [P0] — There is no "Run now" affordance on any duty row

Scene 2 step 2: "Operator clicks 'Run now' on `morning_kickoff`". The only affordances on a `DutyRow` are (a) the schedule toggle (enable / disable) and (b) clicking the label to edit. The toggle's aria-label is "Enable schedule" / "Disable schedule" (correct vocab per `project_vocabulary_duty_vs_schedule`), and "Run now" appears in the codebase only on the workflow-detail page (`/workflows/[verticalId]/[name]`), not on the worker page. The worker overflow menu has Pause, Disable schedules, Archive, Export, View as Operator — also no run / trigger.

Even the existing edit click is stubbed: `onEditDuty` fires a transient toast `"Edit schedule ships in E4-h"`; `onAddDuty` fires `"Add responsibility dialog ships in E4-h"`. Both reveal "coming soon" copy on-camera.

Reproduction:
- Open `/workers/<id>` → Overview → DutiesCard
- No button labelled "Run now" anywhere on the page
- Click a duty label → tiny toast appears with "ships in E4-h" copy

Files:
- `~/code/argo/web/components/skycot/worker/DutiesCard.tsx:138-197` (DutyRow — toggle + edit only)
- `~/code/argo/web/components/skycot/worker/WorkerOverviewContent.tsx:74-93` (stub toasts)
- `~/code/argo/web/components/skycot/WorkerOverflowMenu.tsx` (no run action)

Suggested fix: this is the single highest-leverage demo blocker. Either (a) add a real "Run now" button per row wired to a gateway run-duty endpoint, or (b) re-script Scene 2 around the existing `/workflows/<vertical>/<name>` "Run now" button (which IS real — see `web/lib/api/workflow-execute.ts`). Option (b) is the v1.0 path and matches Scene 5; option (a) is a separate brief.

---

### F4 [P0] — Gateway has zero `worker_duty.fired` rows; Scene 2 pass cue is unsatisfiable

Scene 2 pass cue: "One `worker_duty.fired` row appears in `/audit` within 60 seconds, tagged with the worker's `tenant_id` and the fired `duty_id`." The gateway's `/api/v1/audit?action_type=worker_duty.fired` returns `{"items":[]}`. The audit table currently shows only `audit.detail.viewed`, `audit.replay`, and `http.request` rows — all "unknown" worker. Even if a Run-now button existed (F3), the audit pipeline currently produces no `worker_duty.fired` kind for the SPA to filter on.

Secondary issue: the gateway ignores the runbook's filter spelling `?kind=`. The SPA uses `?action_type=`. The runbook should be amended to match the SPA — but the deeper problem is no row of that kind exists at all.

Reproduction:
- `curl -s 'http://127.0.0.1:7400/api/v1/audit?action_type=worker_duty.fired'` → `{"items":[]}` HTTP 200
- `curl -s 'http://127.0.0.1:7400/api/v1/audit?kind=worker_duty.fired'` → returns *all* rows (filter ignored)

Files:
- gateway audit handler (not in scope to identify) — the kind filter is silently ignored
- `~/code/argo/web/lib/filters/audit.ts:76-80` (URL key is `action_type`, not `kind`)

Suggested fix: depends on F3 resolution. If Scene 2 pivots to a workflow run, the pass cue becomes `workflow.step.executed` (Scene 3's kind), which DOES have a code path. Drop the `worker_duty.fired` claim from the runbook unless the duty-cron-firing pipeline is shippable on the camera-side.

---

### F5 [P0] — Runbook's duty IDs (`morning_kickoff` / `listing_health_check` / `weekly_summary`) do not exist anywhere

The runbook narrates the three duty names by ID + cron string. None of these strings appear in the SPA codebase. Mariah's actual duties (from `web/lib/sample-worker-overview.ts:79-87`) are: "List new inventory" / "Re-price stale listings against competitor data, daily 06:30" / "Respond to buyer questions" / "Flag items dropping below floor". Four duties, not three; different labels and crons.

A diligent operator reading the runbook will narrate one set of words while the camera captures a different set — instant credibility loss in the recording.

Reproduction:
- `rg -l 'morning_kickoff|listing_health_check|weekly_summary' ~/code/argo/web/` → zero hits
- Open Mariah Overview → see four duties with different labels

Files:
- `~/code/argo/web/lib/sample-worker-overview.ts:79-87` (Mariah's actual duties)
- `~/code/argo/docs/skycot/demos/v1-mission-mvp-runbook.md:139-141` (runbook's fictional duties)

Suggested fix: rewrite the runbook to use Mariah's actual duty labels and counts, OR change the seed data to match the runbook. Either way they must align before recording.

---

### F6 [P1] — Scene 1 narration references "memory counts in the badge corners"; the badges don't exist

The Scene 1 talk-track says "same names, same avatars, same memory counts in the badge corners". The `TeammateCard` has a level badge ("LEVEL 5" etc.), a status pill ("Waiting on you"), and traits — but no memory count visible anywhere on the card. `rg memory_count|memoryCount /web/components/skycot` returns nothing. If the operator points at "memory counts" on the recording, the camera will catch them gesturing at nothing.

Suggested fix: drop the memory-count phrase from the narration, or add a memory-count chip to the card. The level number is the closest existing visual hook.

Files: `~/code/argo/web/components/skycot/TeammateCard.tsx` (no memory-count slot)

---

### F7 [P1] — Peta has no traits chips on her trading card; falls back to hash-derived level (18)

The TRAITS_BY_NAME map in `TeammateCard.tsx:51-58` covers Mariah / Priya / Jamal / Nori / Sam / Rio. **Peta is missing.** Her card renders without a TRAITS line; her level (18) is derived from a string-hash fallback (`hashLevel`) instead of the curated map. The accessibility snapshot at audit start confirms: Peta's card has no traits whatsoever while every other worker has 4. Since Scene 1 pans across all 5 cards, Peta will look visibly less finished than her teammates.

Files: `~/code/argo/web/components/skycot/TeammateCard.tsx:51-67`

Suggested fix: add Peta to TRAITS_BY_NAME + LEVEL_BY_NAME with curated values matching her Project Ops persona.

---

### F8 [P1] — Peta and Nori have no curated personality on the worker-detail page

Same shape as F7 but on the detail page. `WorkerPersonalityCard.tsx:35-96` defines PERSONALITIES for Mariah / Priya / Jamal only. Peta and Nori (and the new Patrick) fall through to `DEFAULT_PERSONALITY`: traits "Reliable, Curious, Helpful", generic "great at / escalate when / signature" bullets. Scene 1 only opens Mariah so this is a P1 not a P0, but if the operator's hand slips to Peta during the take, the difference is conspicuous.

Files: `~/code/argo/web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`

---

### F9 [P2] — Sidebar duplicates "Team" and "Workers" — same destination, two labels

Sidebar (`web/components/skycot/nav-items.tsx`) has both `{ href: "/team", label: "Team" }` and `{ href: "/workers", label: "Workers" }`. `/workers` is a redirect-to-`/team` shim (`web/app/(app)/workers/page.tsx`). Two adjacent nav items pointing at the same surface invites the "which one is right?" hesitation on-camera, and adds vocabulary noise (the brand voice elsewhere is "teammates", not "workers"). Not a blocker; the sidebar is in shot only briefly.

Suggested fix: drop one. "Team" matches the page heading ("Team"), brand vocabulary, and the URL the user lands on.

Files: `~/code/argo/web/components/skycot/nav-items.tsx:22-24`, `~/code/argo/web/app/(app)/workers/page.tsx`

---

### F10 [P2] — Hero stats are dressing labels on a function-side surface — risky mix

The `/team` hero shows "0 working right now · 935 actions this week · saving you ~60 hours" and a 3-up stat strip (WORKING 0 / ACTIONS/WK 935 / HOURS SAVED 60). The "0 working" updates with the real worker statuses but "935" and "60" are static fixtures. Per the function-vs-dressing rule these are dressing if the camera doesn't drill into them — and Scene 1 doesn't. But two of the three numbers being demonstrably fake + one being real is a credibility split-brain if a viewer pauses on the frame. P2 because the runbook doesn't linger here.

Files: search for the BrandLoudHero stats source (`~/code/argo/web/components/skycot/BrandLoudHero.tsx` + the team page).

Suggested fix: either compute both numbers from sample data so they at least align, or hide them in demo mode.

---

### F11 [P2] — All 5 trading cards show identical "Waiting on you" status

Every card in the snapshot showed the same "Waiting on you" pill. That implies the SAMPLE_TEAM fixture has the same status for everyone (verified — Mariah / Priya / Peta = `waiting`; Jamal = `working`; Nori = `idle`), but the visual rendering may be collapsing them. Worth eyeballing on a real take: if all five show "Waiting on you", that's the opposite of the "5 teammates, one team" hero — the deck looks like one big task list.

Suggested fix: confirm the visual rendering reflects each worker's actual status (the data is varied; the snapshot suggests the UI may not be).

Files: `~/code/argo/web/components/skycot/TeammateCard.tsx` (STATUS_LABELS + status badge render)

---

### F12 [P2] — Edit-duty + Add-duty buttons reveal "ships in E4-h" copy on click

Already covered as a side-effect of F3 but worth its own line. If the operator's mouse drifts onto a duty label during Scene 2, a toast appears with `Edit schedule ships in E4-h` — visible engineering language ("E4-h" is a brief ID) on camera. Mitigation: don't click the labels during the take. But this is exactly the kind of footgun pre-record walkthroughs are meant to catch.

Suggested fix: replace the toast with "Coming soon" or remove the click handler entirely until the real edit lands.

Files: `~/code/argo/web/components/skycot/worker/WorkerOverviewContent.tsx:74-93`

---

### F13 [P3] — Audit page filter URL key (`action_type`) vs runbook vocabulary (`kind`)

Runbook says "Filter by `kind = worker_duty.fired`." The actual URL is `?action_type=worker_duty.fired`. Internal-only; doesn't affect the recording (the operator clicks a filter chip, doesn't type a URL). Worth a doc fix in the runbook for accuracy.

Files: `~/code/argo/web/lib/filters/audit.ts:76-80`

---

## Resolved since last audit

(No prior audit in `docs/skycot/ux-audits/` for this scope.)

## Orientation score

**46 / 100.**

The visual scaffolding is in place — the C2 trading-card deck looks great, the worker-detail layout is clean, the duty-card design is recognisable as the operator-locked design. But Scenes 1 + 2 cannot be recorded as written: the persistence story is mock, the Duties tab doesn't exist, the Run-now button doesn't exist, and the audit pipeline produces no row of the demanded kind. None of these are pixel-polish problems; they are gaps between the runbook's narrative claims and the working software. The cheapest path to a recordable demo is to **rewrite Scenes 1 + 2 around what actually works** (drop the gateway-restart proof, point at the inline Schedules card on Overview, use `/workflows/<id>` "Run now" instead of a duty Run-now), then revisit the gateway-side proof in a v1.1 cycle.
