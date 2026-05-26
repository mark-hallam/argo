# UX audit — Scenes 1 + 2 (Claims 1 & 2: Teammate-not-tool + Delegation-is-the-unit)

**Audit timestamp**: 2026-05-27T04:15Z (Melbourne local: 2026-05-27 14:15 AEST)
**Auditor**: e2e-ux-journey-test-agent (retry after prior socket-error attempt)
**SPA**: http://127.0.0.1:3000 (running, serverId 1d7d725c)
**Gateway**: http://127.0.0.1:7400 (schema v83, uptime 14h, FRESH)
**Scope**: `/workers` deck → `/workers/{id}` (Personality / Schedules / Skills / Memory / Audit tabs), Run-now affordance, audit-row landing, delegation/handoff-chain visibility for Claim 2

**Methodology note**: the Claude Preview MCP wedged after the very first navigation attempt (`preview_eval` / `preview_snapshot` / `preview_screenshot` all timeout after 30s with no console logs). Pivoted to curl + static-source inspection of `web/app/(app)/...` and `web/lib/api/...` plus live gateway probes. This means I can ladder the **structural** findings (routes that don't exist, endpoints that 404, copy that's hardcoded) confidently; I cannot ladder **rendered-pixel** findings (hover states, animations, hydration jank). Save the latter for the next pass when the preview tool is healthy.

---

## TL;DR

11 findings: **5× P0, 4× P1, 2× P2**.

The headline is brutal but clean: **Scene 2 as scripted is not recordable today** because (a) there is no "Schedules" tab and no "Run now" button anywhere on the worker-detail surface — the runbook's "click Mariah → Schedules → 3 duties → Run now" path simply doesn't exist; (b) the gateway has zero workflows + only 1 worker (Maya) seeded, not the 5-persona deck (Mariah/Peta/Priya/Jamal/Nori) the runbook narrates; and (c) the SPA's data-source endpoint mismatch (`GET /api/v1/workers` 404 on gateway — only `/api/v1/team` is exposed for the list) means the real-backend path falls back to either an empty roster or the `SAMPLE_TEAM` mock depending on demo-mode flag.

Scene 1's structural surface (`/team` → click → `/workers/{id}/overview` with a Personality card) **does exist and would record** — the C2 holographic-card deck is implemented per the locked design, the worker detail has tabs, and the DelegationChainCard for Claim 2 audit drill-in is built. But the demo seed has to be run AND the runbook narration has to be edited to match what's actually there ("Activity" tab, not "Schedules"; Personality lives on Overview, not its own tab).

The single biggest unblock: **run `argo demo seed --tenant <demo>` against the live gateway, fix the `/api/v1/workers` vs `/api/v1/team` endpoint mismatch, and rewrite Scene 2 to drive from `/workflows/<vert>/<name>` "Run now" (which exists) instead of from a non-existent worker-Schedules tab**.

---

## Headline blockers

| # | Sev | Finding | Scene impact |
|---|-----|---------|--------------|
| F1 | P0 | `/workers/{id}` has no Schedules/Duties tab | Scene 2 entire opening beat |
| F2 | P0 | No Run-now button on worker detail (only on workflows) | Scene 2 trigger beat |
| F3 | P0 | Gateway has only Maya seeded; 5-persona deck path unrun | Scene 1 deck render |
| F4 | P0 | SPA hits `/api/v1/workers` collection → 404; gateway only exposes `/api/v1/team` | Scene 1 cold-start |
| F5 | P0 | Audit feed has zero real worker-turn rows (only `audit.detail.viewed` / `audit.replay` from UI clicks) | Scene 2 audit landing |

---

## All findings

### F1 [P0] — No "Schedules" (or "Duties") tab on worker detail

The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`) Scene 2 reads "Mariah's Schedules tab shows 3 duties (cron-shaped). Click Run now → audit row lands in /audit within 5s." There is no Schedules tab. The actual tab list (`web/components/skycot/WorkerTabs.tsx:19-27`) is: Overview, Activity, Memory, Skills, Trust, Escalation, Settings. Activity is the closest — it shows past actions, not upcoming schedules. Duties only appear during the hire wizard (`web/components/skycot/hire/DutyToggleRow.tsx`); once a worker is hired there's no surface to see or edit their duty cron-schedule.

Reproduction:
- `curl http://127.0.0.1:3000/workers/<uuid>/schedules` → SPA returns the 404 layout (no such sub-route)
- Inspect `web/app/(app)/workers/[workerId]/` — only `overview / activity / escalation / evals / memory / responsibilities / settings / skills / trust` exist
- `grep "Schedule" web/components/skycot/WorkerTabs.tsx` → no match

