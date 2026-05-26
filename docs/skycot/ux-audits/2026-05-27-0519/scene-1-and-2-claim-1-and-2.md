# UX e2e audit — Scenes 1+2 (Claims 1+2)

**Date**: 2026-05-27 05:19 AEST
**Scope**: `/workers` deck · `/workers/{id}` profile + tabs · Schedules/Duties · Run-now affordance · Audit drill-in from worker
**Mode**: read-only against code + dev server (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, gateway live on :7400)
**Auditor**: e2e-ux-journey-test-agent

---

## TL;DR

Scene 2 of the v1.0 demo runbook ("Mariah's Schedules tab shows 3 duties · click Run now → audit row lands") is **not recordable as scripted**. There is no Schedules tab, no Run-now button on any worker page, and no Schedules surface anywhere in the worker profile. Scene 1 is partially blocked: 2 of the 5 demo workers (Peta, Nori) render a fabricated *default* personality card on `/workers/{id}` because `WorkerPersonalityCard.tsx` only knows about Mariah / Priya / Jamal. Any audit drill-in from the worker activity tab or Overview "Recent activity" card lands on a synthetic placeholder or a row whose verb disagrees with the row the camera clicked. Found **4 P0**, **5 P1**, **5 P2**, **2 P3**.

## Headline blockers

1. **P0 — No Schedules tab on `/workers/{id}`. No Run-now button anywhere on the worker profile.** Scene 2 of the runbook is unrecordable.
2. **P0 — Peta + Nori show fabricated default personality** ("Reliable / Curious / Helpful") instead of their canonical traits. Confirmed in-browser. If Scene 1 narrates each card, two of five are theatrically wrong.
3. **P0 — Audit drill-in from worker Overview "Recent activity" lands on a synthetic placeholder** ("Action detail unavailable …" / unknown worker). The Overview card uses IDs like `ap_priya_1` which match neither `SAMPLE_AUDIT_DETAILS` nor `SAMPLE_AUDIT_ROWS`.
4. **P0 — Duty edit/toggle stubs leak brief-IDs into a camera-visible toast** ("Edit schedule ships in E4-h"). The camera reads this if the operator clicks any duty body during Scene 2.

---

## All findings

### Scene 1 — `/workers` deck + `/workers/{id}` hero & personality

#### F1 [P0] — `WorkerPersonalityCard` missing entries for Peta + Nori (2 of 5 demo workers)

Verified in-browser. Loading `/workers/018f67d0-0000-7000-8000-000000000007/overview` (Peta) and `/workers/018f67d0-0000-7000-8000-000000000004/overview` (Nori) renders:

- Trait chips: "Reliable / Curious / Helpful" (the `DEFAULT_PERSONALITY` fallback)
- "Great at" list: 3 generic items ("Working through the responsibilities you set", …)
- "Escalate when" list: 3 generic items ("Confidence in the next action is below 80%", …)

The canonical persona-spine has Peta's traits as "Cadence-true / Follow-through / Calm escalator / Context keeper" and Nori's as "On-voice / Considered / Schedule-respecting / Reach-tracking" (`web/lib/sample-personas.ts:136,118`). But `web/components/skycot/worker/WorkerPersonalityCard.tsx:35` only declares PERSONALITIES for Mariah, Priya, Jamal. Peta + Nori fall through to `DEFAULT_PERSONALITY` at line 98.

Camera visibility: this card is the FIRST card on every worker overview page (`web/app/(app)/workers/[workerId]/overview/page.tsx:24`), labelled "HOW {NAME} WORKS". If Scene 1 click-throughs include Peta or Nori (the runbook says Mariah is the demo target, but the workers deck shows all 5), the viewer reads canned generic content under "HOW PETA WORKS" — that's theater the camera sees.

- Repro: dev server up with demo mode → `open http://127.0.0.1:3000/workers/018f67d0-0000-7000-8000-000000000007/overview` → top card shows "Reliable / Curious / Helpful" chips.
- File: `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96`
- Fix: add PETA + NORI entries to the PERSONALITIES map, sourced from `sample-personas.ts` traits + new escalate/signature copy. Estimated 30 LOC.

#### F2 [P1] — `WorkerPersonalityCard` data hardcoded by worker NAME, not ID — breaks for any worker named anything but the 3 baked-in names

`PERSONALITIES[worker.name]` (line 117) is a string-keyed lookup against the worker's display name. Renaming Mariah → "Maria" in the demo seed silently breaks her personality; any non-demo worker (Bob, Sam, Sarah) gets the default. The persona spine in `sample-personas.ts` uses stable IDs precisely to avoid this fragility — the personality map should too.

- File: `web/components/skycot/worker/WorkerPersonalityCard.tsx:117`
- Fix: key PERSONALITIES by persona ID, not name. Same as `sample-worker-overview.ts` / `sample-worker-activity.ts` already do.

