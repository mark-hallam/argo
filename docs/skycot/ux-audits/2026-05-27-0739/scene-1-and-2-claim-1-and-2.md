# UX e2e audit — Scenes 1+2 (Claims 1+2)

**Date**: 2026-05-27 17:39 AEST
**Scope**: `/workers` deck · `/workers/{id}` profile + tabs · Schedules/Duties · Run-now affordance · Audit drill-in from worker
**Mode**: read-only against code + live gateway on :7400 + live SPA on :3000 (running, returns 200 on `/workers`, `/team`, `/workers/<id>`). Findings derived from code reading + curl probes of the gateway team endpoint + git-log diff against the T0623 audit one tick prior.
**Auditor**: e2e-ux-journey-test-agent
**Prior run**: 2026-05-27 06:23 AEST (same scope)

---

## TL;DR

Scene 1+2 are **still not recordable**. Zero of the prior pass's worker-detail P0/P1 briefs landed in the last hour — the merge-train this tick was Scene-5 / approvals / signup / docs / demo-mode collateral, none of which touch the worker profile. The gateway is still seeded with only Maya (1 worker, persona `marketing-intern`); the canonical 5-roster (Mariah/Peta/Priya/Jamal/Nori) is absent, so `/workers` redirects past its own page into `/team`, where one tile renders for a worker whose name the voiceover never says. **Net-new this pass: the worker-hero overflow menu's Pause/Disable-schedules/Archive/Export actions are all wired only to `console.warn` — if the camera films an overflow click, the menu closes silently and nothing changes.** Found **3 P0**, **5 P1** (1 net-new), **2 P2**, **2 P3**.

## Headline blockers

