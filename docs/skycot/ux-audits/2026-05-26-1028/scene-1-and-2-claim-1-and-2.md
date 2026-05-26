# Scenes 1 + 2 — Claims 1 + 2 (Teammate-not-tool, Delegation-is-the-unit)

**Date:** 2026-05-26 10:28 AEST
**Scope:** `/workers` deck + `/workers/{id}` tabs (Mariah + Peta) + Schedules / Duties "Run now" + audit-row landing.
**Tester:** e2e-ux-journey-test-agent (browser MCP + code-read + curl).
**Lens:** function-vs-dressing rule per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — POC/investor demo, not public release.
**Prior baseline:** `docs/skycot/ux-audits/2026-05-26-0820/scene-1-and-2-claim-1-and-2.md` (15 findings; 52/100 orientation).

---

## TL;DR

**5 P0, 4 P1, 3 P2, 2 P3 — 14 findings total.** Net state vs the 08:20 audit: **unchanged on the headline blockers, plus three new browser-confirmed defects.** Scenes 1 + 2 are still not recordable as scripted. The live gateway still returns one worker "Maya" instead of the five-card deck (Mariah / Peta / Priya / Jamal / Nori). The Worker detail page has no Schedules / Duties tab on the tab strip (Overview / Activity / Memory / Skills / Trust / Escalation / Settings), and although a `DutiesCard` does render *inside* Overview, it has no "Run now" affordance and Maya's `duties` array on the gateway is empty. Claim 2 ("delegation is the unit") still has zero visual surface in the audit drill-in — the 8-step `DecisionChainCard` renders all eight rows with empty `body` / `null` details against the only audit rows the gateway has (which are `audit.detail.viewed` meta-events, worker.name "unknown"). New this run: the gateway `/api/v1/workers/<id>` endpoint **fails with `net::ERR_FAILED` from cross-origin browsers** (CORS gap or the preview-port mismatch), giving "Worker not found" even though curl returns 200. The Next.js dev-tools floating "1 Issue" red badge is visible on every page during recording. The dismissible "Demo mode — showing sample data. Sign in to see your own workspace" banner is on by default and undercuts the "real teammates" register if not dismissed before record.

