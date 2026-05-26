# UX audit — Scenes 1 + 2 (Claims 1 + 2)

**Date**: 2026-05-27 01:19 AEST (T0119MEL)
**Scope**: `/workers` deck (Mariah / Peta / Priya / Jamal / Nori), `/workers/{id}` overview + tabs, Schedules/Duties surface, the "Run now" affordance, the click-through audit row for the run.
**Environment**: dev server (web port 3000), gateway port 7400 — both UP. `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. `argo-store/v83` per the operator-supplied context.
**Method**: Preview MCP server is wedged (timed out on `preview_snapshot` + `preview_eval` + every subsequent call). All findings sourced from direct source-code reading + `curl` against the gateway + diff vs prior audits at `2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` and `2026-05-26-2309/scene-1-and-2-claim-1-and-2.md`. Function-vs-dressing rule applied throughout.
**Delta vs T0015MEL (66 min earlier)**: zero workers-code commits in the window (last 5 commits all touched `/workflows` or docs). Findings persist 1:1 from T0015MEL — re-verified, not re-discovered.

---

## TL;DR

Scenes 1 + 2 are **still not recordable as the runbook scripts them**. Same five demo-blocking gaps as T0015MEL persist on origin/main: (1) the `/workers` roster is a 404-fallback to `SAMPLE_TEAM` with a visible "MOCK-OFFLINE" pill in dev mode — the camera will see it, and the "stop / restart gateway" persistence beat is theatre because the workers never came from the gateway; (2) there is no Duties or Schedules tab in `WORKER_TAB_IDS` — Scene 2 step 1 has no tab to click into; (3) there is no "Run now" button anywhere on the worker-detail page — Scene 2 step 2 cannot be performed; (4) the gateway returns zero `worker_duty.fired` rows and the only audit kinds present in the database are `audit.replay` and `audit.detail.viewed` — Scene 2 pass cue is unsatisfiable; (5) the audit drill-in `delegationChain` field exists in the type but **no fixture writes one** — the parent → child delegation chain the runbook narrates is invisible in /audit detail. Findings: 10 (P0 × 5, P1 × 3, P2 × 2). All but one are dedupes of the 0015 audit; one (F10) is net-new — the demo-mode banner overlays the page even on /workers, occupying camera vertical space the runbook didn't budget for.

## Headline blockers (persistent — no progress since T0015MEL)

1. **F1 [P0]** — `/workers` deck is `SAMPLE_TEAM` mock + visible "MOCK-OFFLINE" pill. Scene 1 persistence beat is theatre the camera sees.
2. **F2 [P0]** — No Duties / Schedules tab in `WORKER_TAB_IDS`. Scene 2 step 1 unrecordable.
3. **F3 [P0]** — No "Run now" affordance on `DutyRow`. Scene 2 step 2 unrecordable.
4. **F4 [P0]** — Gateway audit table has zero `worker_duty.fired` rows; pass cue unsatisfiable.
5. **F5 [P0]** — Audit drill-in `delegationChain` is wired in the renderer but no sample fixture populates it — claim-2 evidence the camera drills into is empty.

---

## All findings

### F1 [P0] — `/workers` roster is mock data; "MOCK-OFFLINE" pill is visible on-camera (dev mode)

`curl http://127.0.0.1:7400/api/v1/workers` → `HTTP 404 {"error":"not found"}`. The SPA falls through to `SAMPLE_TEAM` in `web/lib/sample-team.ts`, and `web/app/(app)/team/page.tsx:318-329` renders a `<TeamDataSourceChip>` adjacent to the "Team" heading with the text "mock-offline" (or "mock-bypass" with the bypass flag on). The chip is suppressed only when `process.env.NODE_ENV === "production"`. The demo runbook does not say to record from a `next build` production server, and the operator runs `npm run dev` (NODE_ENV=development) — so the pill is in shot during Scene 1.