1. **P0 — Gateway only has 1 worker ("Maya"), not the 5-name canonical roster.** Scene 1 voiceover names Mariah/Peta/Priya/Jamal/Nori; the visible deck cannot match. (carried T0623 F1; still unaddressed on this gateway boot.)
2. **P0 — Still no "Schedules" tab + still no "Run now" button on the worker profile.** Scene 2 unrecordable as scripted. Workflow-detail page has Run-now (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263`); if Scene 2 cuts there, runbook needs a re-narrate. (carried T0623 F3.)
3. **P0 — Worker Overview "Recent activity" card hrefs still route to placeholder audit-detail.** `ap_<name>_N` IDs match neither `SAMPLE_AUDIT_DETAILS` (which uses `act_01J0NAME_NNN` form) nor `SAMPLE_AUDIT_ROWS`, so every click in demo-mode falls through to `placeholderAuditDetail()` and renders "Action detail unavailable" — theatre the camera sees. (carried T0623; confirmed against `web/lib/sample-audit-detail.ts:270-310` + `web/lib/sample-worker-overview.ts:95-99`.)

---

## All findings

### Scene 1 — `/workers` deck + `/workers/{id}` hero & personality

#### F1 [P0] — Gateway still has 1 worker ("Maya") instead of the 5-name canonical roster

`curl http://127.0.0.1:7400/api/v1/workers` returns one row: `Maya`, vertical `marketing-intern`, status `idle`, headline "No activity yet." Scene 1's voiceover explicitly names Mariah / Peta / Priya / Jamal / Nori. `web/lib/api/team.ts:79-93` reads the gateway envelope verbatim — a successful 200 with `workers.length=1` does NOT trigger the mock-bypass fallback (that only fires on 404 / network error per the strict-production guard at line 95-105). So the SPA renders one card, for the wrong persona, with the wrong tagline.

Compounding effect: `/workers` (`web/app/(app)/workers/page.tsx:25-27`) calls `router.replace("/team")` whenever `workers.length > 0`. With Maya present, the user never sees the "Workers" route at all — Scene 1's URL bar reads `/team`, not `/workers`. That's a runbook mismatch.

This is the same finding as T0623 F1; nothing in the last hour's merge train touched gateway seed data.

- Repro: `curl -fsS http://127.0.0.1:7400/api/v1/workers | jq '.workers[].name'` → expect 5 names, observed `Maya`.
- File: operational; runbook `docs/skycot/demos/v1-mission-mvp-runbook.md` lines 64-77 documents `argo demo seed --tenant $ARGO_DEMO_TENANT_ID`.
- Fix: re-run the seed CLI against the current gateway boot; add a roster-size check to `scripts/demo-dry-run.sh --demo-mode` that fails loudly if `workers.length != 5`.

#### F2 [P0] — `WorkerPersonalityCard` STILL missing entries for Peta + Nori

Carried from T0623 F2. `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` declares PERSONALITIES for Mariah / Priya / Jamal only. Peta + Nori (and Maya, the only worker the gateway currently exposes) fall through to `DEFAULT_PERSONALITY` at line 98 — "Reliable / Curious / Helpful" with three generic items per column. This card is the FIRST thing rendered on every worker overview page (`web/app/(app)/workers/[workerId]/overview/page.tsx:24`). Labelled `HOW {NAME} WORKS` in `wpc-section-label` (line 214). With current seed data, opening Maya's profile reads "HOW MAYA WORKS · Reliable Curious Helpful" — generic boilerplate where the camera expects voice.

Brief `fix-peta-personality-trait-level-lookup-coverage.md` + `fix-worker-personality-coverage-extend-to-nori.md` exist; neither has merged.

- File: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-117`
- Fix: add PETA + NORI entries to PERSONALITIES; key the lookup by stable persona ID rather than `worker.name` (current name-key is a string-match foot-gun if any persona rename happens).

#### F3 [P1, net-new] — Worker-hero overflow menu actions wired only to `console.warn` (Pause, Disable schedules, Archive, Export — all silent stubs)

`WorkerLayoutShell.tsx:131-137` defines `handleHeroAction` as a `console.warn("[E4-a] worker hero action '<x>' pending wiring (E3-b/E4-f)")` no-op. `WorkerOverflowMenu.tsx:53-94` mounts five items — Pause/Resume, Disable schedules, Archive, Export data, View as Operator. Clicking any of them closes the menu and emits one console warning. No toast, no banner, no visible status change. The "View as Operator" item at least labels itself "soon" inline (line 92); the other four pretend to be real.

Camera visibility: the overflow `…` button sits in the WorkerHero top-right (testid `worker-overflow`). It's a candidate for a passing "look how rich this profile is" beat in Scene 1. If the camera clicks Pause and nothing happens, that's theatre going wrong — the customer can't tell whether the button is broken or whether their action succeeded silently.

- File: `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:131-137`; menu at `web/components/skycot/WorkerOverflowMenu.tsx:53-94`
- Existing brief: `fix-worker-hero-overflow-menu-pause-disable-archive-stub.md` (unclaimed).
- Fix: either (a) wire Pause to the existing `PATCH /api/v1/workers/{id}/state` endpoint with an optimistic UI flip, OR (b) add a `disabled` flag + `… soon` suffix to the four unwired items the way "View as Operator" already does, so the customer can't click into a no-op.

#### F4 [P2] — Six of seven non-Overview worker tabs render "coming in E4-X" placeholder text

`web/app/(app)/workers/[workerId]/[tab]/page.tsx:17-49` is the dynamic per-tab fallback. The `TAB_BRIEFS` map ties each tab to an engineering brief-ID (`E4-b`, `E4-c`, `E4-d`, `E4-e`, `mission-trust-grade-ui`, `web-worker-detail-escalation-and-trust-chart`, `E4-f`) and the panel renders `"{Label} — coming in {brief}."` inside a dashed-border placeholder. The only tab with real content is Overview (which has its own route, not the dynamic one).

Camera visibility: the runbook for Scene 1 says "tour Mariah's profile, click through the tabs." Five of the seven labels are wired to live tabs that show "Activity — coming in E4-c." style copy. The brief-ID leak ("E4-c", "E4-f") is the bigger issue than the absence of content — it shows engineering vocabulary in a customer surface.

- File: `web/app/(app)/workers/[workerId]/[tab]/page.tsx:17-49`
- Fix: in the placeholder copy, drop the brief-ID — "Activity history is rolling out in our next update." reads like product copy; "coming in E4-c." reads like a sprint board. If Scene 1 will only land on Overview, scope the runbook accordingly (and leave the placeholders to ship in their dedicated briefs).

#### F5 [P3] — Worker-detail page has a `responsibilities/new` route segment but no Schedules tab to land on

`web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx` exists (visible in the directory listing); the `WORKER_TAB_IDS` array at `WorkerTabs.tsx:19-27` has no "responsibilities" or "schedules" entry. The route is reachable via direct URL but has no tab link or breadcrumb in the in-product UI. Either the route is a leftover from an earlier brief or a tab needs to be added — currently it dangles.

- Files: `web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx`, `web/components/skycot/WorkerTabs.tsx:19-27`
- Fix: delete the route if dead OR add a "Schedules" tab (and bring DutiesCard / AddResponsibilityDialog inside it per `fix-worker-detail-responsibilities-schedules-tab-end-to-end`).

### Scene 2 — Schedules / Duties + Run-now

#### F6 [P0] — Still no "Schedules" tab + still no "Run now" button on the worker profile

Carried from T0623 F3. `WorkerTabs` (`web/components/skycot/WorkerTabs.tsx:19-39`) declares 7 tabs — overview / activity / memory / skills / trust / escalation / settings — none of which is Schedules. Schedules currently live as a sub-card on Overview via `DutiesCard.tsx`. There is no Run-now / Trigger / Execute control anywhere under `/workers/<id>/*`. The workflow-detail surface DOES have Run-now (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263`) — `<Play size={13} /> {running ? "Running…" : "Run now"}` — so the runbook can pivot there.

Brief `fix-worker-detail-run-now-action-and-endpoint.md` exists; `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md` exists. Neither merged in the last 6h of activity.

- Files: `web/components/skycot/WorkerTabs.tsx:19-39`, `web/components/skycot/worker/DutiesCard.tsx:89` (CardTitle is "Responsibilities", not "Schedules")
- Fix: pivot Scene 2 to `/workflows/<vertical>/<name>` where Run-now exists today AND re-narrate the runbook OR add per-duty Run-now to DutiesCard + promote it to a top-level "Schedules" tab.

#### F7 [P1] — DutiesCard still violates operator vocab lock (says "Responsibilities", not "Schedules")

Carried from T0623 F6. The 2026-05-23 rename commit (`0ab4f4d0c`) touched only the aria-label `"Disable duty" → "Disable schedule"` at DutyToggle line 215. The CardTitle at line 89 is still `Responsibilities`; the footer button at line 121 still says "Add responsibility"; the AddResponsibilityDialog props (`workerName`, etc.) are passed at line 130-131 — the dialog title itself almost certainly says "Add a new responsibility" too. Memory `project_vocabulary_duty_vs_schedule` locks the UI word to "Schedule" (code/SQL/audit can keep "duty"). The same component contradicts itself in the same render: visible heading says one thing, screen-reader announces another.

- File: `web/components/skycot/worker/DutiesCard.tsx:89, 120-122` (and `AddResponsibilityDialog.tsx`)
- Fix: rename CardTitle → "Schedules"; rename "Add responsibility" → "Add a schedule"; audit the dialog copy in the same pass; do NOT touch test-ids (`card-duties`, `duty-row-*`) — keep code-vocab "duty" per operator lock.

#### F8 [P1] — Duty toggle is still a `console.warn` stub (visual flip, no persistence)

Carried from T0623 F4. `WorkerOverviewContent.tsx:87-92` defines `onToggleDuty` as `console.warn(\`[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h\`)`. Clicking the toggle in DutiesCard flips the local UI state (DutyRow holds its own `useState(duty.enabled)` at line 149) but nothing reaches the gateway. The gateway endpoint `PATCH /api/v1/workers/{id}/duties/{dutyId}` returns 404 today — confirmed via curl probe of `/api/v1/workers/maya/duties` returning 404.

Brief `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.md` exists; not claimed.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:87-92`
- Fix: either (a) wire to `PATCH /api/v1/workers/{id}/duties/{dutyId}` (gateway brief required first), OR (b) disable the toggle until the endpoint exists so the customer can't flip a switch that doesn't stick.

#### F9 [P1] — Brief-ID leak `E4-h` still surfaces in toast on duty edit-click + (stale) add-click

Carried from T0623 F5. `WorkerOverviewContent.tsx:74-81`:
- `onAddDuty`: `setAnnouncement("Add responsibility dialog ships in E4-h")` — but the dialog IS already mounted via DutiesCard at line 125-133. So the toast announces "ships in E4-h" while the feature already ships. The toast renders at testid `overview-toast` (line 175-182) in the bottom-right.
- `onEditDuty`: `setAnnouncement("Edit schedule ships in E4-h")` — accurate but still leaks `E4-h` (an engineering brief-ID) into a customer surface.

The toast is visible to the camera on any Scene-2 duty interaction.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:74-81, 175-182`
- Fix: drop the `onAddDuty` announcement (dialog is real now); rewrite `onEditDuty` to "Editing a schedule is rolling out soon — disable + re-add for now." Strip "E4-h".

### Audit drill-in from worker

#### F10 [P0] — Recent activity hrefs land on the placeholder audit detail ("Action detail unavailable")

Carried from T0623 F12. `web/lib/sample-worker-overview.ts:95-99` defines Mariah's activity rows with IDs `ap_mariah_1`, `ap_mariah_2`, etc. Each row's `href` is `/audit/action/${id}` per the `activity()` helper at line 36. The audit-detail lookup in `web/lib/sample-audit-detail.ts:270-276` checks `SAMPLE_AUDIT_DETAILS` (which uses `act_01J0MARIAH_001` form, lines 17-23) then `SAMPLE_AUDIT_ROWS` (no `ap_mariah_*` entries, confirmed via grep) and returns `undefined`. The API layer at `audit-detail.ts:82-84` then falls through to `placeholderAuditDetail()` → action body "Action detail unavailable — drill into another row for full provenance." rendered.

The camera will drill in on Scene 2 to verify "the action you just ran lands as a real audit row." It will read a placeholder fabrication instead. Function-side fabrication = P0 per the function-vs-dressing rule.

- Files: `web/lib/sample-worker-overview.ts:36` (href construction); `web/lib/sample-audit-detail.ts:270-310` (lookup + placeholder); `web/lib/api/audit-detail.ts:82-84` (demo-mode fallthrough)
- Fix: either rename the `ap_NAME_N` IDs in `sample-worker-overview.ts` to match `act_01J0NAME_NNN` IDs already in `SAMPLE_AUDIT_DETAILS`, OR add `ap_NAME_*` entries to `SAMPLE_AUDIT_ROWS` so `synthesizeAuditDetail()` builds them.

#### F11 [P2] — "See all in Activity" footer link routes to a placeholder tab

`web/components/skycot/worker/RecentActivityCard.tsx:54` links to `/workers/${workerId}/activity` which is the dynamic-tab placeholder ("Activity — coming in E4-c"). So clicking "See all in Activity →" leads from a real-ish list of activity rows directly into a "coming soon" stub. The footer chevron promises a destination that isn't there.

- File: `web/components/skycot/worker/RecentActivityCard.tsx:53-60`; placeholder at `web/app/(app)/workers/[workerId]/[tab]/page.tsx`
- Fix: until the Activity tab ships, hide the See-all link OR route it to `/audit?worker=<id>` (the audit feed with a worker filter — if that filter doesn't exist yet, that's the brief).

#### F12 [P3] — RetrainDialog mounted but `onAdjust` is a comment-only no-op (no analytics fire)

`WorkerOverviewContent.tsx:71-73` defines `onAdjust = useCallback(() => { /* no-op — dialog mount lives in WhatSheDoesCard */ }, [])`. Comment says "kept as an analytics seam for callers who want to know an adjust was launched" — but it doesn't fire any analytics. Either wire it or delete it; the comment misrepresents what the callback does.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:71-73`
- Fix: drop the no-op + comment if no analytics event is planned, or wire it to the analytics seam.

