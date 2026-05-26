# Scene 1+2 (Claims 1+2) UX audit — 2026-05-26 23:09 AEDT

Scope: `/workers` redirect → `/team` deck of 5 → `/workers/<id>` tabs (Overview / Activity / Memory / Schedules / Audit / Skills) → Run-now path → audit-row drill-in. Tested against the v1.0 demo runbook (Scenes 1+2) + the locked C2 trading-card / C2 personality designs + the operator-locked "Schedule" vocab + the function-vs-dressing rule.

Backend: gateway live on `http://127.0.0.1:7400`, schema v83, `argo demo seed` has NOT been run on this instance (the gateway has 1 real worker named "Maya · marketing-intern", not the canonical 5).

---

## TL;DR

12 findings: **5 P0**, **4 P1**, **3 P2/P3**. The C2 personality design is implemented well (trait chips, great-at / escalate-when split, signature behaviors, "what she does" paragraph — all there for the 5 sample teammates) and the trading-card deck on `/team` looks the part. The hard blockers are all about **function-side theatre that the camera DRILLS INTO**:

1. The 5 canonical teammates (Mariah / Peta / Priya / Jamal / Nori) **only exist as a TypeScript fixture** (`web/lib/sample-team.ts`). The gateway endpoint `/api/v1/workers` returns **404**; the SPA falls back to `SAMPLE_TEAM` and chips the page `MOCK-OFFLINE`. The 5 teammates are theatre, not real workers — and Scene 1 narrates them by name.
2. The gateway-real worker (Maya · marketing-intern) **crashes the worker detail page** because the gateway returns `charter: { text, version, last_edited_at, last_edited_by }` and the SPA expects `charter: string`. Click the only real worker → "500 — We couldn't load this worker." plus an infinite render loop in the console.
3. Audit-row drill-in is a **fake decision chain renderer**: clicking any real audit row in `/audit` lands on `/audit/action/<id>` which calls `placeholderAuditDetail()` and synthesises an 8-step "Decision chain" with fabricated provenance (`generic-skill v1.0 trust 92%`, `Top score 0.87; alternates 0.74 / 0.62`, fake model + prompt + memory + policy hashes) NEXT TO the heading "Action detail unavailable — drill into another row for full provenance." That is precisely the theatre the no-theater rule bans. **POC demo-blocker.**
4. There is **no Schedules tab** and **no per-duty Run-now button** anywhere on `/workers/<id>`. The closest surface is a card called **"Responsibilities"** on the Overview tab (vocab is "Schedule" per operator-lock; the on-page card title says "Responsibilities", "Add responsibility", "No responsibilities yet"). The "Run now" affordance only exists on `/workflows/<verticalId>/<name>` and it is **disabled** because `/api/v1/verticals` returns `{verticals: []}`.
5. The audit list `/audit` shows ~25 rows BUT every row is `http.request · audit.detail.viewed` or `http.replay` from the SPA's own self-fetches. Worker name on every row reads "UN" (unknown). There is zero agent-turn evidence in the audit timeline. Scene 6 (and the Scene 2 audit-row-landing affordance) will record "audit-of-the-audit" not "audit-of-real-work".

If the operator wants to record Scenes 1+2 today, the function-side blockers (above) compound: even the cards look good only because they are fixtures. Run `argo demo seed --tenant <demo>` on a fresh gateway, but then verify the worker-detail crash (finding F2) is fixed before pointing the camera at the real workers.

Function-vs-dressing applied: the "935 actions this week · saving you ~60 hours" hero numbers are dressing (workers.length × 187 / 12 — see `web/app/(app)/team/page.tsx:151-152`) — fine for camera. Persona personalities are dressing — fine. The drill-in theatre, the worker-detail crash on real data, and the empty backend behind 5 fake cards are function-side problems.

Orientation score: **42 / 100**. The design is mostly in place — the function under the design is mostly not.

---

## Headline blockers