The Scene 1 talk-track "I'm going to stop the gateway, restart it, and they're all still here" implies a gateway-backed roster. The roster never came from the gateway in the first place: stopping it produces the same mocks, restarting it produces the same mocks. The "proof of claim 1 — persistence" is unsupported by the function-side wiring.

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/workers` → 404
- Open `/team` in dev → 5 workers + `mock-offline` chip next to "Team" heading
- Stop gateway, reload → still 5 workers + same chip — proves nothing about persistence

Files:
- `~/code/argo/web/lib/api/team.ts:30,90` (`source: "mock-offline"` on 404 fallback)
- `~/code/argo/web/app/(app)/team/page.tsx:253,318-329` (chip rendered next to heading; dev-only)
- `~/code/argo/web/lib/sample-team.ts` (hard-coded 5-worker roster)

Suggested fix: either ship the `/api/v1/workers` gateway endpoint backed by seed data (real — function-side fix), or hide the data-source chip in demo mode and re-script Scene 1's narration to drop the persistence claim. Per the function-vs-dressing rule, the persistence claim is **function-side** — hiding the chip alone is not enough; the camera-drilled-into proof still doesn't exist.

---

### F2 [P0] — No Duties / Schedules tab on the worker-detail page

`WORKER_TAB_IDS` at `web/components/skycot/WorkerTabs.tsx:19-27` contains exactly: `overview`, `activity`, `memory`, `skills`, `trust`, `escalation`, `settings`. No Duties tab, no Schedules tab. The runbook (Scene 2 step 1) says "click into her Duties tab" — there is no such tab.

Duties render inline on the Overview tab as `<DutiesCard>` (`WorkerOverviewContent.tsx:151-159`). Typing `/workers/<id>/duties` directly hits the `[tab]/page.tsx` catch-all and calls `notFound()` — visually broken (but not a 500).

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001` (Mariah). Tab strip lists 7 tabs, none labelled Duties or Schedules.
- Curl `/workers/<id>/duties` → 200 page-shell, body renders worker-shell not-found content.

Files:
- `~/code/argo/web/components/skycot/WorkerTabs.tsx:19-39`
- `~/code/argo/web/app/(app)/workers/[workerId]/[tab]/page.tsx` (catch-all → `notFound()`)

Suggested fix: rewrite Scene 2 to say "scroll to her Schedules card on Overview". The runbook should match what ships. (The operator-locked vocab is Schedule in UI; if a tab is added, label it "Schedules".)

---

### F3 [P0] — No "Run now" affordance on any duty row

`rg "Run now"` against `web/components/skycot` + `web/app` returns exactly one match: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263` — the workflow detail page. The worker page has no run-trigger UI anywhere. `DutyRow` in `DutiesCard.tsx:138-197` exposes only (a) toggle on/off (`DutyToggle`, aria-label "Enable schedule" / "Disable schedule") and (b) clicking the label.

Clicking the label fires `onEditDuty` → `setAnnouncement("Edit schedule ships in E4-h")` (`WorkerOverviewContent.tsx:78-81`). The "Add responsibility" button at the bottom of the card opens a real dialog (E4-h has shipped for *add*), but the per-row run-now affordance has not.

Reproduction:
- Open `/workers/<id>` → Overview → DutiesCard. Inspect any row.
- No button, link, or menu item labelled "Run now" anywhere on the page.
- Click a duty label → tiny toast appears with `"Edit schedule ships in E4-h"` — engineering brief-ID copy visible on camera.

Files:
- `~/code/argo/web/components/skycot/worker/DutiesCard.tsx:138-197`
- `~/code/argo/web/components/skycot/worker/WorkerOverviewContent.tsx:74-93`

Suggested fix: highest-leverage Scene-2 demo blocker. Two options. (a) Pivot Scene 2 to the `/workflows/<vertical>/<name>` "Run now" button that already works (matches Scene 5; cheap re-script). (b) Add a per-row Run-now button wired to a new `POST /api/v1/workers/<id>/duties/<dutyId>/run` gateway endpoint (real but multi-brief). Option (a) is the v1.0 path.

---

### F4 [P0] — Gateway audit table has only `audit.detail.viewed` + `audit.replay` rows; `worker_duty.fired` is empty

`curl 'http://127.0.0.1:7400/api/v1/audit?limit=200'` returns 5 rows total, all of action_type `audit.detail.viewed` or `audit.replay`. `?action_type=worker_duty.fired` returns `{"items":[]}`. The audit pipeline for actual worker duty firings has either not landed or has never been exercised on this gateway instance. Scene 2's pass cue ("`worker_duty.fired` row appears in /audit within 60 seconds") cannot be satisfied.

Even the kind-filter spelling differs: the gateway silently ignores `?kind=` (returns *all* rows when given that param) — the SPA uses `?action_type=` (`web/lib/filters/audit.ts:76-80`). The runbook still uses `kind =` in the narration.

Reproduction:
- `curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=200' | python3 -c "import json,sys; d=json.load(sys.stdin); print(sorted({i['action_type'] for i in d['items']}))"` → `['audit.detail.viewed', 'audit.replay']`
- `curl 'http://127.0.0.1:7400/api/v1/audit?action_type=worker_duty.fired'` → `{"items":[]}`

Files:
- gateway audit handler (out of scope to identify)
- `~/code/argo/web/lib/filters/audit.ts:76-80` (URL key is `action_type`)
- `~/code/argo/docs/skycot/demos/v1-mission-mvp-runbook.md:144-145` (uses `kind =`)

