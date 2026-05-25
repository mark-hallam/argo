# Scenes 1 + 2 — Claims 1 + 2 (Teammate-not-tool, Delegation-is-the-unit)

**Date:** 2026-05-26 08:20 AEST
**Scope:** `/workers` deck + `/workers/{id}` tabs (Mariah + Peta) + Schedules/Duties "Run now" + audit-row landing.
**Tester:** e2e-ux-journey-test-agent (code-read + curl mode; no browser MCP this run).
**Lens:** function-vs-dressing rule per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

---

## TL;DR

**5 P0, 4 P1, 4 P2, 2 P3 — 15 findings total.**

The bad news, in plain English: **Scene 1 and Scene 2 are not recordable as scripted today.** The 5-worker trading-card deck the runbook narrates (Mariah / Peta / Priya / Jamal / Nori) does not exist on the running gateway — the live `/api/v1/team` returns a single worker named "Maya" with no avatar, no department, no headline activity, and no traits. The Worker detail page has no Schedules / Duties tab and no "Run now" affordance anywhere; the operator-listed tab set (Profile / Skills / Schedules / Runs / Audit) does not match what shipped (Overview / Activity / Memory / Skills / Trust / Escalation / Settings). Claim 2 ("delegation is the unit") has zero UI surface in the audit-action drill-in — the `DecisionChainCard` renders a generic 8-step trace with no delegated-by / delegated-to / handoff-chain rendering, even though the runtime is reportedly carrying the chain through `ToolContext.current_handoff_id`. Claim 1 ("teammate not tool") is half-built: the personality card (traits, Great-at, Escalate-when, Signature) is implemented beautifully but keyed by hardcoded English name — Maya falls back to a generic "Reliable / Curious / Helpful" persona. The C2 holographic deck (gradient banner, animated shine, weight-900 numbers) IS implemented, but with one card it never reads as a "deck".

**Headline blocker for the v1.0 demo MP4:** the gateway seed data has to be regenerated to produce the 5 expected workers, and either the duties endpoint needs to ship + a Schedules/Duties tab + "Run now" button, or Scene 2 has to be rewritten to a flow that actually exists today.

---

## Headline blockers (top 5 P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Gateway returns **1 worker "Maya"**, not the 5 expected. | Scene 1 can't pan across the deck; Scene 2 has nobody named Mariah to click into. |
| F2 | P0 | **No Schedules / Duties tab** on worker detail; tab list is Overview/Activity/Memory/Skills/Trust/Escalation/Settings. | Scene 2 step 1 (click "Duties" tab) impossible. |
| F3 | P0 | **No "Run now" affordance** anywhere in the SPA. `grep` for `Run now`, `RunNow`, `run.*duty` returns zero hits. | Scene 2 step 2 (click Run now) impossible. |
| F4 | P0 | **No `/api/v1/duties` endpoint** on the gateway (404). | Even if a UI shipped, there's nothing to call. |
| F5 | P0 | **No delegated-by / delegated-to / handoff-chain rendering** in `DecisionChainCard` or `ActivityRow`. Claim 2 has no visual surface. | Scene 2 talk-track "delegation is the unit" is not provable by drill-in; investor will ask "where do I see who delegated what to whom?" — answer: nowhere visible. |

---

## All findings

### F1 [P0] — Gateway returns 1 worker "Maya", not the 5-worker deck the runbook expects

`/api/v1/team` against the live gateway returns:
```json
{"total_count":1,"workers":[{"name":"Maya","avatar_url":null,"department":null,"headline":{"bullets":["No activity yet."],"tomorrow":null},"role":{"tagline":"Marketing intern that drafts and asks before publishing.","vertical":"marketing-intern"},"status":"idle"}]}
```