| # | Sev | Finding | Demo scene impact |
|---|-----|---------|-------------------|
| F1 | P0 | Drill-in audit detail fabricates an 8-step decision chain with fake provenance hashes next to "detail unavailable" text | Scene 2 + Scene 6 (camera drills into a row) |
| F2 | P0 | The only real gateway worker (Maya) crashes the worker-detail page — `charter` is an object, SPA expects a string | Scene 1 (clicking the only real worker → 500 + render loop) |
| F3 | P0 | `/api/v1/workers` returns 404; `/team` falls back to SAMPLE_TEAM fixture for the 5 canonical workers | Scene 1 (5 teammates the runbook scripts by name are fictional, not real) |
| F4 | P0 | No Schedules tab or per-duty Run-now button anywhere on `/workers/<id>` | Scene 2 (the entire scripted Run-now → audit flow has no affordance) |
| F5 | P0 | `/audit` lists ~25 rows but every one is the SPA auditing its own page views; no agent-turn evidence | Scene 2 audit-row-landing beat + Scene 6 |

---

## All findings

### F1 [P0] — Audit-detail page renders a fabricated 8-step decision chain when gateway has no detail endpoint

Clicking any row in `/audit` navigates to `/audit/action/<id>`. The gateway responds 404 for `/api/v1/audit/action/<id>` (no single-action endpoint), so the SPA falls through to `placeholderAuditDetail(actionId)` in `web/lib/sample-audit-detail.ts:285-310`. That function renders:

- Verb: "Action detail unavailable — drill into another row for full provenance."
- AND THEN: an 8-step `decisionChain` ("Memory loaded · context (cached) · pricing rules" → "Skill: generic-skill v1.0 (trust 92%)" → "Plan selected by score · Top score 0.87; alternates 0.74 / 0.62" → "Policy check · Action permitted by policy" → "Approval · Autonomous — no approval required" → "Execution · Executor dispatched" → "Outcome · Completed successfully")
- AND a "Provenance & Chain" section with `model: claude-opus-4-7`, `promptTemplate: generic.placeholder / placehold00000000`, `skill: generic-skill 1.0.0`, `memorySnapshot: mem_placeholder`, `policy: generic-policy 1.0 / policyplaceholder`.

That's the exact pattern the no-theater rule bans (memory `feedback_no_theater_in_demo`): the camera CLICKS a real-looking audit row and the destination shows fabricated decision-chain content. The "detail unavailable" heading is read by an attentive viewer but the chain + provenance look completely plausible on first glance.

Reproduction:
1. `curl -fsS http://127.0.0.1:7400/api/v1/audit | jq '.items[0].id'` to get a real action id
2. Navigate `/audit/action/<that-id>` in the SPA
3. Observe the heading + fabricated chain rendering side by side

Files: `web/lib/sample-audit-detail.ts:285-310`, `web/app/(app)/audit/action/[actionId]/page.tsx`.

Suggested fix: when the placeholder branch fires, render ONLY the unavailability message + a back-to-audit CTA — drop the chain card and the provenance grid entirely. Keep DelegationChainCard suppressed (it already is via `length > 1`).

### F2 [P0] — Worker-detail page crashes for the only real gateway worker (charter shape mismatch + infinite render loop)

The gateway returns one real worker on `/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` (Maya · marketing-intern). When the SPA loads `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218`, the page renders the breadcrumb + hero (so the singleton fetch is working), then crashes the tab body with:

> `Error: Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version}). If you meant to render a collection of children, use an array instead.`

…followed by a "Maximum update depth exceeded" loop. End-state: the page shows "500 — We couldn't load this worker." but the breadcrumb + hero stay rendered, so it looks half-broken.

Root cause: the gateway sends `charter` as `{ text, version, last_edited_at, last_edited_by }` (see `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs:162-173`), while the SPA's `WorkerOverview.charter` is typed as `string` (see `web/lib/api/worker-overview.ts:61`). The renderer drops the object into a `<p>` directly.

Reproduction:
1. Visit `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` in the SPA with the demo gateway up
2. Watch the page render hero + crash; check console for both errors above

Files: `web/lib/api/worker-overview.ts:61` (string typing), `crates/argo-cli/src/handlers/gateway/routes_worker_overview.rs:162-173` (object shape), worker-overview consumer (likely WhatSheDoesCard).

Suggested fix: pick one side of the contract — either flatten gateway response to `charter.text` at fetch boundary, or update the SPA type + WhatSheDoesCard to read `charter.text` + `charter.last_edited_at`. The gateway shape is richer; the SPA should adapt. (Do NOT do this fix during this audit run — read-only.)

