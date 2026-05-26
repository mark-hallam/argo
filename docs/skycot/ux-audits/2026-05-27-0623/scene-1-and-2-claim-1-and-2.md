# UX e2e audit — Scenes 1+2 (Claims 1+2)

**Date**: 2026-05-27 06:23 AEST
**Scope**: `/workers` deck · `/workers/{id}` profile + tabs · Schedules/Duties · Run-now affordance · Audit drill-in from worker
**Mode**: read-only against code + live gateway on :7400. Dev SPA was NOT booted this run — `curl http://127.0.0.1:7400/api/v1/team` returns only **1 worker (Maya)**, so the 5-roster deck the camera films cannot render until `argo demo seed --tenant <demo>` is run. Findings derived from code reading + live gateway probes.
**Auditor**: e2e-ux-journey-test-agent
**Prior run**: 2026-05-27 05:19 AEST (same scope)

---

## TL;DR

Scene 1+2 are **still not recordable**. Zero of the 17 fix-briefs queued at T0519 (~1h ago) have merged. The two T0519MEL P0s still claimed (`fix-worker-detail-run-now-action-and-endpoint`, `fix-spa-demo-live-mode-route-real-gateway-on-camera`) remain in flight; no new code has shipped against either since claim. **Net-new finding this pass: the gateway is currently seeded with only 1 worker ("Maya"), not the canonical 5 (Mariah/Peta/Priya/Jamal/Nori) — the demo-mode seed CLI hasn't been run on this gateway boot.** That's a Scene-1 blocker on its own, separate from the SPA defects. Found **3 P0** (1 net-new), **4 P1** (1 net-new), **3 P2**, **2 P3**.

## Headline blockers

1. **P0 — Gateway only has Maya (1 worker), not the 5-worker canonical roster.** Scene 1's "Mariah/Peta/Priya/Jamal/Nori" deck cannot render. `argo demo seed --tenant <demo>` must run on the current gateway boot before recording.
2. **P0 — Still no Schedules tab + still no Run-now button on the worker profile.** Scene 2 unrecordable as scripted. (carried T0519 F3+F7; brief `fix-worker-detail-run-now-action-and-endpoint` CLAIMED, not landed.)
3. **P0 — Worker Overview "Recent activity" card hrefs still route to placeholder audit-detail.** `ap_<name>_N` IDs match neither `SAMPLE_AUDIT_DETAILS` nor `SAMPLE_AUDIT_ROWS`. (carried T0519 F12.)

---

## All findings

### Scene 1 — `/workers` deck + `/workers/{id}` hero & personality

#### F1 [P0] — Gateway seed missing: only "Maya" present (1 worker, not the 5-name canonical roster)

**Net-new this pass.** `curl http://127.0.0.1:7400/api/v1/team` returns one worker — `Maya`, vertical `marketing-intern`, status `idle`, headline "No activity yet." Scene 1's voiceover explicitly names Mariah / Peta / Priya / Jamal / Nori. The SPA's `/team` page renders whatever the gateway returns, so the camera would film a single tile for an unrelated persona. The fallback path in `web/lib/api/team.ts` only kicks in if the gateway is unreachable (HTTP 0/500); a successful 200 with N=1 prevents the mock-bypass fallback from firing.

This is a function-side blocker (the demo narrates 5 by name; the visible deck must match). The fix is operational — run the canonical seed CLI documented in `docs/skycot/demos/v1-mission-mvp-runbook.md` line 64-77 — but the runbook step has not been executed against the current gateway boot.

- Repro: `curl -fsS http://127.0.0.1:7400/api/v1/team | jq '.workers[].name'` → expect `Mariah Peta Priya Jamal Nori`, observed `Maya`.
- File: operational; runbook expects `argo demo seed --tenant $ARGO_DEMO_TENANT_ID`.
- Fix: re-run the seed CLI; add the seed-status check to the pre-record checklist's automation (`scripts/demo-dry-run.sh --demo-mode` should fail loudly if `workers.length != 5`).