#### F3 [P1] — Tab strip lacks "Schedules" — there is no Schedules tab on `/workers/{id}` at all

`WorkerTabs` declares 7 tabs: overview / activity / memory / skills / trust / escalation / settings. The runbook Scene-2 narration "Mariah's Schedules tab shows 3 duties" cannot point at anything. Duties currently render as a sub-card on the Overview tab (`WorkerOverviewContent.tsx:151`), titled "Responsibilities". The closest the user can get is the Overview-tab's Responsibilities card — which is camera-far-from "Mariah's Schedules tab".

- File: `web/components/skycot/WorkerTabs.tsx:19-39`
- Fix (smallest): rename the "Responsibilities" CardTitle inside DutiesCard to "Schedules", update Scene-2 runbook narration to say "scroll down to Mariah's Schedules card." Bigger fix: promote duties to a dedicated tab.

#### F4 [P2] — Three worker-detail tabs are explicit "coming in <brief>" placeholders the camera could land on

`WorkerTabs` exposes Memory, Skills, Trust, Escalation, Settings as nav items. The `[tab]/page.tsx` dynamic page returns a dashed-border panel reading "Memory — coming in E4-d." / "Skills — coming in E4-e." / "Settings — coming in E4-f." for any tab without a static sibling. Camera clicking Memory or Settings during exploratory framing reads brief-IDs.

Note: `escalation/page.tsx` (444 LOC) DOES exist as a real implementation, so escalation is fine. Trust is implemented (via `mission-trust-grade-ui`).

- File: `web/app/(app)/workers/[workerId]/[tab]/page.tsx:38-49`
- Fix: hide nav tabs whose body is still a placeholder, OR replace placeholder bodies with a brand-voiced "coming soon" message that doesn't leak brief-IDs.

#### F5 [P2] — Activity tab uses gendered pronouns by default ("she's working", "she hasn't taken any actions yet")

`/workers/{id}/activity` empty state hardcodes feminine pronouns: "Once she's working, you'll see her activity here" (`web/app/(app)/workers/[workerId]/activity/page.tsx:203`). Demo workers Jamal + Patrick (future) aren't female. The personality card itself is gender-neutral, but the activity tab is not.

- File: `web/app/(app)/workers/[workerId]/activity/page.tsx:203`
- Fix: rewrite as "Once {workerName} starts working, you'll see actions here" — drop pronouns.

#### F6 [P3] — `WhatSheDoesCard` component name + visible card header "What she does" hardcodes feminine

The Overview tab's first card under the personality is "What she does" — a charter description card. The header is literally the string "What she does" (`web/components/skycot/worker/WorkerOverviewContent.tsx:8` import + visible label). Same gendered assumption.

- File: `web/components/skycot/worker/WhatSheDoesCard.tsx` (header copy)
- Fix: rename to "Charter" or "What {name} does".

### Scene 2 — Schedules / Duties + Run-now

#### F7 [P0] — No "Run now" button anywhere on the worker profile