Suggested fix: depends on F3 resolution. If Scene 2 pivots to a workflow run (F3 option a), the pass cue becomes `workflow.step.executed` instead — but the audit table currently has none of those either, so the deeper gap is the audit pipeline not emitting domain rows in this gateway instance. Operator may need to run a `argo demo seed --tenant <demo>` step before recording to populate the table; if no seed CLI exists, that's a separate brief.

---

### F5 [P0] — Audit drill-in `delegationChain` is wired in renderer but NO fixture populates it; Scene 2 → /audit drill-in shows nothing

`web/lib/api/audit-detail.ts:24-36` defines `DelegationStep` + `AuditDetailWithDelegation`. The drill-in renderer expects `delegationChain` array. `rg "delegationChain|delegation_chain"` against `web/lib/sample-*` returns **zero hits** — no sample fixture writes the field. The gateway returns nothing either (audit table is empty per F4).

This is the function-side claim-2 proof: the runbook narrates "delegation is the unit — Mariah's duty fires → audit row shows parent→child". The camera clicks an audit row, the detail modal opens, the delegation chain section is empty. Per the function-vs-dressing rule, drill-in destinations must be real — a blank chain on the *exact element the narrative points at* is theatre.

Reproduction:
- `rg "delegationChain|delegation_chain" web/lib/sample-*` → no hits
- (Could not verify via preview — wedged — but the code path is conclusive.)

Files:
- `~/code/argo/web/lib/api/audit-detail.ts:24-36,140-180` (type + normalizer)
- `~/code/argo/web/lib/sample-audit-detail.ts` (no chain populated)
- `~/code/argo/docs/skycot/demos/v1-mission-mvp-runbook.md:143-147` (Scene 2 narrates the chain)

Suggested fix: add at least one sample audit detail entry with a populated `delegationChain` for a Mariah duty-fired row (originator: human operator → delegated_to: Mariah → executed: runner). Wire that fixture into the audit list response so the operator can click it from /audit during Scene 2.

---

### F6 [P1] — Peta still missing from `TRAITS_BY_NAME` + `LEVEL_BY_NAME`; her card uses hash-derived level

`web/components/skycot/TeammateCard.tsx:51-67` defines TRAITS / LEVEL maps for Mariah, Priya, Jamal, Nori, Sam, Rio. **Peta is still absent** (re-verified at 01:19 — unchanged since T0015MEL). Her card renders without curated traits (empty traits row) and her LEVEL number falls through to `hashLevel("Peta")` (string-hash-derived). On a 5-card hero deck, the one card without traits is conspicuous.

Of note: Peta IS in `sample-personas.ts:122-137` with proper traits ("Cadence-true", "Follow-through", "Calm escalator", "Context keeper"). The bug is the TeammateCard's per-name override map ignores the persona traits and uses its own hard-coded TRAITS_BY_NAME table that hasn't been updated to include Peta.

Reproduction:
- `grep -n "Peta" web/components/skycot/TeammateCard.tsx` → no match
- `grep -n "Peta" web/lib/sample-personas.ts` → `name: "Peta"` confirmed (line 124)

Files:
- `~/code/argo/web/components/skycot/TeammateCard.tsx:51-67`

Suggested fix: either (a) add Peta to TRAITS_BY_NAME + LEVEL_BY_NAME, or (b) refactor `traitsFor()` / `levelFor()` to pull from `sample-personas.ts` (which is already the canonical persona spine). Option (b) prevents future drift for the next worker the operator names.

---