Runbook Scene 1 narrates "Five worker cards re-render post-restart with identical names" (`docs/skycot/demos/v1-mission-mvp-runbook.md:100-103`) — Mariah, Peta, Priya, Jamal, Nori. None of those are in the gateway. `SAMPLE_TEAM` (which would fall back in `mock-offline` mode) is also wrong: `SAMPLE_PERSONA_IDS` (`web/lib/sample-personas.ts:181-188`) lists `MARIAH / PRIYA / JAMAL / NORI / SAM / RIO` — Peta is defined as a persona (line 122) but is NOT in the ID list, and Sam/Rio aren't in the demo runbook. So even the mock fallback wouldn't render the right 5.

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/team | jq` — see total_count=1
- Open http://localhost:3000/workers → redirects to `/team` → shows one card "Maya"
- Compare to runbook Scene 1 expectation

Suggested fix: re-seed the gateway with the 5 demo personas (or rewrite SAMPLE_TEAM to match the runbook 5 and flip on `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` for the recording session).

---

### F2 [P0] — No "Schedules" or "Duties" tab on worker detail; runbook Scene 2 expects one

The shipped tab strip (`web/components/skycot/WorkerTabs.tsx:19-27`):
```
overview · activity · memory · skills · trust · escalation · settings
```

The operator-listed test scope expected **Profile / Skills / Schedules-or-Duties / Runs / Audit**, and Scene 2 step 1 says "click into her **Duties** tab" (runbook line 121). Closest existing surface is `/workers/<id>/responsibilities/new` which is a conversational workflow-author chat, not a cron-shaped duty list.

Reproduction:
- `curl http://localhost:3000/workers/<id>/duties` → 404 (`[tab]/page.tsx` `notFound()`)
- Read `web/components/skycot/WorkerTabs.tsx` — Duties / Schedules not in `WORKER_TAB_IDS`

Suggested fix: ship the Duties (or Schedules) tab page + add it to `WORKER_TAB_IDS` per memory `project_vocabulary_duty_vs_schedule` ("Schedule" in UI copy).

---

### F3 [P0] — No "Run now" button anywhere; runbook Scene 2 step 2 says operator clicks it

`grep -rnE 'Run now|run-now|run_now|RunNow|RunDuty' web/components/skycot/ web/app/(app)/workers/` returns **zero** matches.

Reproduction:
- Walk every tab on `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/*` — no Run-now CTA visible
- `grep` the codebase

Suggested fix: pair with F2 — the Duties tab needs a per-row Run-now button that POSTs to a `/api/v1/duties/<id>/fire` endpoint (which also needs authoring; see F4).

---

### F4 [P0] — No `/api/v1/duties` endpoint on the gateway (404)

```
$ curl http://127.0.0.1:7400/api/v1/duties → {"error":"not found"} [404]
$ curl http://127.0.0.1:7400/api/v1/workers/<id>/duties → {"error":"not found"} [404]
$ curl http://127.0.0.1:7400/api/v1/audit/actions?limit=3 → {"error":{"code":"not_found"}} [404]
```

The runbook Scene 2 live-test source is `crates/argo-orchestrator/tests/claim_2_live_duty_runs.rs` (cron fires → runner → `WorkerDutyFired` audit row). The backend may pass that test, but it isn't exposed as a list-endpoint the SPA can read or a fire-endpoint the SPA can POST to. **This is theatre the camera will see** — the runbook narrates a UI that doesn't connect to the backend.

Reproduction: curl loops above.

Suggested fix: author + ship `mission-gateway-duties-list-and-fire-endpoints` brief; until then, Scene 2 is unrecordable without a mock seam (which violates the No Theater rule).

---

### F5 [P0] — No delegation-chain rendering anywhere in audit surfaces; Claim 2 has no visual proof

`grep -rnE 'delegat|handoff|delegated_by|delegated_to|parent.*action|by.*worker' web/components/skycot/audit/ web/components/skycot/ActivityRow.tsx` returns **zero** non-test hits. `DecisionChainCard` renders a generic "8-step trace this action followed" (`web/components/skycot/audit/DecisionChainCard.tsx:38`) — no parent-action link, no who-delegated-to-whom edge.