`/workers/{id}/*` has no Run-now / Trigger / Execute control. The only "Run now" button in the entire SPA is the workflow-detail page (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263`). The runbook Scene-2 line "Click 'Run now' → audit row lands in /audit within 5s" assumes a worker-scoped trigger that doesn't exist.

- Repro: `/workers/018f67d0-0000-7000-8000-000000000001/overview` → grep DOM for "Run now" → no match.
- Fix options:
  - (a) cut Scene 2 to the workflow surface ("Click 'Run now' on the workflow page → audit row lands");
  - (b) add a `Run now` button per duty row on DutiesCard wired to a gateway endpoint that synchronously appends to the audit feed.

#### F8 [P0] — Duty toggle is a fake — flips the visual switch but no mutation is persisted

`WorkerOverviewContent.tsx:87` defines `onToggleDuty` as a `console.warn` that says "mutation lands with E4-h". Clicking the duty switch in DutiesCard flips the UI control but nothing hits the gateway. If the camera films a toggle during Scene 2 it looks like a real change, but the next page-load reverts.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:87-92`
- Fix: wire to a `PATCH /api/v1/workers/{id}/duties/{dutyId}` mutation. Disable the toggle (greyed) until the endpoint exists.

#### F9 [P0] — Clicking a duty body emits a brief-ID leak in the camera-visible toast: "Edit schedule ships in E4-h"

`onEditDuty` (same file, line 78) sets a transient toast with the literal string "Edit schedule ships in E4-h" — visible in the bottom-right corner as a card with `data-testid="overview-toast"`. The brief-ID "E4-h" is not part of the brand voice; it's an engineering token that should never reach a customer surface.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:78-81`, `:175-182`
- Fix: replace with "Editing a schedule will land soon — for now you can disable + re-add it." Strip the brief-ID.

#### F10 [P1] — DutiesCard title violates operator vocab lock: shows "Responsibilities" / "Add responsibility", not "Schedules"

`sample-personas.ts` calls them duties; `WorkerDuty` type uses `duty.schedule` for the cron chip; the aria-label on the toggle says "Enable schedule" / "Disable schedule"; but the visible card title is **"Responsibilities"** and the add button is **"Add responsibility"** (`web/components/skycot/worker/DutiesCard.tsx:89,121`). Operator-locked vocab (memory `project_vocabulary_duty_vs_schedule`, 2026-05-23): UI = "Schedule", code = "Duty". This card is the most prominent surface of duty data in the app; the visible nouns disagree with the lock.

- File: `web/components/skycot/worker/DutiesCard.tsx:89, 121`, plus `AddResponsibilityDialog.tsx` (dialog title/copy)
- Fix: relabel CardTitle to "Schedules", button to "Add schedule". Component filename can stay (code-name).

#### F11 [P1] — Commit-toast says "She's next run kicks off …" — gendered + slightly grammatically off

`DutiesCard.tsx:75` builds the success toast with `const who = workerName ?? "She";`. When `workerName` is missing, the fallback is literally the pronoun "She". For workers like Jamal that's wrong; for an unnamed worker the pronoun bias is also wrong.

- File: `web/components/skycot/worker/DutiesCard.tsx:75-79`
- Fix: fallback to "The teammate" — pronoun-free.

### Worker activity tab + audit drill-in

#### F12 [P0] — Activity tab + Overview "Recent activity" drill-in lands on placeholder/wrong-content detail for most rows

Three independent ID schemes for audit/activity:

| Source | ID shape | example |
|---|---|---|
| `SAMPLE_AUDIT_ROWS` (the /audit list fixtures) | `act_<persona-id>_NNN` | `act_018f67d0-0000-7000-8000-000000000002_000` |
| `SAMPLE_WORKER_ACTIVITY` (the worker Activity tab) | `act_<workerId>_NNN` | same shape as above |
| `SAMPLE_AUDIT_DETAILS` (the canonical detail seeds) | `act_01J0PRIYA_001` | `act_01J0PRIYA_001` |
| `WorkerOverview.recentActivity` (Overview "Recent activity" card) | `ap_<name>_N` | `ap_priya_1` |

`findSampleAuditDetail` first looks in `SAMPLE_AUDIT_DETAILS` (5 hand-crafted IDs), then in `SAMPLE_AUDIT_ROWS`. Anything else → `placeholderAuditDetail` which renders "Action detail unavailable — drill into another row for full provenance" with worker = "Unknown worker". The Overview "Recent activity" card hrefs ALL go to `/audit/action/ap_<name>_N` — all fall to placeholder. The Activity tab IDs sometimes collide with SAMPLE_AUDIT_ROWS index N — but only by index, not by verb, so the detail card shows a different action than the row the user clicked.

Camera-visible failures:
- Scene 2 if narrated as "click a recent action to see the audit trail" from the Overview card → placeholder.
- Cross-surface mismatch: PRIYA's Activity row #0 verb "screened candidate #4821 against role's must-haves" drills into a detail card with verb "screened Diego Alvarez against JD-2024-08" (because IDs collide on index but seeds differ across files).

- Repro: `curl http://127.0.0.1:7400/api/v1/audit/ap_priya_1` → HTTP 404; falls to `placeholderAuditDetail` (`web/lib/api/audit-detail.ts:120`).
- Files: `web/lib/sample-worker-overview.ts:36`, `web/lib/sample-worker-activity.ts:188`, `web/lib/sample-audit.ts:134`, `web/lib/sample-audit-detail.ts:46,85,123,161,197`, `web/lib/api/audit-detail.ts:82-83`.
- Fix: rebuild Overview activity + worker-activity IDs to mirror `SAMPLE_AUDIT_ROWS` IDs; OR expand `SAMPLE_AUDIT_DETAILS` to cover the IDs surfaced in both sample feeds; OR have the seed CLI (per memory `feedback_no_theater_in_demo`'s real-backend-seed path) ship a single canonical row set the gateway, activity tab, and detail endpoint all read from.

#### F13 [P1] — Verb/timestamp disagree across surfaces for same action ID (cross-fixture drift)

Even when an ID collides — e.g. `act_018f67d0-0000-7000-8000-000000000002_000` exists in both `SAMPLE_AUDIT_ROWS` (verb "screened Diego Alvarez") and `SAMPLE_WORKER_ACTIVITY[priya][0]` (verb "screened candidate #4821") — the verbs differ and synthesizeAuditDetail returns SAMPLE_AUDIT_ROWS's verb. The Activity feed's row text doesn't match the detail card's hero text after click.

This is the same root cause as F12 but worth flagging separately because it doesn't render a "placeholder" — it renders rich-looking content that's just wrong. From the camera that's worse than the placeholder.

- Files: same as F12 — three independent seed files describing the same actions with different verbs.
- Fix: single canonical seed source for action verb + timestamp; consumers project subsets.

#### F14 [P2] — Activity tab loads `/api/v1/workers/{id}/activity` which the gateway 404s — relies on dev-mode fallback the demo MP4 must trigger

The Activity tab fetches `/api/v1/workers/<id>/activity?range=…` (`web/lib/api/worker-activity.ts:246`). Gateway returns 404 (confirmed via `curl http://127.0.0.1:7400/api/v1/workers/018f67d0-.../activity` returns "not found"). The page falls back to `SAMPLE_WORKER_ACTIVITY` only in dev / demo-mode. If the demo MP4 is recorded against a non-demo-mode build, this tab is empty.

The runbook should explicitly confirm the demo recording happens with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`.

- File: `web/lib/api/worker-activity.ts:236-280`
- Fix: document the env var requirement in the runbook; OR implement the gateway endpoint (a tracked brief: `mission-gateway-worker-activity-endpoint`).

#### F15 [P2] — Hero KPI math on `/team` is hardcoded multiplier (`workers.length * 187` for weeklyActions, `* 12` for hoursSaved)

`web/app/(app)/team/page.tsx:151-152`:

```ts
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```

`/team` is camera-visible during Scene 1 (`/workers` redirects to `/team` per `web/app/(app)/workers/page.tsx:26`). The hero shows "5 teammates, one team. 4 working right now · 935 actions this week · saving you ~60 hours." All three numbers are synthetic — but they're DRESSING the camera doesn't drill into, so per the function-vs-dressing rule this is acceptable. Flagging as P2 polish (not P0) — but if Scene 1's voiceover specifically reads "935 actions" the cameraman should know it's fixture-driven.

- File: `web/app/(app)/team/page.tsx:151-152`
- Fix (post-MVP): wire to a real KPI rollup endpoint.

#### F16 [P2] — `WorkerHero` pause/disable/archive overflow-menu is a console.warn stub

`WorkerLayoutShell.tsx:131-137` defines `handleHeroAction` as `console.warn('worker hero action ${action} pending wiring (E3-b/E4-f)')`. The hero shows a "…" menu; clicking pause / disable / archive prints to console — no toast, no error. Camera unlikely to click but a curious viewer playing back the demo might.

- File: `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:131-137`
- Fix: hide the overflow menu until E3-b ships; or wire it.

#### F17 [P3] — `team-data-source-chip` shows in dev (the diagnostic chip next to the "Team" header)

`/team` displays a small uppercase chip next to the H1 with values like "MOCK-OFFLINE" / "GATEWAY" / "LOADING". It's gated behind `NODE_ENV !== "production"` (`team/page.tsx:319`) so the demo MP4 should NOT see it if recorded against `next build && next start`. If recorded against `next dev`, the chip is camera-visible. Worth re-confirming the demo build mode.

- File: `web/app/(app)/team/page.tsx:316-329`
- Fix: confirm the demo MP4 records against a production build with `NODE_ENV=production NEXT_PUBLIC_ENV=production`, OR widen the gate to also check `NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1"`.

#### F18 [P3] — `/workers` is now a redirect-only page; "Workers" nav label points to a route that bounces

`web/app/(app)/workers/page.tsx:19-46` — `/workers` does `router.replace("/team")` immediately when workers exist, and shows the "Hire your first" empty state otherwise. There's no "/workers list view" — it's all under `/team`. The URL semantics are slightly off ("workers" plural but the canonical name is "Team") and the bouncing redirect adds a frame to navigation timing.

If the demo MP4 narrates "click Workers in the nav" the URL bar momentarily shows `/workers` then jumps to `/team`. Minor camera artifact.

- File: `web/app/(app)/workers/page.tsx:19-46`
- Fix: change nav to point at `/team` directly, OR collapse the two routes.

---

## Resolved since last audit

No prior audit at this scope. The previous-day pre-record-ux-audit doc was operator-rejected (memory `feedback_avoid_pre_record_ux_audit_2026_05_25`).

## Orientation score

**38 / 100** — Scene 2 is unrecordable as scripted; Scene 1 needs personality content for 2 of 5 workers + a vocabulary relabel; the audit drill-in is theatre the camera sees on every Overview "Recent activity" click. Solidly-built bits (the hero, the personality-card design for the 3 wired workers, the Activity tab filter rail, the freeze button) are real and well-shaped — the gaps are concentrated in a small surface area. Three days of focused work would lift this above 80.