#### F2 [P0] — `WorkerPersonalityCard` STILL missing entries for Peta + Nori (2 of 5 demo workers fall through to "Reliable / Curious / Helpful")

Carried from T0519 F1. `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` declares PERSONALITIES for Mariah / Priya / Jamal only. Peta and Nori fall through to the `DEFAULT_PERSONALITY` constant at line 98 ("Reliable / Curious / Helpful" traits, three generic great-at items, three generic escalate items). The persona spine in `web/lib/sample-personas.ts` has canonical traits for Peta + Nori; they aren't being threaded.

Camera visibility: this card is the FIRST card on every worker overview page (`web/app/(app)/workers/[workerId]/overview/page.tsx:24`), labelled "HOW {NAME} WORKS". If the camera clicks Peta or Nori during the Scene-1 click-through tour, the viewer reads canned generic content under "HOW PETA WORKS" / "HOW NORI WORKS".

- File: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96`
- Existing brief: `fix-peta-personality-trait-level-lookup-coverage`, `fix-worker-personality-coverage-extend-to-nori` — both authored T0519, neither merged.
- Fix: add PETA + NORI entries to the PERSONALITIES map keyed by persona ID (not name — see T0519 F2 for the ID-keying nit).

#### F3 [P0] — Still no "Schedules" tab + still no "Run now" button anywhere on the worker profile

Carried from T0519 F3 + F7. `WorkerTabs` (`web/components/skycot/WorkerTabs.tsx:19-39`) declares 7 tabs — overview / activity / memory / skills / trust / escalation / settings — no Schedules tab. Duties currently render as a sub-card on the Overview tab via `DutiesCard.tsx`. There is no Run-now / Trigger / Execute control anywhere on `/workers/{id}/*`.

Brief `fix-worker-detail-run-now-action-and-endpoint` (T0519 P0) is CLAIMED to mac-host at 19:51Z; brief `fix-worker-detail-responsibilities-schedules-tab-end-to-end` exists but is unclaimed. Neither is in review.

- Files: `web/components/skycot/WorkerTabs.tsx:19-39`, `web/components/skycot/worker/DutiesCard.tsx:89-122` (no Run-now CTA per duty)
- Fix: either (a) cut Scene 2 to the workflow surface (the workflow-detail page DOES have Run-now at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263`) and re-narrate the runbook, OR (b) add a per-duty Run-now button + promote duties to a "Schedules" top-level tab.

### Scene 2 — Schedules / Duties + Run-now

#### F4 [P1] — Duty toggle is still a `console.warn` stub (visual flip, no persistence)

Carried from T0519 F8. `WorkerOverviewContent.tsx:87-92` defines `onToggleDuty` as `console.warn(\`[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h\`)`. Clicking the duty switch in DutiesCard flips the UI control but nothing hits the gateway. If the camera films a toggle during Scene 2 it looks like a real change, but the next page-load reverts.

Brief `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak` (T0519 P0) was authored — still TODO/unclaimed.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:87-92`
- Fix: wire to `PATCH /api/v1/workers/{id}/duties/{dutyId}` mutation, OR disable the toggle (greyed) until the endpoint exists. Gateway endpoint `/api/v1/workers/{id}/duties` currently 404s — confirmed via `curl`.

#### F5 [P1] — Brief-ID leak `E4-h` still surfaces in toast on duty edit-click + add-click

Carried from T0519 F9. `WorkerOverviewContent.tsx:74-81` still fires:
- `onAddDuty`: `setAnnouncement("Add responsibility dialog ships in E4-h")` — but the dialog IS now mounted (`AddResponsibilityDialog` is wired in DutiesCard.tsx:125-133). So the toast is a stale lie — it announces a feature ship date while the feature ALREADY ships. Visible during Scene-2 framing.
- `onEditDuty`: `setAnnouncement("Edit schedule ships in E4-h")` — accurate but still leaks the engineering brief-ID "E4-h" into a customer surface.

The toast renders in the bottom-right at `data-testid="overview-toast"` (line 175-182); the camera would read both strings on any duty interaction.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:74-81`, `:175-182`
- Fix: (a) drop the `onAddDuty` announcement entirely now that the dialog mounts; (b) rewrite `onEditDuty` to "Editing a schedule will land soon — for now you can disable + re-add it." Strip "E4-h".

#### F6 [P1] — DutiesCard title still violates operator vocab lock (says "Responsibilities" / "Add responsibility", not "Schedules")

Carried from T0519 F10. `DutiesCard.tsx:89` (`<CardTitle>Responsibilities</CardTitle>`) and `:121` ("Add responsibility" button). Memory `project_vocabulary_duty_vs_schedule` locks UI = "Schedule", code = "Duty". The aria-label on the toggle DOES say "Enable schedule" / "Disable schedule" (line 215) — so the same component contradicts itself between visible label and a11y. AddResponsibilityDialog also keeps the "Responsibility" terminology in its copy.

- Files: `web/components/skycot/worker/DutiesCard.tsx:89,121`, `web/components/skycot/worker/AddResponsibilityDialog.tsx` (dialog title/copy)
- Existing brief: none specifically — `fix-runbook-scene-2-personality-card-vs-tab-vocab` and `fix-playwright-duty-to-schedule-vocab-completion` touch the area but neither is the production-UI relabel.
- Fix: change CardTitle to "Schedules", button to "Add schedule"; rename `AddResponsibilityDialog` copy strings.

### Worker activity tab + audit drill-in

#### F7 [P0] — Overview "Recent activity" still routes to placeholder/wrong-content audit detail

Carried from T0519 F12. The hrefs in `web/lib/sample-worker-overview.ts:70-75,95-99,119-123,143-145` use IDs of shape `ap_<name>_N`. `findSampleAuditDetail` in `web/lib/api/audit-detail.ts:82-83` looks in `SAMPLE_AUDIT_DETAILS` (5 hand-crafted IDs of shape `act_01J0PRIYA_001`) then `SAMPLE_AUDIT_ROWS`. `ap_*` matches neither → falls to `placeholderAuditDetail` rendering "Action detail unavailable …" with worker = "Unknown worker".

Camera visibility: the Overview tab's Recent activity card is the LAST card on /workers/{id}/overview; Scene 2's "see the audit chain" beat naturally clicks one of those rows. Lands on a placeholder.

- Repro: `curl http://127.0.0.1:7400/api/v1/audit/ap_priya_1` → HTTP 404; SPA falls to `placeholderAuditDetail`.
- Files: `web/lib/sample-worker-overview.ts:36` (the `activity()` helper hardcodes the `ap_*` shape into the href), `web/lib/sample-audit-detail.ts:46,85,123,161,197`, `web/lib/api/audit-detail.ts:82-83`.
- Existing brief: `fix-spa-inbound-channels-webhook-urls-and-recent-activity` touches the recent-activity surface but the audit-detail-ID alignment is its own brief (none filed).
- Fix: rewrite `activity()` helper to emit IDs from `SAMPLE_AUDIT_DETAILS` (or expand `SAMPLE_AUDIT_DETAILS` to cover the surfaced IDs). Best long-term: have `argo demo seed` materialise a single canonical row-set the gateway, activity tab, and detail endpoint all read from (per memory `feedback_no_theater_in_demo`'s real-backend-seed path).

#### F8 [P1] — Worker Activity tab gateway endpoint returns SPA self-traffic (`http.request` / `worker.kpi_rolled`), not real agent turns

**Net-new this pass.** `curl http://127.0.0.1:7400/api/v1/workers/{maya-id}/activity` returns 200 with rows tagged `verb=worker.kpi_rolled` (recurring 5-min cron) and `verb=http.request` (SPA self-pings from `/workers`, `/audit`). Zero rows show an actual agent turn. The activity tab renders these literally — if Scene 2's narration is "click into Mariah's activity, see what she's been doing" the camera reads "worker.kpi_rolled · http.request · http.request …".

This is the same root pattern as T0519's `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` brief, but observed on the **per-worker** activity endpoint rather than the global audit feed. The fix should either filter these verbs out at the gateway response layer, or bucket them under a collapsed "system" section in the SPA.

- Repro: `curl -sS http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/activity | jq '.items[].verb' | sort -u` → only `http.request` + `worker.kpi_rolled`.
- File: gateway-side (`crates/argo-gateway/.../routes_workers_activity*.rs`); SPA-side filter at `web/lib/api/worker-activity.ts`.
- Fix: filter `verb in {worker.kpi_rolled, http.request}` from the worker-scoped activity feed at the gateway layer; the global audit feed is separately ticketed.

### Hero, layout, navigation

#### F9 [P2] — `WhatSheDoesCard` still hardcodes "What she does" as the visible CardTitle (gendered)

Carried from T0519 F6. `web/components/skycot/worker/WhatSheDoesCard.tsx:103` — `<CardTitle>What she does</CardTitle>`. The card is the SECOND card on every worker Overview. Jamal is one of the seeded 5 — for him the heading is wrong; for any non-female worker added post-seed the heading lies.

The same file is plastered with gendered pronouns across surfaces — `MemoryFactsCard.tsx:59` "Facts she knows", `MemoryFactsCard.tsx:233` "let her pick it up as she works", `SkillsLearnedCard.tsx:47` "once she's used them", `VoiceProfileSummary.tsx:89` "she'll pick them up", `RecentChangesPanel.tsx:79,146` "her something new" / "She'll go back to how she was", `ChannelStripCard.tsx:199` ternary `?: "she"`, `RecentActivityCard.tsx:42` "she'll show up here once she starts working." Existing brief `fix-worker-pronouns-policy-they-or-explicit-pronoun-field` exists but unmerged.

Camera visibility on /workers/{id}/overview: card title, recent-activity empty state. Other surfaces (memory, skills, etc.) only fire if the camera tabs into them.

- File: `web/components/skycot/worker/WhatSheDoesCard.tsx:103` (the visible CardTitle is the load-bearing one)
- Fix: rename `<CardTitle>` to "Charter" or "What {workerName} does"; bulk-edit pronouns to `they` per existing pronouns-policy brief.

#### F10 [P2] — Worker-detail hero overflow menu (pause/disable/archive) still a `console.warn` stub

Carried from T0519 F16. `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:131-137` — `handleHeroAction` is `console.warn(\`[E4-a] worker hero action '${action}' pending wiring (E3-b/E4-f)\`)`. Existing brief `fix-worker-hero-overflow-menu-pause-disable-archive-stub` authored, not merged. Camera unlikely to open the "…" menu, but if the operator drags-vs-clicks during the take, the menu appears and a click leaks to console (silent to the camera — but the menu opens and closes without effect, which a perceptive viewer would notice).

- File: `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:131-137`
- Fix: hide the overflow menu until E3-b ships; or wire it.

#### F11 [P2] — Hero KPI math on `/team` still hardcoded multiplier (`workers.length * 187`, `* 12`)

Carried from T0519 F15. `web/app/(app)/team/page.tsx:151-152`:

```ts
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```

`/team` is camera-visible during Scene 1 (`/workers` redirects to `/team`). With the gateway currently returning 1 worker, the hero would read "1 teammate, one team. 0 working right now · 187 actions this week · saving you ~12 hours." The "187 actions" lands as a synthetic number the camera doesn't drill into → **per the function-vs-dressing rule, this is dressing-side fixture and NOT a POC blocker.** Still flagging as P2 polish because the wording reads odd at small Ns.

- File: `web/app/(app)/team/page.tsx:151-152`
- Fix (post-MVP): wire to a real KPI rollup endpoint, OR hide the KPI strip when N=1.

### Nits

#### F12 [P3] — `/workers` is still a redirect-only page → URL flicker

Carried from T0519 F18. `web/app/(app)/workers/page.tsx:19-46` — `/workers` does `router.replace("/team")` when workers exist. URL momentarily shows `/workers` then jumps to `/team`. Existing brief `fix-workers-redirect-to-team-url-flicker` authored T0519, not merged.

- Fix: change the sidebar nav to point at `/team` directly, OR collapse the two routes.

#### F13 [P3] — `[tab]/page.tsx` dynamic placeholder still leaks brief-IDs for unknown tab segments

Carried from T0519 F4 in reduced form. The static segments `memory/`, `skills/`, `trust/`, `escalation/`, `settings/` all now have real implementations (verified by directory listing), so the dynamic placeholder fallback in `web/app/(app)/workers/[workerId]/[tab]/page.tsx:38-49` only fires on misspelled URLs (e.g. `/workers/{id}/typo`) — and there it surfaces strings like "Memory — coming in E4-d." The user-reachable surface is small (only typo'd URLs), demoting this to P3.

- File: `web/app/(app)/workers/[workerId]/[tab]/page.tsx:38-49`
- Fix: render a brand-voiced "page not found" body instead of the brief-ID stub for any unrecognised tab segment, OR call `notFound()` on any tab whose body is still placeholder.

---

## Resolved since T0519MEL (1h ago)

**None.** Zero T0519 fixes have merged in the 1-hour interval. The 3 carried-P0 briefs:

- `fix-worker-detail-run-now-action-and-endpoint` — CLAIMED to mac-host 19:51Z, no PR opened yet
- `fix-spa-demo-live-mode-route-real-gateway-on-camera` — CLAIMED to mac-host 19:57Z, no PR opened yet
- `fix-skills-try-chips-zero-results-rewrite` — MERGED #2431 (T20-05) — but this is Scene 4 scope, not Scene 1+2

The 4 new P0s authored at T0519 (`fix-fetchworker-400-bad-request-no-branch-infinite-retry`, `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`, `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak`, plus T0519 carried P0s) remain TODO/CLAIMED.

**Partial mitigation observed**: `AddResponsibilityDialog` IS now mounted in DutiesCard (`DutiesCard.tsx:125-133`), so the "ships in E4-h" toast on Add-click is now factually wrong (the dialog DOES open). Worth a 2-LOC delete of the announcement in `WorkerOverviewContent.tsx:75` independent of the bigger brief-ID-leak fix.

## Orientation score

**38 / 100** — unchanged from T0519MEL. The audit pass mines defects faster than the worker pool clears them; the merge cadence is the gate. Scene 2 remains unrecordable; Scene 1 is blocked twice (gateway seed missing + Peta/Nori personality fallback). New net-new finding this pass — gateway seed only contains Maya — is operational rather than code-fixable; running `argo demo seed --tenant <demo>` against the current gateway boot would clear it without touching the codebase.

### Net-new this pass (worth queueing if not duplicates of T0519)

1. **F1 (P0)** — gateway seed-state needs a pre-record check / automated guard; `scripts/demo-dry-run.sh --demo-mode` should fail loudly if `/api/v1/team` returns fewer than 5 workers.
2. **F8 (P1)** — `/api/v1/workers/{id}/activity` returns `http.request` + `worker.kpi_rolled` SPA-self-traffic; per-worker activity should filter or bucket the same way the global audit feed brief proposes.
3. **F5 micro (P1, embedded in F5)** — the `onAddDuty` toast in `WorkerOverviewContent.tsx:75` lies about a not-yet-shipped dialog that has actually shipped; 2-LOC delete possible independent of the larger brief-ID-leak fix.