---

## Resolved since last audit (T0623, ~11h ago)

None — zero of the Scene 1/2 fix briefs queued in prior audits have merged. The last 6h of git history (genesis nav, Scene 5 connector palette, signup/demo polish, approvals inline actions, hire-flow Playwright cluster, audit-replay routing) touched everything EXCEPT the worker-profile surface this audit covers. The T0623 directive bullet remains: pre-record requires either (a) demo seed CLI run + Schedules-tab + Run-now + personality coverage + audit hrefs fixed, OR (b) Scene 2 re-scoped to `/workflows/<v>/<n>` Run-now and re-narrated.

---

## Orientation score

**51 / 100** (down 1 from T0623's 52).

The Scene 1/2 path has been at a soft impasse for ~11h. The substrate is there: WorkerHero renders, WorkerTabs renders, DutiesCard renders, AddResponsibilityDialog mounts. Three to five hours of focused work would close the gap (rename copy, swap activity-row IDs, wire Pause to the existing state endpoint, gate the unwired overflow items behind `disabled` + "soon", run the seed CLI). The blocker is allocation, not difficulty.

For the camera's view today: Scene 1 cannot be filmed as the runbook reads — there is only one worker, and they are not on the script. Scene 2's Schedules-tab + Run-now beats land elsewhere in the product (`/workflows/<v>/<n>`) so the runbook can pivot, but the worker-detail "click into a schedule and run it" framing is not buildable from the current code without either landing the in-flight briefs or re-scoping the scene.