File: `web/components/skycot/WorkerTabs.tsx:19-39` (WORKER_TAB_IDS + WORKER_TAB_LABELS)

Suggested fix: either (a) add a Schedules tab that calls a (yet-to-exist) `GET /api/v1/workers/<id>/schedules` endpoint + reuses the hire wizard's duty-cron renderer, or (b) rewrite Scene 2's narration to drive from `/workflows/<vert>/<name>` (where Run-now genuinely exists per `WorkflowDetail.test.tsx`).

### F2 [P0] — No "Run now" affordance on the worker surface

Scene 2 requires manually triggering a worker run from the worker detail page. No such button exists anywhere under `/workers/<id>/*`. Grep across `web/components/skycot/` for `Run now` / `RunNow` / `run-now` / `onRun` finds only `WorkflowDetail.test.tsx` (which asserts a Run-now button on the **workflow** detail) and `add-as-schedule-drawer.tsx` (which adds a workflow as a recurring schedule, not a manual trigger).

Reproduction:
- `rg -l 'Run\s*now|run-now|"Run"' web/app/\(app\)/workers/ web/components/skycot/` → no matches outside `WorkflowDetail.test.tsx`
- The WorkerOverflowMenu has `"disable-duties"` but no `"run-now"` action

File: `web/components/skycot/WorkerOverflowMenu.tsx` (overflow menu missing the action)

Suggested fix: add a "Run now" button (or overflow-menu item) on `/workers/<id>/overview` that POSTs to a new `/api/v1/workers/<id>/run` gateway endpoint and toasts the resulting action_id with a deep-link to `/audit?id=...`. **OR** rewrite Scene 2 to use `/workflows/<vert>/<name>` (where the button exists and tests confirm wiring).

### F3 [P0] — Gateway has 1 worker (Maya), not the 5-persona demo roster

`GET /api/v1/team` returns `{"total_count":1, "workers":[{"name":"Maya"…}]}`. The Scene 1 narration is built around the 5-card deck: Mariah / Peta / Priya / Jamal / Nori. The persona definitions exist in `crates/argo-demo-seed/src/personas.rs` (Mariah → marketplace, Peta → project-coordination, Priya → recruiting, Jamal → support, Nori → marketing) — the CLI just hasn't been executed against this gateway tenant. Maya has `headline: "No activity yet."` and ALL `permissions: false` (cannot edit charter / pause / view audit / etc.) — so even her detail page would record as a permissions-denied shell.

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/team | jq '.workers | length'` → 1
- `curl -s http://127.0.0.1:7400/api/v1/workers/<maya-id> | jq .permissions` → all false

File: `crates/argo-demo-seed/src/personas.rs` (seed source); operator-action gap

Suggested fix: run `cargo run -p argo-cli -- demo seed --tenant <demo-tenant-id>` (or whichever invocation the brief `fix-demo-seed-cli-operator-action-runbook.md` lays out). Verify post-seed roster reads 5 workers + each carries an audit-row trail.

### F4 [P0] — SPA fetches `/api/v1/workers` (collection); gateway only exposes `/api/v1/team`

`web/lib/api/team.ts:69-71` calls `apiGet("/api/v1/workers")`. Gateway returns 404. The fallback paths are: (a) if `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → return SAMPLE_TEAM immediately (never hits gateway); (b) if not bypass + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` → mock-offline path on 404 → SAMPLE_TEAM; (c) plain dev → empty roster + "unreachable" source chip. The launch.json in this repo sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (note the env-var name has NO `ARGO_` prefix) which the code does NOT read — the SPA still tries to hit `/api/v1/workers`, gets 404, and only the `SKYCOT_DEMO_MODE=1` flag (which IS set) gives us the SAMPLE_TEAM mock. So today the deck renders fixture data, not real backend data, even with the gateway up.

The team.ts comment says: "The gateway's `/api/v1/workers` is the canonical collection endpoint." Reality: it isn't.