### F3 [P0] — `/api/v1/workers` returns 404; the canonical 5 are a TypeScript fixture, not a tenant seed

`curl http://127.0.0.1:7400/api/v1/workers` returns HTTP 404 `{"error":"not found"}`. The SPA's `fetchTeam()` in `web/lib/api/team.ts:58-99` catches the 404, checks `isDemoMode()`, and returns `SAMPLE_TEAM` (the 6-worker constant in `web/lib/sample-team.ts`) with `source: "mock-offline"`. The dev-only chip "MOCK-OFFLINE" next to the Team heading is the operator's only signal that the cards aren't real.

This means the demo recording — where Scene 1 calls out Mariah, Peta, Priya, Jamal and Nori by name and gestures at their KPI hero ("935 actions this week · saving you ~60 hours") — is showing fictional workers. Real-tenant data: the gateway has 1 worker named Maya in the `marketing-intern` cohort (via `/api/v1/team`, which is the still-live legacy endpoint).

Mitigation already documented in the runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md:51-69`): run `argo demo seed --tenant <demo>` after the gateway boots, which seeds the canonical 5. That step has not been run on this gateway. Without it, `/team` is theatre that the camera narrates.

Also: the hero KPI numbers ("935 actions this week · saving you ~60 hours") are derived as `workers.length * 187` and `workers.length * 12` (`web/app/(app)/team/page.tsx:151-152`). These are dressing per the function-vs-dressing rule — fine as fixture labels. But the underlying cards are function-side.

Reproduction:
1. Boot fresh gateway, do not run `argo demo seed`
2. Open `/team` — see 5 cards, MOCK-OFFLINE chip, hero "5 teammates · 935 actions / wk"
3. `curl -fsS http://127.0.0.1:7400/api/v1/workers` returns 404
4. `curl -fsS http://127.0.0.1:7400/api/v1/team` returns 1 real worker (Maya)

Files: `web/lib/api/team.ts:58-99`, `web/lib/sample-team.ts`, `crates/argo-cli/src/handlers/gateway/routes_workers.rs` (or equivalent — 404 means the collection route is missing).

Suggested fix: either (a) wire the gateway collection endpoint `/api/v1/workers` so it returns the demo-seeded 5 directly, OR (b) seed the 5 canonical worker ids before recording. The runbook already points at (b); the test pre-flight just hasn't been done on this gateway.

### F4 [P0] — No Schedules tab; no per-duty Run-now button; Scene 2's central affordance is missing

The locked C2-personality design and Scene 2 of the runbook both call for a "Schedules" tab (operator-locked vocab "Schedule" per memory `project_vocabulary_duty_vs_schedule`) showing 3 cron-shaped duties, each with a "Run now" button that fires a workflow execution and lands an audit row within 5s.

Actual state of `/workers/<id>`: the seven tabs are **Overview / Activity / Memory / Skills / Trust / Escalation / Settings**. No Schedules. No Duties. Inside the Overview tab there IS a card labelled **"Responsibilities"** (`web/components/skycot/worker/DutiesCard.tsx:89` — `<CardTitle>Responsibilities</CardTitle>`) that renders the duty list. But: it has no per-row Run-now button. Rows have toggle + label + schedule chip + edit-on-click only (`DutiesCard.tsx:138-197`).

Note that the underlying data model already uses the word "schedule" — `DutyToggle` aria-label is "Disable schedule" / "Enable schedule" (`DutiesCard.tsx:215`) — but the card title and add-button copy say "Responsibilities". Inconsistency on the same component.