**Headline blocker for the v1.0 demo MP4:** the gateway seed needs to land the five canonical workers with non-empty `duties` and at least one real worker action per worker (so the Overview "Recent activity" + the global audit feed have rows that aren't `http.request` self-references). Either ship a Schedules / Duties tab with a Run-now button, or rewrite Scene 2 to a flow that exists (Workflow detail "Run now" works; worker duty "Run now" does not).

---

## Headline blockers (top 5 P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Gateway returns **1 worker "Maya"**, not the 5 expected. SAMPLE_PERSONA_IDS still ships Sam+Rio instead of Peta. | Scene 1 can't pan across the deck; Scene 2 has nobody named Mariah to click into. |
| F2 | P0 | **No Schedules / Duties tab** on worker tab strip. DutiesCard renders inside Overview, but with no "Run now" button. | Scene 2 step 1 (click "Schedules" tab) impossible; step 2 (Run now on duty) impossible. |
| F3 | P0 | **`duties: []`** on the gateway for Maya, plus all `permissions.can_edit_duties` are `false`. Even if Run-now shipped, no duty exists to fire. | Camera lands on a "No responsibilities yet" empty state. |
| F4 | P0 | **No delegated-by / delegated-to / handoff-chain rendering** anywhere in the audit + activity drill-in. `DecisionChainCard` has 8 stages of empty body / null details. | Claim 2 talk track "delegation is the unit" is not provable by drill-in. |
| F5 | P0 | Audit feed contains only `audit.detail.viewed` rows where `worker.name = "unknown"`. Real worker turns absent. | Audit drill-in (Scene 6) AND any worker-Activity-tab drill-in lands on theatrical scaffolding, not a real action. |

---

## All findings

### F1 [P0] — Gateway returns 1 worker "Maya", not the 5-worker deck the runbook expects

`curl http://127.0.0.1:7400/api/v1/team`:
```json
{"total_count":1,"workers":[{"name":"Maya","id":"019e5dbb-c03d-79a0-a9a4-ee60bb3c8218","department":null,"role":{"tagline":"Marketing intern that drafts and asks before publishing.","vertical":"marketing-intern"},"status":"idle",...}]}
```
Browser screenshot of `/team`: hero says **"Your first teammate is working."**, count chip says **"1 teammate working"**, deck shows one MA tile. Runbook Scene 1 line 100-103 says five cards re-render. The mock fallback would not save it either: `web/lib/sample-personas.ts:181-188` `SAMPLE_PERSONA_IDS` still lists `MARIAH / PRIYA / JAMAL / NORI / SAM / RIO` — Peta is defined as a persona (line 122) but omitted from the ID list. `DEMO_SEED_PERSONAS` is correct (Mariah/Peta/Priya/Jamal/Nori), but the live SPA reads from `SAMPLE_PERSONAS`/`SAMPLE_PERSONA_IDS`, not the seed list.

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/team | jq` — total_count=1
- Open http://localhost:3000/workers → redirects to `/team` → one "MA" Maya card

Suggested fix: re-seed the gateway with the 5 demo personas (the seed CLI route per `mission-demo-seed-data-bootstrap`). At minimum, drop SAM and RIO from `SAMPLE_PERSONA_IDS` and add PETA, so the in-app mock-bypass fallback matches the runbook.

`web/lib/sample-personas.ts:181-188`

---

### F2 [P0] — No "Schedules" or "Duties" tab on worker detail; DutiesCard sits inside Overview with no Run-now

Tab strip per `web/components/skycot/WorkerTabs.tsx:19-27`:
```
overview · activity · memory · skills · trust · escalation · settings
```
A `DutiesCard` titled "Responsibilities" *does* render on Overview (`web/components/skycot/worker/WorkerOverviewContent.tsx:11` and `web/components/skycot/worker/DutiesCard.tsx`), but it has no "Run now" / Play button. The only per-row CTAs are an enable/disable toggle and a click-to-edit, plus a "+ Add responsibility" footer button. The runbook Scene 2 expects:
1. Click into the "Schedules" tab — **no such tab.**
2. Click "Run now" on a duty — **no such button.**

`grep -rn 'Run now|run-now|RunNow|RunDuty' web/components/skycot/ web/app/(app)/workers/` returns only `app/(app)/workflows/[verticalId]/[name]/page.tsx:280` — that's the workflow-detail Run-now, not duty-level.

Reproduction:
- Open `/workers/<id>/overview` — see Responsibilities card (or empty state if no duties)
- Search the page for "Run now" — absent
- Click into `/workers/<id>/duties` → 404 (`[tab]/page.tsx` `notFound()`)

Suggested fix: pair with F3. Add a Schedules tab + a per-row Play button that POSTs to a `/api/v1/workers/<id>/duties/<id>/fire` endpoint. Until that ships, rewrite Scene 2 to use the Workflow detail "Run now" path (which exists) — but that re-uses Scene 5's beat, so the demo's claim-2 narration needs a different camera-visible action.

`web/components/skycot/WorkerTabs.tsx:19-27`, `web/components/skycot/worker/DutiesCard.tsx`

---

### F3 [P0] — Gateway returns `duties: []` for Maya AND all `permissions.can_edit_duties` are false

`curl http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`:
```json
{"charter":{...},"duties":[],"kpis":[...],"permissions":{"can_archive":false,"can_delete":false,"can_edit_charter":false,"can_edit_duties":false,"can_pause_resume":false,"can_view_audit":false,"can_view_settings":false},"recent_activity":[]}
```
So even if Scene 2's "click Run now" path shipped, the camera would land on "No responsibilities yet" with a disabled Add button. Recent activity is also `[]` — so the Overview's RecentActivityCard is empty. The combined effect: an entire Overview tab full of empty cards.

Reproduction:
- Curl above
- Visit `/workers/<id>/overview` — Responsibilities card shows "No responsibilities yet"; Recent activity card is empty; Add Responsibility button is disabled

Suggested fix: gateway seed should populate 3 duties per persona (Daily at 09:00, Weekly Mon at 08:00, On every X) per the runbook's expectation. Authentication path needs to grant `can_edit_*` permissions when the operator's principal hits the gateway in demo mode (currently the call is anonymous).

`crates/argo-cli/src/handlers/gateway/*.rs` (per DELTA.md tenant-resolution-cwd-vs-principal-mismatch)

---

### F4 [P0] — DecisionChainCard renders 8 empty rows; no delegation chain anywhere

`curl http://127.0.0.1:7400/api/v1/audit/<actionId>` payload:
```json
"decision_chain":[
  {"n":1,"title":"Memory loaded","body":"","details":{"snapshot_id":null}},
  {"n":2,"title":"Skill chosen","body":"","details":{"skill_id":null,...}},
  ... (8 entries total, all body:"" and details:null) ...
]
```
The card renders all 8 rows with title + empty body. No `delegated_by` / `delegated_to` / `parent_action_id` / `handoff_id` field on the payload at all. `grep -rnE 'delegat|handoff' web/components/skycot/audit/ web/components/skycot/ActivityRow.tsx` returns zero non-test hits. The runtime reportedly threads `ToolContext.current_handoff_id` (per CLAUDE.md Claim 2 LIVE block) but it never surfaces to the SPA.

For an investor watching Scene 2's "delegation is the unit" talk track and drilling into the audit row, the destination is **theatre the camera will see** under the function-vs-dressing rule — an 8-stage scaffold with no content and no delegation surface.

Reproduction:
- Open `/audit`, click any row → Decision Chain shows 8 titles, no bodies
- Read `web/components/skycot/audit/DecisionChainCard.tsx` and `web/types/audit-detail.ts` — no delegation fields on `DecisionStep`

Suggested fix: ship `fix-delegation-chain-card-visual-surface` (already authored per DELTA.md). Add `parent_action_id`, `delegated_by`, `delegated_to`, `handoff_chain[]` to the audit-detail API and render a `DelegationChainCard` (or extend `DecisionChainCard`) above the existing 8-stage trace.

`web/components/skycot/audit/DecisionChainCard.tsx`, `crates/argo-acp/src/audit_detail.rs` (or wherever the audit-detail handler builds the payload)

---

### F5 [P0] — Live audit feed contains only "audit.detail.viewed" meta-events with worker.name = "unknown"

`curl http://127.0.0.1:7400/api/v1/audit?range=today` returns 4 items, every one of them like:
```json
{"action_type":"audit.detail.viewed","verb":"http.request","worker":{"id":"","name":"unknown"},"channel":"web","status":"succeeded"}
```
The audit feed is self-referential — every row is "the SPA looked at the audit page". No real agent turns. Scene 6 ("audit page renders >=10 rows in Etherscan-style table with hash-chain hover. Click a row → real provenance") cannot record as scripted; **Scene 2 audit-row-lands-within-5s also cannot** because there is nothing for a duty fire to write besides another `http.request` meta-row.

This is the same root cause as F3 — the demo-seed CLI hasn't run real agent turns into the gateway under operator-locked **No Theater** policy (memory `feedback_no_theater_in_demo`).

Reproduction: curl above.

Suggested fix: run `argo demo seed --tenant <demo>` per CLAUDE.md operator-locked process, or the `mission-demo-seed-data-bootstrap` Option B real-backend seed CLI. **Do not** add a mock-bypass — operator explicitly rejected that 2026-05-24.

---

### F6 [P0/P1?] — `/api/v1/workers/<id>` fails with `net::ERR_FAILED` from cross-origin SPA; "Worker not found" lands

New since 08:20 audit. Browser preview was running on port 53854; the SPA's `useWorker` hook fetched `http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` and got `net::ERR_FAILED` (visible in `preview_network` output). The OPTIONS preflight succeeded (204), but the GET that followed failed. Direct curl returns 200. The same call from `localhost:3000` succeeds. The worker-detail page renders "Worker not found / The worker '...' doesn't exist or you don't have access." — a confusing dead-end if the operator records from any browser running on a non-`localhost:3000` origin.

Severity: marked P0 because if the operator records from anywhere other than `localhost:3000` (e.g. an OBS browser source pointing at a different port, or a deployed staging URL), the demo crashes immediately. Demote to P1 if the operator commits to `localhost:3000` exclusively.

Reproduction:
- From `localhost:3000`: `curl -s http://127.0.0.1:7400/api/v1/workers/<id>` returns 200
- From a browser at `localhost:53854`: fetch returns `net::ERR_FAILED`, SPA renders "Worker not found"

Suggested fix: widen gateway CORS allow-list to all `localhost:*` and `127.0.0.1:*` origins in dev mode, or document that demo recording must come from port 3000 only.

`crates/argo-gateway/src/cors.rs` (or wherever the CORS layer is configured)

---

### F7 [P1] — Personality card hardcoded by English name; Maya/Peta/Nori fall back to generic "Reliable/Curious/Helpful"

Unchanged from 08:20 F6. `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` keys personalities by name string for Mariah / Priya / Jamal only. Live worker "Maya" + canonical demo personas Peta and Nori all fall through to `DEFAULT_PERSONALITY`. **Claim 1's "teammate not tool" register breaks the moment the camera scrolls to a non-Mariah persona.**

Reproduction: open `/workers/<peta-id>/overview` (if seeded) or `/workers/<maya-id>/overview` (current state) — see "Reliable / Curious / Helpful" generic chips.

Suggested fix: source from `worker.traits` (already exists on `SamplePersona` per `sample-personas.ts:152, 168`) instead of name lookup. Extend `PERSONALITIES` map to include Peta / Nori / Sam / Rio at minimum.

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-117`

---

### F8 [P1] — Workers have no photo. Gateway returns `avatar_url: null`; no persona-avatar files bundled

Unchanged from 08:20 F7. `ls web/public/personas/ web/public/avatars/` both 404. `<TeammateCard>` falls back to text initials in a gradient tile. Per Claim 1's "teammate not tool" register, the photo dimension is missing. Dressing-side per the function-vs-dressing rule, but the camera will catch initials-only avatars across the deck.

Reproduction:
- `curl http://127.0.0.1:7400/api/v1/team | jq '.workers[0].avatar_url'` → `null`
- `ls web/public/personas/ web/public/avatars/` → both missing

Suggested fix: drop AI-generated headshot PNGs into `web/public/personas/<persona-id>.webp`, populate `SamplePersona.avatarUrl`, return non-null `avatar_url` from the gateway team listing.

---

### F9 [P1] — `/workers` redirects to `/team`; URL flicker visible during recording

Unchanged from 08:20 F8. `web/app/(app)/workers/page.tsx` client-side redirects to `/team` when workers exist. Runbook says open `/workers`; viewer sees a brief spinner then `/team` in the address bar. Small but camera-visible URL change that breaks the narration "we just typed /workers".

Reproduction: open `localhost:3000/workers` — observe redirect to `/team`.

Suggested fix: either canonicalise on `/workers` (move `/team` to redirect TO `/workers`), or rewrite runbook to say `/team`.

`web/app/(app)/workers/page.tsx:25-27`

---

### F10 [P1] — Team-hero KPIs are pure multipliers on workers.length, not real aggregations

Unchanged from 08:20 F9. `web/app/(app)/team/page.tsx:151-152`:
```ts
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```
Browser screenshot: hero shows **"0 working right now · 187 actions this week · saving you ~12 hours"** — derived from 1 worker × 187. Dressing-side, but if the camera reads the numbers and an investor asks "how is 187 computed", the answer is uncomfortable. With 5 personas seeded, the hero would say "935 actions / 60 hours" — a more theatrical fixture but still derived from worker count.

Suggested fix: either pull from a real aggregation endpoint or replace with copy that isn't a computable number ("This week's headline goes here once we have 7 days of data").

`web/app/(app)/team/page.tsx:151-152`

---

### F11 [P2] — "1 Issue" Next.js dev-tools red badge sits bottom-left every page during recording

New this run. Browser screenshot shows a fat red rounded chip "1 Issue ×" in the bottom-left, sourced from the Next.js dev overlay (`a051f_next_dist_compiled_next-devtools_index_*.js` per network listing). The dev overlay always loads in `next dev` mode regardless of `NEXT_PUBLIC_*` flags. If the operator records in dev (per the runbook's "open localhost:3000" setup), this chip is permanently in frame.

Reproduction: open any in-app page in `pnpm dev` — chip is bottom-left.

Suggested fix: either (a) record against a `next build && next start` production build (no overlay) per memory `project_v1_demo_mp4_ship_gate`'s pre-record checklist gap, or (b) document a `next dev --turbo --no-devtools` flag if it exists. Operator's existing checklist says "dev tools closed" but doesn't address the embedded Next.js overlay.

---

### F12 [P2] — "Demo mode — showing sample data. Sign in to see your own workspace." banner pinned top of every page

`web/components/skycot/DemoBanner.tsx`. Renders whenever `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` — which is the demo recording mode. Visible across the top of every in-app page. The line **"Sign in to see your own workspace"** explicitly tells the viewer the data isn't real. The banner IS dismissible (× on the right, persists in localStorage). The operator's pre-record checklist needs to include "dismiss demo banner once before recording" — that's a single click that survives the recording.

Reproduction: visit any in-app page in demo mode — banner is sticky at top.

Suggested fix: add to operator pre-record checklist. Optionally rephrase the banner copy to be more investor-neutral ("Sample workspace · evaluation mode") so even if undismissed it doesn't read as "fake."

`web/components/skycot/DemoBanner.tsx:44-46`

---

### F13 [P2] — Activity tab empty-state copy hardcodes she/her pronoun

Unchanged from 08:20 F10. `web/app/(app)/workers/[workerId]/activity/page.tsx:201-205`:
```
{workerName} hasn't taken any actions yet. Once she's working, you'll see her activity here.
```
Will read as misgendered for Jamal (male persona) and any worker with they/them pronouns. Scene 2 doesn't dwell here; P2.

Suggested fix: rewrite to neutral — "Once they start working, you'll see their activity here."

---

### F14 [P3] — Console warns on every worker-hero overflow menu click ("E4-a worker hero action ... pending wiring")

Unchanged from 08:20 F11. `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:122-123` emits a `console.warn` for any unwired hero action. Only visible if the operator opens devtools, but the runbook check should explicitly include "no devtools open."

Suggested fix: gate behind `NODE_ENV !== "production"` or remove until wired.

---

## Resolved since 08:20 audit

Confirmed unchanged: F1-F13 from the 08:20 audit are all still in effect with the same root cause. Nothing has shipped to gateway seed, the worker tab strip, the DutiesCard's Run-now affordance, the audit drill-in delegation surface, or the persona-id list in the 2h 8m between audits.

The DELTA.md's three highest-leverage fix-briefs (`fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` / `fix-replay-endpoint-contract-spa-vs-gateway-mismatch` / `fix-delegation-chain-card-visual-surface`) are still all unshipped in main as of the time of this run.

---

## Orientation score: 50 / 100

Reasoning:
- Visual chrome (C2 deck, gradient banner, personality card design) remains locked, shipped, and well-built — would score 80+ on static-mockup-rendering.
- Of the five things the operator asked me to verify this run (5-worker deck, worker detail tabs, Schedules/Duties, Run-now, audit landing), three (Schedules tab, Run-now button, delegation chain rendering) don't exist in the SPA at all, and the gateway seed for the other two is empty.
- Claim 1 (Teammate, not tool) reads as colleague for Mariah only; degrades to "Reliable / Curious / Helpful" config defaults for every other persona including the only live one (Maya). The +1 point vs the 08:20 audit's 52 comes from the C2 deck's polish.
- Claim 2 (Delegation is the unit) has zero UI surface. The audit drill-in renders 8 empty rows over `audit.detail.viewed` self-referential events.
- New defect F6 (cross-origin gateway fetch failure) makes the demo additionally fragile to recording-environment choices.

The path from 50 → 85: ship the 3 DELTA.md P0 fix-briefs (tenant-resolution + delegation-chain + replay-endpoint) + the seed-data CLI (5 personas with 3 duties each + 10+ real agent turns per persona) + a Schedules tab with Run-now per `responsibilities-and-schedules` brief tree. Estimate 3-5 days focused work, smaller if scoped strictly as a recording-day seed-and-render.