Reproduction:
- `curl -s -w 'status:%{http_code}' http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"} status:404`
- `curl -s -w 'status:%{http_code}' http://127.0.0.1:7400/api/v1/team` → 200 with envelope

Files:
- `web/lib/api/team.ts:69-71` (wrong endpoint)
- `.claude/launch.json` (wrong env var name — `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` should be `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`, IF bypass is the intent)
- Gateway-side: whichever crate owns the team/workers HTTP route

Suggested fix: either (a) add a `/api/v1/workers` gateway alias that mirrors `/api/v1/team`, or (b) flip `web/lib/api/team.ts` to call `/api/v1/team`. The brief `fix-demo-seed-cli-for-real-roster.md` may already be tracking this — confirm before authoring a new one.

### F5 [P0] — Audit feed shows only meta-actions, no real worker turns

`GET /api/v1/audit?limit=20` returns 20 rows, ALL of which are `audit.detail.viewed` or `audit.replay` (someone clicking around the Audit UI). The `worker` field on every row is `{"id":"","name":"unknown"}`. There are no rows of action_type `worker.run` / `worker.delegate` / `skill.invoke` / `mcp.call`. So even if Scenes 2's Run-now button existed, the camera-drill-in into `/audit` would show nothing recognizable as a worker turn.

Per the function-vs-dressing rule (the operator-locked rule at `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`), the audit page is FUNCTION-side — the camera drills into it. Showing 20 rows of "Someone viewed an audit row" is theatre the camera reads as broken.

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/audit?limit=20 | jq '.items[].action_type' | sort -u` → only `audit.detail.viewed` + `audit.replay`
- `curl -s http://127.0.0.1:7400/api/v1/audit?limit=20 | jq '.items[].worker.id' | sort -u` → only `""`

File: gateway-side audit-write hook; the demo seed brief `fix-demo-seed-emit-worker-hired-audit-rows.md` is likely the right place to wire this up.

Suggested fix: after `argo demo seed` runs, the seed CLI should emit a small synthetic-but-real audit trail per persona (worker.hired + 2-3 worker.run rows + 1 delegation row) so the audit page lands with credible rows on cold open. Brief titled `fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona.md` (already exists per `ls streams/`) addresses this — confirm status.

### F6 [P1] — TeammateCard hardcodes traits/level for 6 names; Peta is missing

`web/components/skycot/TeammateCard.tsx:51-67` defines `TRAITS_BY_NAME` and `LEVEL_BY_NAME` with entries for Mariah / Priya / Jamal / Nori / Sam / Rio. **Peta is not present**, and Peta IS in the demo roster (project-coordination vertical, locked in memory `project_pm_cohorts_peta_patrick`). The fallback: Peta would render with `traits=[]` (no chip row) and `level=hashLevel("Peta")` — a stable but meaningless number. Visible to the camera in Scene 1 if it pans across the 5-card deck.

Reproduction:
- `grep -E '^\s+(Mariah|Peta|Priya|Jamal|Nori):' web/components/skycot/TeammateCard.tsx` → no Peta line
- Same problem in `web/components/skycot/worker/WorkerPersonalityCard.tsx` PERSONALITIES map (covers only Mariah/Priya/Jamal — missing Nori AND Peta)

File: `web/components/skycot/TeammateCard.tsx:51-67` + `web/components/skycot/worker/WorkerPersonalityCard.tsx` (PERSONALITIES map)

Suggested fix: add `Peta` + `Nori` entries to both maps (3-5 trait words each + a level). Cheap one-file change; high camera-impact if the deck is in shot.

### F7 [P1] — Worker `permissions` from gateway is all-false → operator can't edit/pause/run

`GET /api/v1/workers/<id>` returns `permissions: {can_archive:false, can_delete:false, can_edit_charter:false, can_edit_duties:false, can_pause_resume:false, can_view_audit:false, can_view_settings:false}`. So even after demo seed lands, every operator action on the worker detail page would be disabled. The camera narration "I'll pause Mariah" would land on a greyed button. This is upstream of any specific scene but blocks all worker-interaction beats.

Reproduction: `curl -s http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218 | jq .permissions`

File: gateway worker singleton handler (RBAC check / capability evaluation)