The runtime reportedly threads `ToolContext.current_handoff_id` (per CLAUDE.md Claim 2 LIVE block referencing #2256), but the SPA never reads it. **An investor watching Scene 2's "delegation is the unit" talk track and looking for the actual delegation in the drill-in sees a generic 8-step trace and has no idea who delegated what to whom.**

Reproduction:
- Open any `/audit/action/<id>` page (in dev, this 404s because audit is empty; check code path)
- Read `DecisionChainCard.tsx` — no delegation fields on `DecisionStep`
- Read `ActivityRow.tsx` — only renders worker avatar of the acting worker, no parent / origin

Suggested fix: extend the audit-detail API to surface the handoff chain, then ship a `DelegationChainCard` (or extend `DecisionChainCard`) that shows "Delegated by → → → executed by" with worker chips and timestamps.

---

### F6 [P1] — Personality card hardcoded by English name; live worker "Maya" falls back to generic persona

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` defines personalities keyed by worker `name` string for Mariah / Priya / Jamal only. Peta, Nori, Sam, Rio (and Maya, the live one) all fall through to `DEFAULT_PERSONALITY`:
```
traits: ["Reliable", "Curious", "Helpful"]
greatAt: ["Working through the responsibilities you set", "Asking when something is ambiguous", "Surfacing patterns across recent runs"]
```

For the v1.0 demo recording: Mariah will look like a person; Peta and Nori (also named in Scene 1) will look like config defaults. This breaks Claim 1's "teammate not tool" register the moment the camera scrolls.

Reproduction: read the file; observe only 3 of the 5 expected names are wired.

Suggested fix: wire the personality data into `sample-personas.ts` (where `traits` already exists per line 152, 168) and pull from `worker.traits` instead of a name lookup; or extend `PERSONALITIES` to include Peta / Nori / Sam / Rio at minimum.

---

### F7 [P1] — Workers have no photo. Gateway returns `avatar_url: null`; no persona-avatar PNGs bundled

`web/public/personas/` and `web/public/avatars/` do not exist. `SamplePersona` type doesn't carry an `avatarUrl` field. `<Avatar>` falls back to text initials in a circle. Per Claim 1's "name, photo, personality traits", the photo dimension is absent.

This is **dressing-side** per the function-vs-dressing rule (worker persona is fictional), so it's P1 not P0 — but the camera will catch initials-only avatars in the deck and the worker hero, and they will read as "config rows with avatars" rather than colleagues.

Reproduction:
- Inspect `gateway → /api/v1/team` response: `"avatar_url": null`
- `ls web/public/personas/ web/public/avatars/` → both missing
- Inspect `<TeammateCard>` `holo-avatar-inner` div (line 247) — just text initials

Suggested fix: drop AI-generated headshot PNGs into `web/public/personas/<persona-id>.webp`, populate `SamplePersona.avatarUrl`, return non-null `avatar_url` from gateway when persona has one.

---

### F8 [P1] — `/workers` always redirects to `/team`; URL the runbook says to open isn't the canonical surface

`web/app/(app)/workers/page.tsx:25-27` — `useEffect(() => { if (hasWorkers) router.replace("/team") }, ...)`. Runbook Scene 1 says open `/workers`; viewer sees a brief loading spinner then `/team`. This is a small camera-visible flicker + a URL change in the address bar that breaks the narrative ("we just typed /workers but now it says /team").

Reproduction:
- `curl -I http://localhost:3000/workers` → 200 then a client-side replace fires
- Watch the address bar at 60fps during recording — `/workers` then `/team`

Suggested fix: either rename `/team` → `/workers` (canonical), make `/workers` SSR the same component, or rewrite the runbook narration to say "/team" not "/workers".

---

### F9 [P1] — Worker hero / TeamHero KPI numbers are computed multipliers, not real

`web/app/(app)/team/page.tsx:151-152`:
```ts
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```

These are **dressing-side** (KPI labels on a card), so they're not auto-P0. But the talk track for Scene 1 leans on "x actions this week, saving you ~y hours" — if the camera reads those numbers and the investor asks "how is that calculated", the answer is "workers.length × 187". For the POC demo, swap to either a real aggregation or stop showing the number.

Reproduction: read the file; observe the multiplier.

Suggested fix: hide the KPI line until real aggregation lands, OR move to mock-dressing label that isn't a computable number.

---

### F10 [P2] — Activity tab empty state copy uses she/her without checking worker pronouns

`web/app/(app)/workers/[workerId]/activity/page.tsx:201-205`:
```
{workerName} hasn't taken any actions yet. Once she's working, you'll see her activity here.
```

Hardcoded "she/her" — Jamal (male persona) and any worker with they/them pronouns will read as misgendered. P2 because Scene 2 doesn't dwell here.

Reproduction: visit `/workers/<jamal-id>/activity` with empty activity.

Suggested fix: rewrite to neutral phrasing — "Once they start working, you'll see their activity here."

---

### F11 [P2] — Worker overflow menu warns to console on click; warning will appear in dev tools during recording

`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:122-123`:
```ts
console.warn(`[E4-a] worker hero action '${action}' pending wiring (E3-b/E4-f)`);
```

If the operator opens dev tools (per memory `project_v1_demo_mp4_ship_gate` they're told to close them, but they sometimes don't) the warning is visible. Cosmetic but breaks "production-ready" feel.

Reproduction: open worker hero overflow menu, click any item, see console warn.

Suggested fix: gate behind `process.env.NODE_ENV !== "production"` or remove until wired.

---

### F12 [P2] — Team-data-source chip shows raw label ("gateway" / "mock-offline" / "unreachable") in dev

`web/app/(app)/team/page.tsx:316-329` renders a chip with `data-source` like `gateway` / `mock-bypass`. Code guards with `NODE_ENV === "production"` so it disappears in prod — but during recording the operator records dev. The chip will be next to the "Team" heading in the camera frame.

Reproduction: open `/team` in dev → see chip "GATEWAY" next to title.

Suggested fix: also gate behind `NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1"` so the chip hides during recording.

---

### F13 [P2] — `WorkerHero` lazy-loads overflow menu via `next/dynamic`; first menu open has a small delay

`web/components/skycot/WorkerHero.tsx:39-42` — `LazyOverflowMenu = dynamic(() => import("./WorkerOverflowMenu"), { ssr: false })`. First click loads the chunk; on slow first-click, the menu appears ~100-300ms after the click. The runbook never opens this menu, so this is purely a passing note — but if the operator improvises an overflow-menu open during recording, expect a perceptible lag.

Suggested fix: prefetch on hover, or move to non-lazy once chunk size measured.

---

### F14 [P3] — `/api/v1/team` envelope handler tolerates bare-array OR object — defensive but obscures schema contract

`web/lib/api/team.ts:64-84` handles `Worker[] | { workers: Worker[] }` — a temporary tolerance for old gateway builds (per the comment). Now the gateway returns object envelope, so the array branch is dead. Not blocking anything.

Suggested fix: drop the array branch + tighten the type once gateway schema is stable. Mention in a tech-debt sweep.

---

### F15 [P3] — Two "audit" endpoint candidates 404 (`/api/v1/audit/actions`, `/api/v1/audit/events`); only `/api/v1/audit` (returns `{"items":[]}`) responds

The client-side `useAuditDetail` would call action-detail at some path; the surface-pinning ledger doesn't make it obvious which path is canonical. With empty audit data, the drill-in flow can't actually be tested end-to-end against this gateway.

Suggested fix: document the canonical audit-action-detail path in `docs/skycot/audit-api.md` (or in `web/lib/api/audit-detail.ts` header).

---

## Resolved since last audit

The prior audit doc `2026-05-25-1100-scene-1-and-2-claim-1-and-2.md` exists in `docs/skycot/ux-audits/`. Without re-reading that file end-to-end this run, the following are confirmed to have shipped since then: surface-pin tests on team-headline (#2261); the `<WorkerPersonalityCard>` (file dated to 2026-05-19 design lock); the C2 holographic deck shine animation. The Schedules/Duties tab and Run-now gap is unchanged.

---

## Orientation score: 52 / 100

Reasoning:
- Visual chrome (C2 deck, gradient banner, personality card design) is locked, shipped, and beautiful — would score 80+ on a static-mockup-rendering audit.
- But two of the five things the operator asked me to verify (Schedules/Duties tab, Run-now, audit-row delegation chain) don't exist in the SPA at all.
- The live gateway has 1 of 5 workers + 0 of N expected audit rows + 0 duty endpoints, so the recording surface is unusable as-is.
- Claim 1 (Teammate, not tool) reads as colleague for Mariah only; degrades to generic config for everyone else.
- Claim 2 (Delegation is the unit) has zero UI surface. This is the demo-recording-day blocker.

The path from 52 → 85: ship the Duties tab + Run-now + duties endpoint (closes Scenes 2 entire); ship a DelegationChainCard (closes Claim 2); regenerate gateway seed data with 5 personas + headshots (closes Scene 1 deck); wire personality lookup to `worker.traits` (closes Claim 1 fan-out). Estimate 3–5 days of focused work, smaller if scoped as demo-only seed-and-render.