### F7 [P1] — Peta + Nori fall through to DEFAULT_PERSONALITY on worker-detail page

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` PERSONALITIES map covers only Mariah / Priya / Jamal. Peta + Nori (both in the canonical 5-roster) render the generic `DEFAULT_PERSONALITY` ("Reliable / Curious / Helpful" traits; placeholder "Always confirms before a public or irreversible action" signature). Scene 1 step 5 only opens Mariah's profile so this is P1 not P0 — but if the operator's cursor drifts to Peta or Nori during the take, the difference is conspicuous.

This is unchanged since T0015MEL. The persona spine has Peta + Nori with real traits; the WorkerPersonalityCard component just hasn't been updated to consume them.

Files:
- `~/code/argo/web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`

Suggested fix: same as F6 — either inline Peta + Nori PERSONALITIES, or refactor to read from `sample-personas.ts` traits + extend persona schema with greatAt/escalateWhen/signature so it's the single source.

---

### F8 [P1] — Worker activity tab has NO live streaming; Scene 2 step 3 "streams in real time" unsupported

Scene 2 step 3 narration: "The activity feed streams in real time: turn start → runner invocation → `WorkerDutyFired` audit row." `web/lib/hooks/use-worker-activity.ts` has no `refetchInterval`, no SSE, no polling — it's a one-shot React Query. The page does not auto-refresh after a click elsewhere. The camera will not see anything stream; rows arrive only on full page reload.

This compounds with F3 (no Run-now to fire the duty) and F4 (no `worker_duty.fired` rows exist anyway). But even if F3/F4 were fixed, the activity tab still wouldn't *stream* — it would require a page reload to show the new row.

Files:
- `~/code/argo/web/lib/hooks/use-worker-activity.ts` (no polling/refresh config)
- `~/code/argo/web/app/(app)/workers/[workerId]/activity/page.tsx` (consumer)

Suggested fix: add `refetchInterval: 3000` to the query (cheap polling will pass the camera test). Or re-script: "I refresh — there's the audit row." But streaming is the more credible story for claim 2; polling is enough.

---

### F9 [P2] — Sidebar still has both "Team" and "Workers" routes (same destination, two labels)

Persisting from T0015MEL F9. `web/components/skycot/nav-items.tsx:24-25` shows both `{ href: "/team", label: "Team" }` and `{ href: "/workers", label: "Workers" }`. `/workers` redirects to `/team`. Two adjacent nav items pointing at the same page risk hesitation on-camera. Sidebar is briefly in shot during scene transitions.

Suggested fix: drop one. "Team" matches the page heading + brand vocabulary + the URL the user lands on.

Files:
- `~/code/argo/web/components/skycot/nav-items.tsx:22-25`
- `~/code/argo/web/app/(app)/workers/page.tsx` (redirect shim)

---

### F10 [P2] — DemoBanner stickily covers the top of every page including /workers and /workers/{id} — eats camera vertical real-estate (NET-NEW vs T0015MEL)

**Net-new finding** not surfaced in T0015MEL. `DemoBanner` (`web/components/skycot/DemoBanner.tsx`) renders `sticky top-0 z-30` with the copy "Demo mode — showing sample data. Sign in to see your own workspace." whenever `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. It is dismissible (X icon) and the dismissed state persists in localStorage — but on a fresh recording profile (no prior localStorage), the banner is in shot at the top of *every* page including /workers and /workers/{id}.

Two camera-side problems:
1. "Showing sample data" copy directly contradicts the Scene 1 talk-track "I hired these last week" — the viewer reads the banner first.
2. The banner is brand-loud (purple→pink gradient) and pushes the BrandLoudHero down, breaking the trading-card-deck visual rhythm.

The operator can pre-dismiss before take (set `localStorage["skycot:demo-banner-dismissed"]="1"`), but a fresh recording profile re-shows it.

Reproduction:
- `localStorage.clear()` then load `/team` → banner visible, dismissed-state false
- The banner text "showing sample data" is in the same camera frame as the hero "5 teammates working" claim

Files:
- `~/code/argo/web/components/skycot/DemoBanner.tsx`

Suggested fix: pre-dismiss before the take (cheapest); OR change the banner copy to something that doesn't contradict the talk-track ("Skycot live walkthrough — Acme Co demo workspace") — keep it visible (honesty) but don't have it argue with the script.

---

## Resolved since T0015MEL audit

Zero findings resolved. Last 5 commits on origin/main touched `/workflows` dark-mode + the gateway-store helpers brief + docs only — none touched `/workers`, `/team`, `DutiesCard`, `WorkerPersonalityCard`, or the audit pipeline. The persistent 5-P0 cluster is unchanged.

The T0015MEL DELTA doc identified 16 P0 findings, of which 11 had `fix-*` briefs in flight (IN_REVIEW or CLAIMED). None of the briefs in flight target Scenes 1+2 specifically — the gates that block this scope are:
- a Duties/Schedules tab brief (not authored)
- a worker Run-now button + gateway endpoint brief (not authored)
- a `worker_duty.fired` audit row population brief (not authored)
- a `delegationChain` fixture brief (not authored)

The runbook misalignment brief (`fix-runbook-scene-1-2-align-with-mariah-actual-duties`) was queued in the T0015MEL synthesis batch but **does not address the F2/F3/F4/F5 underlying gaps** — it only fixes the doc, not the software.

## Orientation score

**44 / 100.**

Marginally lower than T0015MEL's 46 because the net-new F10 (DemoBanner contradicting talk-track copy) compounds the persistent 5-P0 cluster. The visual scaffolding remains in place; the C2 trading-card deck looks great and the worker-detail layout is clean. The gap is unchanged: the runbook's narrative claims about Scenes 1+2 are not matched by the working software. The cheapest path to a recordable demo is **rewrite Scenes 1+2 around what actually works** — drop the gateway-restart persistence proof, point at the inline Schedules card on Overview, use `/workflows/<id>` "Run now" instead of a duty Run-now, populate at least one audit-detail fixture with a `delegationChain` for the click-through proof. The gateway-side proof of persistence + the duty-cron pipeline are v1.1 cycle work.