Suggested fix: confirm the auth-bypass / demo-mode path on the gateway grants all worker-scoped permissions to the seeded operator. Likely fixed by running with the correct demo identity, not a code change — but verify before recording.

### F8 [P1] — Activity-tab empty state hardcodes "she's"

`web/app/(app)/workers/[workerId]/activity/page.tsx:202-204` reads: *"{workerName} hasn't taken any actions yet. Once **she's** working, you'll see her activity here."* Jamal + (future) Patrick are male personas. If the camera lands on Jamal's empty Activity tab, the pronoun mismatch reads cheap.

Reproduction:
- Open `/workers/<jamal-uuid>/activity` (when seeded, with no activity yet) — the empty-state copy says "Once she's working…"

File: `web/app/(app)/workers/[workerId]/activity/page.tsx:200-205`

Suggested fix: replace `she's`/`her` with `they're`/`their` (gender-neutral); same patch likely needed in any other PERSONA-pronoun-assuming string the audit hasn't caught.

### F9 [P1] — `/workers` route is a redirect-to-`/team` (not the actual page)

`/workers` immediately `router.replace("/team")` if workers exist (`web/app/(app)/workers/page.tsx:25-27`). The runbook docs say Scene 1 lands on `/workers`. The URL bar will read `/team` for the entire deck-narration beat. Minor but a recording-script gotcha — the operator's voiceover should reference `/team` (or the page URL needs to stop redirecting).

Reproduction: `curl -sI http://127.0.0.1:3000/workers` (then load in a real browser to see the redirect)

File: `web/app/(app)/workers/page.tsx:25-27`

Suggested fix: either make `/workers` render the deck inline (currently it's an empty-state-only page), or update the runbook to land on `/team`. The latter is one-doc-line + zero code.

### F10 [P2] — "Personality" is a card on Overview, not its own tab (matches design lock, but contradicts runbook wording)

Per memory `project_worker_detail_design_locked` (C2 personality), the worker-detail design IS a LinkedIn-style profile with personality framing — but personality is realised as the `WorkerPersonalityCard` at the top of the Overview tab (`web/app/(app)/workers/[workerId]/overview/page.tsx:24-26`), not as its own tab named "Personality". The runbook says "all tabs: Personality, Schedules, Skills, Memory, Audit" — Personality is not a tab. If the camera narrates "let me click into her Personality tab" it would have to actually click Overview.

This is the cheapest fix in this audit: change the runbook line to "look at her Personality card on Overview" — no code change.

File: doc-only: `docs/skycot/demos/v1-mission-mvp-runbook.md`

### F11 [P2] — DelegationChainCard exists but doesn't surface in cold-open audit feed

`web/components/skycot/audit/DelegationChainCard.tsx` is well-built — shows "Who handed this work to whom" with an ordered chain + arrows. Claim 2's UI is real. But it only renders when an audit row carries a populated `delegation_chain`. Since the current audit feed is meta-actions only (F5), this card never appears in practice. So while Claim 2 has a UI surface that proves the platform thinks delegation is a first-class concept, the operator/investor would have to drill into a synthetic seeded delegation row to see it.

Tied to F5 — fixing the demo seed to fire one real delegation per persona-pair unblocks both findings.

File: `web/components/skycot/audit/DelegationChainCard.tsx` (works; just needs data)

---

## Resolved since last audit

N/A — first audit for this scope at this timestamp.

---

## Orientation score

**62 / 100**

What I observed: the **structural design** is in good shape — C2 holographic deck is built, worker-tab layout matches the locked design, DelegationChainCard for Claim 2 is built, the Audit page exists with real backend rows. The **plumbing and seed** are broken — wrong gateway endpoint name for collection, demo seed unrun, no Schedules tab, no Run-now button on workers, audit feed dominated by UI-meta rows. The 5 P0s are mostly **one-day fixes each**: rerun the seed CLI, fix one URL in team.ts, add a Schedules sub-route + a Run-now button + a `POST /api/v1/workers/<id>/run` endpoint, and emit synthetic-but-real audit rows from the seed. None of them are architectural rewrites.

The score would be 80+ if (a) demo seed actually ran against the live gateway, (b) audit feed had ≥10 real worker.* rows the camera could drill into, and (c) Scene 2 had a recordable trigger path (either via a worker-Run-now button OR via the `/workflows` Run-now button with runbook rewritten to match).