The "Run now" button does exist but only on `/workflows/<verticalId>/<name>` (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:251-264`), and it is disabled when `source !== "gateway"`. With `/api/v1/verticals` returning `{verticals: []}`, the SPA's workflow list is empty, so even reaching that page requires a verticals seed.

Reproduction:
1. Open `/workers/018f67d0-0000-7000-8000-000000000001/overview` (Mariah, mock id)
2. Confirm the tab bar — Overview, Activity, Memory, Skills, Trust, Escalation, Settings (no Schedules)
3. Scroll into Responsibilities card — confirm card title is "Responsibilities", footer button is "Add responsibility"
4. Each row: toggle + label + schedule chip + click-to-edit. No Run-now.

Files: `web/app/(app)/workers/[workerId]/layout.tsx` (tab nav), `web/components/skycot/worker/DutiesCard.tsx:89,113-122,138-197`, `web/components/skycot/worker/WorkerOverviewContent.tsx:74-87`.

Suggested fix: (a) rename "Responsibilities" → "Schedules" in the card title + add-button + empty-state copy in `DutiesCard.tsx`; (b) add a per-row Run-now affordance that POSTs to `/api/v1/verticals/<verticalId>/workflows/<duty.workflow_name>/execute` (the existing executeWorkflow client). Both can ship without touching the gateway. Until then, Scene 2's central beat is unrecordable as scripted.

### F5 [P0] — `/audit` shows ~25 rows but every row is the SPA auditing itself; zero agent-turn evidence

`curl http://127.0.0.1:7400/api/v1/audit | jq '.items[].action_type' | sort -u` returns only two action_types: `audit.detail.viewed` and `audit.replay`. Every row has `worker: { id: "", name: "unknown" }`. The `verb` field is `http.request` on every row. So the demo gateway's audit timeline contains nothing but the SPA's own self-fetches.

When Scene 2 says "Run now → audit row lands in /audit within 5s" — the row would be another `http.request · audit.detail.viewed`. When Scene 6 says "click a row → real provenance" — the row contains no provenance because there's no agent work to provenance.

This compounds F3 (no real workers) + F4 (no Run-now button). Even if the operator hand-runs a workflow via curl, the audit shape has no `worker.name`.

Reproduction:
1. `curl -fsS http://127.0.0.1:7400/api/v1/audit | jq '[.items[] | .action_type] | unique'` → `["audit.detail.viewed", "audit.replay"]`
2. Open `/audit` in SPA → confirm every visible row shows `http.request` + `UN` (unknown worker)

Files: gateway audit-sink configuration — investigate why agent-turn audits aren't landing in the same table the SPA reads. (See `crates/argo-siem/` or `crates/argo-cli/src/handlers/gateway/routes_audit*.rs`.)

Suggested fix: run `argo demo seed` (per F3) — that path is supposed to fire enough Pass-A turns to populate the audit. Then verify agent-turn audits are visible. If not, gateway-side bug (audit sink may be filtering or tagging wrong tenant).

### F6 [P1] — Worker UUID renders in the browser tab title instead of the worker name

Visiting `/workers/018f67d0-0000-7000-8000-000000000001` produces a browser-tab title of `018f67d0-0000-7000-8000-000000000001 · Overview` (and identical aria-live alert content). The hero already knows the name is "Mariah" (it's rendered in the H1). The page title should match.

Cosmetic — but operator records demo on Chrome at 100% zoom; the tab title is camera-visible if the tab strip is in the recording crop.

Files: `web/app/(app)/workers/[workerId]/overview/page.tsx` (likely needs a `generateMetadata` that resolves worker name); same for `/activity`, `/memory`, etc.

Suggested fix: server-resolve worker name (or accept a small flicker as the client query lands) and set `document.title` to "<Name> · Overview".

### F7 [P1] — Persona drift between `/team` card and `/workers/<id>` Overview (Nori's traits don't match, Mariah's role swaps from "Marketplace" to "Operations · Procurement")

On `/team`, Nori's TRAITS are "Creative · On-brand · Concise · Curious". On the Nori detail page, the trait chips read "Reliable · Curious · Helpful". Different attributes, different counts. The locked C2 personality design promises trait chips that survive both surfaces.

Mariah is even worse: the `/team` card cohort badge says **MARKETPLACE**, but the worker hero on `/workers/<mariah-id>` says **Operations · Procurement**. Inconsistent persona / cohort metadata between the two surfaces.

Files: `web/lib/sample-personas.ts`, `web/lib/sample-team.ts`, `web/components/skycot/worker/WorkerPersonalityCard.tsx`. The `sample-personas.ts` file is meant to be the persona spine (per its header comment) — but the trait list on the C2-personality "HOW <NAME> WORKS" card is sourced from a different list than the card.

Reproduction:
1. `/team` — note Nori traits
2. `/workers/018f67d0-0000-7000-8000-000000000007/overview` — note Nori traits
3. Diff them.

Suggested fix: single-source the trait list. Card and personality card both pull from `sample-personas.ts`.

### F8 [P1] — `/workers` is a redirect shim to `/team`; the SKYCOT navigation has both items in the sidebar pointing at near-identical surfaces

The sidebar lists both "Team" and "Workers" as separate nav items. Clicking "Workers" sends a tenant with workers to `/workers` → `useEffect → router.replace('/team')` (`web/app/(app)/workers/page.tsx:25-26`). Empty-tenant case falls through to a "hire your first teammate" empty state.

So two side-nav rows point at the same destination. Confusing. The runbook narrates "navigate to /workers" but the URL bar will end up at `/team`. Camera-visible if the URL bar is in the crop.

Files: `web/app/(app)/workers/page.tsx`, `web/components/skycot/AppSidebar.tsx` (or wherever the nav is defined).

Suggested fix: pick one of (a) make /workers and /team distinct surfaces with different intents (workers = table view with filters per the file's TODO comment; team = current C2 card deck), or (b) remove one nav item until they diverge.

### F9 [P1] — Peta's `/team` card has no TRAITS section

Walking the 5 cards on `/team`: Mariah, Priya, Jamal, Nori each render a TRAITS line with 3-4 trait chips. Peta's card jumps straight from the header description ("keeps standups, action items, and status moving · Waiting on you") to the TODAY section — no TRAITS row. The locked C2 deck design has trait chips on every card.

Probably a data fixture gap: `getSamplePersona("peta-id").traits` is empty or unwired in `sample-team.ts`. Cosmetic on its own but the operator's running a fixture-perfect demo recording — a card visibly missing the trait band is camera-catch material.

Files: `web/lib/sample-personas.ts` (find Peta entry), `web/components/skycot/TeammateCard.tsx`.

### F10 [P2] — Worker-detail browser tab race / external navigation steering during the audit run

During this audit, multiple `window.location.assign('/team')` calls were silently overridden mid-tick to `/hire`, `/landing`, `/skills`, `/settings/connectors`. Either (a) another agent is concurrently navigating the same Chrome instance (the operator's IDE preview), or (b) the SPA has an unconditional redirect somewhere in the (app) shell that fires on certain queries. Could not reproduce reliably enough to localise; mentioning so the operator knows that if they see surprise nav during recording, it's not paranoia.

Files: investigate `web/app/(app)/page.tsx`, `web/app/(app)/layout.tsx` for any global redirect logic + check if a Chrome extension is auto-navigating.

### F11 [P2] — Mariah's Skills tab shows skills that don't match her marketplace cohort

Skills tab on Mariah (cohort = MARKETPLACE per /team) lists Active skills: `rfp-response-drafter v4.0.0` + `renewal-checklist-runner v3.1.2`. Library: `contract-redline-suggester v1.0.0`. These are professional-services / B2B-procurement skills — not e-commerce listing skills.

Persona-fixture drift: the Skills tab data set was authored before /team's marketplace cohort badge was finalised. Camera-visible if Scene 1 narrates "Mariah lists items for sale" and then drills into Skills.

Files: `web/lib/sample-worker-skills.ts` (or equivalent — find the per-worker active skills fixture).

### F12 [P3] — "Activity" tab is named "Activity" in the worker tablist but the underlying route is `/activity`; the worker file structure also has `/evals` (likely a leftover from before Activity tab was renamed)

Minor — `web/app/(app)/workers/[workerId]/evals/page.tsx` exists alongside `/activity/page.tsx`. Dead route or stale tab; check if anything links to it.

---

## Resolved since last audit

The last audit at `docs/skycot/ux-audits/2026-05-26-2309/` was empty (this is the first run). Prior audits at `2026-05-25-1100-scene-1-and-2-claim-1-and-2.md` and the iterative reruns in `2026-05-26-*/` should be the comparison baseline; not diffed in this pass.

---

## Orientation score

**42 / 100** — Visual design is largely in place (C2 personality + C2 trading-card decks both look like the locked designs), but the function under the design is theatre on every camera-visible drill-in (audit detail, real workers, run-now). Demo recording today would lean heavily on the fixture layer. With `argo demo seed` run and findings F1 + F2 + F4 fixed, the score should jump to ~75.
