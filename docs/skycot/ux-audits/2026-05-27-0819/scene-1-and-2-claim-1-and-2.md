# Scenes 1 + 2 — Claims 1 + 2 (Teammate-not-tool, Delegation-is-the-unit)

**Date:** 2026-05-27 08:19 AEST
**Scope:** `/workers` (canonical roster surface, redirects to `/team`) + `/workers/{id}` (all tabs) + Schedules/Duties/Responsibilities affordance + "Run now" + activity-row → audit-detail landing.
**Tester:** e2e-ux-journey-test-agent (preview MCP + curl against gateway).
**Lens:** function-vs-dressing per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.
**Prior baseline:** `docs/skycot/ux-audits/2026-05-26-2007/scene-1-and-2-claim-1-and-2.md` (10 findings). Significant deltas since: the duty-toggle "fake mutation + brief-ID toast leak" is now fixed (toggles render `disabled=true`, edit buttons disabled, no toast), and an `AddResponsibilityDialog` with cron presets is now mounted on the Overview tab.
**Servers used:** gateway `http://127.0.0.1:7400` (schema v83), Next dev on :3000 (env: `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`).

---

## TL;DR

**4 P0, 5 P1, 4 P2 — 13 findings total.** The good news: the Mariah `/workers/{id}/overview` page is the strongest single surface I drove in this session — the C2-personality content (HOW MARIAH WORKS / Great at / Escalate when / SIGNATURE BEHAVIORS) renders cleanly with 4 responsibility rows (cron-shaped: "as items arrive", "daily 06:30") and Disabled-state toggles that no longer fake a save. The AddResponsibilityDialog is real, with 5 cron presets and a textarea — that IS the "Scene 2 fire an agent turn" entry point under the new conversational-author UX, not a Run-now button on the worker profile. **The bad news: the front door is broken**. `/workers` redirects to `/team`, which calls `GET /api/v1/workers` → 404 from the gateway, and the React-Query loading skeleton never resolves (the SAMPLE_TEAM fallback contract exists in code but on this build the chip stays `data-source=loading` even after 60s+). So a real demo viewer landing on the SPA nav-bar "Workers" link sees infinite skeleton tiles, not the 5-card deck. Compounding it: the auth-bypass env var name mismatched (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` in env vs `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` in code) means the fast-path mock short-circuit is dead; everything goes through the 404 retry, which is also why /audit/action route compiles took **203s and the first-paint took 784s** on this machine.

**Audit drill-in is honest but theatrical.** Clicking the first activity row on Mariah's Activity tab does eventually land on `/audit/action/act_018f67d0-…_000`, but: (a) it shows "Action detail unavailable — Unknown worker" because the audit-detail mock doesn't reverse-lookup the action id to Mariah, (b) below that honest empty-state copy, the page STILL renders an 8-step generic decision chain ("generic-skill v1.0", "mem_placeholder", "claude-opus-4-7", "generic-policy") that contradicts the empty-state message, and (c) the redaction filter mangles the action id itself ("act_018f67d0-0000-7000-8000-000\[PHONE\]_000") because the 12-digit suffix trips the phone-number redactor. None of this is "real provenance the camera can pause on" — it's still fixture-shaped, and the camera that follows Scene 2's drill-in will see contradictory copy.

**No worker→worker delegation surface exists anywhere on the worker profile.** `DelegationChainCard` exists in code (`web/components/skycot/audit/DelegationChainCard.tsx`) but only renders in `/audit/action/[actionId]` when `chain.length > 1`, and since no real multi-worker turn has been fired, the card never appears in this session. Claim 2 ("delegation is the unit") has zero camera-readable surface today.

**One-line operator summary:** Mariah's profile page is recordable as a static product tour (the personality content, the disabled-but-honest schedules card, the Add-responsibility dialog all read well). What is NOT recordable today: the `/workers` landing into the 5-card deck (infinite skeleton), and Scene 2's "see one worker hand a task to another and watch it in audit" (no delegation chain anywhere). To unblock both with one fix: rename the env var in the SAMPLE_TEAM / SAMPLE_OVERVIEW fast-paths from `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` to the new `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS`, which would short-circuit the 404-retry chain everywhere AND make the 5 personas render instantly without needing the gateway to seed them.

---

## Headline blockers (top 4 P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | `/workers` redirects to `/team` which is **stuck on infinite loading skeleton**. The 5-card deck never renders for a real visitor. | Scene 1 step 1 unrecordable from the nav-bar "Workers" link. |
| F2 | P0 | Auth-bypass env-var name mismatch: code expects `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`; dev server has `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS`. Every fast-path mock branch is dead → every page hits the gateway, gets 404, and depends on a slow retry-and-fallback chain that didn't resolve in 60s on /team. | Cross-cuts every Scene. The whole "demo-mode = instant" promise is broken. |
| F3 | P0 | Activity-row drill-in lands on `/audit/action/{id}` showing "Action detail unavailable — Unknown worker" while ALSO rendering an 8-step generic decision chain + generic provenance hashes. The honest empty-state and the theatrical placeholder chain contradict each other. | Scene 2 step 3 ("drill into a real provenance row") shows theater the camera reads. |
| F4 | P0 | Action ID redaction false-positive: action id renders as `act_018f67d0-0000-7000-8000-000[PHONE]_000` because the 12-digit zero suffix trips the phone-number redactor. | Camera reads `[PHONE]` inside an action id on the audit-detail screen. Customer-visible bug. |

---

## All findings

### F1 [P0] — `/workers` → `/team` infinite skeleton; 5-card deck never appears

`/workers` (`web/app/(app)/workers/page.tsx:25-27`) calls `useTeam()` and `router.replace("/team")` when `hasWorkers === true`. `/team` uses `useTeam()` from `web/lib/hooks/use-team.ts` → `fetchTeam()` from `web/lib/api/team.ts:55-101`. In this session: `apiGet("/api/v1/workers")` returns 404 (gateway does not implement that endpoint), the catch block falls through to `isDemoMode() === true` → returns SAMPLE_TEAM. **But on the running dev server the team-data-source chip stays `data-source="loading"` indefinitely** (tested 60s+ with polling). 12 skeleton tiles render; 0 teammate cards render. I confirmed this independently by:

- `curl http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}` (404)
- `curl http://127.0.0.1:7400/api/v1/team` → 1 worker named "Maya" (not the canonical 5)
- Browser fetch of `/api/v1/workers` from console → 404
- `document.querySelector('[data-testid=team-data-source-chip]')` → `"loading"` after 30s wait

Hypothesis on root cause: the `useTeam` query's loading state isn't tied to the SAMPLE_TEAM-fallback resolving synchronously — there may be an unhandled rejection in the catch chain that React Query sees as "still pending". Plausible interaction with the SSR/client hydration mismatch reported in console (`data-active={undefined}` vs `data-active="true"` on the AppNav links — see F5).

Reproduction:
- Open `http://localhost:3000/workers`
- It redirects to `/team` (Next.js client-side `router.replace`)
- Inspect `[data-testid=team-data-source-chip]` — stays `"loading"`
- Inspect `[data-testid=teammate-loading-*]` — 6-12 skeleton tiles render forever
- `curl -m 60 http://localhost:3000/api/v1/workers` → 404 (proxy passes through)

Suggested fix: short-circuit the SAMPLE_TEAM path on the auth-bypass branch by fixing F2 first; that one-character env-var rename eliminates the network round-trip entirely. Alternative: seed the gateway with the 5 canonical personas via `argo demo seed`.

**Files:** `web/app/(app)/workers/page.tsx:19-46`; `web/app/(app)/team/page.tsx:42-186`; `web/lib/api/team.ts:42-101`; `web/lib/hooks/use-team.ts`.

---

### F2 [P0] — Auth-bypass env-var rename was forgotten in 5+ API client files

Dev server environment has `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` set (verified via `ps eww 83620`). Code in the API-client modules still reads the OLD name `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. Affected files (from `rg "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS" web/lib`):

- `web/lib/api/team.ts:50` — `isAuthBypassed()`
- `web/lib/api/worker-overview.ts:95` — `isAuthBypassed()`
- `web/lib/api/worker-activity.ts` — (referenced in fetchWorkerActivity short-circuit)
- `web/lib/api/workers.ts` — (referenced in pauseWorker/resumeWorker/hireWorker short-circuits)
- Likely also approvals, audit, audit-detail, replay, cohorts, skills, workflows clients

Concrete impact: the "fast-path mock short-circuit" branch (the one that returns SAMPLE_TEAM immediately without ever calling the gateway) is **dead code** in this environment. Every fetch round-trips to the gateway, gets a 404, and depends on the catch-block fallback. That round-trip is what makes /team look slow (and apparently never resolve, per F1) and what makes /audit/action route compile take 203 seconds (because the gateway 404 then triggers React-Query retry-with-backoff against a slow Next.js rewrite).

Reproduction:
- `ps eww $(pgrep -f 'next dev')` → shows `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`
- `rg "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS" web/lib` → 6+ matches
- `rg "NEXT_PUBLIC_GATEWAY_AUTH_BYPASS" web/lib` → 0 matches in production code paths
- Browser `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` inlines to undefined in the chunked bundle (verified by grepping `_73b952ca._.js`)

Suggested fix: pick one canonical name (operator's `launch.json` and `.env.local` use `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS`; the conventional rename direction is "drop the `_ARGO_` prefix as we move to SKYCOT"). Update all 5-8 client files in one PR; no code logic change, just a string replace.

**Files:** `web/lib/api/team.ts:50`; `web/lib/api/worker-overview.ts:95`; `web/lib/api/worker-activity.ts`; `web/lib/api/workers.ts`; plus the same pattern in approvals/audit/replay/skills/workflows clients (grep first).

---

### F3 [P0] — Audit drill-in: "Action detail unavailable" empty-state PLUS theatrical 8-step placeholder chain rendered together

Navigating `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000` (clicking the first activity row on Mariah's Activity tab) lands on a page that simultaneously says:

> Action detail unavailable — drill into another row for full provenance.
> Unknown worker
> Action detail unavailable — drill into another row for full provenance.

AND renders below it (in the same scroll viewport):

- ActionInputsCard: "MESSAGE / Detail payload not available for this action id." (honest)
- ProvenanceBlock with placeholder hashes: `MODEL claude-opus-4-7·20260430…`, `PROMPT TEMPLATE generic.placeholder·aced0000…`, `SKILL VERSION generic-skill·1.0.0…`, `MEMORY SNAPSHOT mem_placeholder·mem_plac…`, `POLICY VERSION generic-policy·10cacede…` (theatrical — these are not real hashes)
- DecisionChainCard with 8 generic stages: "Memory loaded → context (cached) · pricing rules / Skill: generic-skill v1.0 (trust 92%) / Plan generated / Plan selected by score / Policy check / Approval / Execution / Outcome" (theatrical — same generic chain that prior audits flagged)
- Output: "Detail for action act_018f67d0-0000-7000-8000-000\[PHONE\]_000 is not currently available" (honest copy, but see F4 for the `[PHONE]` leak)

The honest "Action detail unavailable" empty-state and the theatrical 8-step chain CANNOT both be correct. Either (a) detail is unavailable and the chain card shouldn't render, or (b) detail is available and the unavailable-message shouldn't render. Today both render and the camera reads contradictory copy.

Reproduction:
- Navigate to `/workers/018f67d0-0000-7000-8000-000000000001/activity`
- Wait for activity rows to load (mock fallback path takes ~10-15s)
- Click the first row (`act_018f67d0-…_000`)
- After cold compile (~200s) page renders with both empty-state copy AND generic 8-step chain

Suggested fix: gate `<ProvenanceBlock>` and `<DecisionChainCard>` rendering on whether `detail.provenance` and `detail.decisionChain` are real (non-placeholder) values. If the action-id can't resolve to a real audit row, render ONLY the empty-state — drop the generic chain entirely.

**Files:** `web/app/(app)/audit/action/[actionId]/page.tsx:138-156`; `web/components/skycot/audit/DecisionChainCard.tsx`; `web/components/skycot/audit/ProvenanceBlock.tsx`; `web/lib/api/audit-detail.ts` (the placeholder shape needs to flag "this is a fallback, do not render the chain").

---

### F4 [P0] — PII redactor mangles the action id itself: `act_018f67d0-0000-7000-8000-000[PHONE]_000`

On `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000`, the Output panel renders:

> Detail for action act_018f67d0-0000-7000-8000-000\[PHONE\]_000 is not currently available.

The redactor saw `000000000001` (12 digits) inside the action id and treated it as a phone number. **Action ids are not PII** — they're opaque tenant-scoped identifiers — and redacting them inside a copy-pasted reference string breaks the camera-visible promise that "the audit feed is the source of truth, you can copy this id and find it again." Also: the Hero, Inputs, and decision-chain don't apply the same redaction, so the SAME id renders differently in different blocks on the same page.

Reproduction:
- Open `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000`
- Search the body text for `[PHONE]` — it appears inside the Output section
- Compare to the same id rendered in the Hero ("act_018f67d0-0000-7000-8000-000000000001_000") — no redaction there

Suggested fix: either (a) exclude UUID-shaped tokens from the phone-redactor regex (UUIDs have `-` separators that no real phone has), or (b) only apply redaction to free-text fields (`output.text`, `inputs.message`), not to copy-id strings. The redactor library is in `web/lib/redaction/` or similar — grep for `[PHONE]` to find the replacement constant.

**Files:** `web/lib/redaction/` (grep `\[PHONE\]`); audit-detail rendering at `web/app/(app)/audit/action/[actionId]/page.tsx:138-156`.

---

### F5 [P1] — SSR/client hydration mismatch on the global nav (data-active drift)

Every navigation into an app-shell page logs a tree-hydration error to the browser console:

```
A tree hydrated but some attributes of the server rendered HTML didn't match the client properties.
…
<_c1 href="/team" active={false}>
+   data-active={undefined}
-   data-active="true"
…
<_c1 href="/audit" active={true}>
+   data-active={true}
-   data-active={null}
```

The AppNav component renders the active-tab marker based on `usePathname()`, but SSR (server-rendered HTML) computes it against the cached previous pathname (e.g. /audit) while the client computes it against the current pathname (e.g. /team). React's hydration log warns this "won't be patched up" — i.e. the SSR-rendered active-tab will be visually incorrect for the first paint until the client re-renders. On slow networks the camera could catch this — wrong tab highlighted for 100-300ms.

The "Audit · SKYCOT" document title leaking to /team pages (observed earlier in this session) is the same root cause: server-rendered metadata gets stamped with the prior route's value before the client-side route resolves.

Reproduction:
- Open browser dev console (Ctrl+Shift+I in Chrome)
- Navigate between /team / /audit / /workers / /workers/{id}
- See "A tree hydrated but some attributes…" error block every nav

Suggested fix: ensure `AppNav` and the metadata <title> are pure functions of the current pathname only — no SSR-time stale path. If `usePathname()` is being read inside a Suspense boundary that already SSR-rendered with a stale value, push the active-state computation below the boundary or use a "use client" + useEffect pattern that doesn't ship an `active` prop in SSR.

**Files:** `web/components/AppNav.tsx` or equivalent; `web/app/(app)/layout.tsx` (metadata generation).

---

### F6 [P1] — Worker-detail document title leaks the raw UUID on cold nav

On `/workers/018f67d0-0000-7000-8000-000000000001/activity` the document title renders as:

> 018f67d0-0000-7000-8000-000000000001 · Activity

…not "Mariah · Activity". The metadata builder uses the route param (workerId) as the worker-name placeholder while the worker fetch is still pending. Once useWorker resolves, the title updates to "Mariah · Activity" — but the cold-nav window is several seconds in this build. A camera that pans to the browser tab title strip will catch the UUID first.

The Overview tab renders the title correctly as "Mariah · Overview" because the metadata builder there hits a cached worker name; the Activity tab apparently doesn't share that cache.

Reproduction:
- Cold-navigate to `/workers/018f67d0-0000-7000-8000-000000000001/activity` (clear all next.js cache first, or open in a fresh incognito)
- Check the browser tab text in the first 5-30s — it shows the raw UUID

Suggested fix: pre-fetch the worker in the layout's `generateMetadata` (server-side) so the title is the worker name from the first SSR paint, not the UUID fallback. Alternative: render a generic "Activity · SKYCOT" if the worker name isn't yet resolved.

**Files:** `web/app/(app)/workers/[workerId]/[tab]/page.tsx` or wherever the `generateMetadata` lives for sub-tabs.

---

### F7 [P1] — No worker → worker delegation surface on the worker profile (Claim 2 has no entry point)

Searched the worker profile for any UI that demonstrates "this worker handed a task to another worker":

```
rg 'handoff|delegation|delegate' web/components/skycot/worker/
```

returns only `TrustGradeCard.tsx` ("Progress: 5-step ladder + criteria list (delegated to …)") — copy, not a real chain visualization. `DelegationChainCard.tsx` exists in `web/components/skycot/audit/` and renders only inside `/audit/action/{actionId}` when `chain.length > 1`. Since no real multi-worker turn has been fired in this session, the card never appears.

Scene 2's narration ("Mariah notices the price drop, hands the buyer-chat to Jamal, you watch it in audit") has no surface to point at today. The talk-track requires either:

- (a) seeding the gateway with a pre-canned multi-worker audit row + a deterministic action-id the demo can link to, then narrating from that audit-detail page (camera sees the DelegationChainCard render);
- (b) building a worker-profile Activity row that visually attributes the next step to a different worker (worker chip → arrow → other worker chip), pulling from the same audit-detail data.

Reproduction:
- Drive the full Scene 1 + 2 path; observe that no part of the worker profile or its tabs references another worker by name or chip.

Suggested fix (cheap): seed a single Mariah→Jamal handoff audit row + an action-id the runbook hardcodes. Even one row is enough for the camera to pause on.

**Files:** gateway seed `crates/argo-demo-seed/`; SPA already has `DelegationChainCard` ready to render.

---

### F8 [P1] — Worker profile tab strip has no "Schedules" tab (vocab + structure mismatch with runbook)

`WORKER_TAB_IDS` at `web/components/skycot/WorkerTabs.tsx:19-27`:

```
overview · activity · memory · skills · trust · escalation · settings
```

The runbook Scene 2 narrates "click Schedules" but there is no Schedules tab. Today schedules live INSIDE the Overview tab (the `DutiesCard` titled "Responsibilities" with cron-shaped right-aligned chips like "as items arrive" and "daily 06:30"). Per the operator-locked vocab memory `project_vocabulary_duty_vs_schedule`: UI says "Schedule", code says "duty". The card title is currently "Responsibilities" — neither "Schedules" nor "Duties". The cadence chips use schedule-vocab ("daily 06:30") but the card heading uses responsibility-vocab. Pick one and stick to it.

For Scene 2 to be recordable as-scripted, either: (a) rewrite the runbook to say "click the Responsibilities card on Overview" (cheap), or (b) hoist the DutiesCard into its own Schedules tab in the tab strip + rename consistently.

Reproduction:
- Open `/workers/{id}/overview` — see Overview tab active
- Look at tab strip — 7 tabs, none named Schedules or Responsibilities
- Look at card heading inside Overview — "Responsibilities"
- Look at duty rows' right-side chips — "as items arrive", "daily 06:30" (schedule-vocab)

Suggested fix: pick one — and update the runbook to match.

**Files:** `web/components/skycot/WorkerTabs.tsx:19-39`; `web/components/skycot/worker/DutiesCard.tsx:101` (CardTitle); `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 2 narration.

---

### F9 [P1] — Cold-start dev compile time for `/audit/action/[actionId]` is 203s; first request takes 784s

From `preview_logs`:

```
Compiling /audit/action/[actionId] ... ✓ Compiled /audit/action/[actionId] in 203s
GET /audit/action/.../act_018f67d0-... 200 in 784813ms
```

13 minutes from click to paint. The audit-detail route appears to drag in a heavy dependency tree (PII redactor + decision-chain renderer + provenance block + evidence-exporter modal lazy-load). For a recorded demo, the operator must pre-warm this route by visiting it once before recording starts; otherwise the camera will catch a 3-13 minute blank state.

Reproduction:
- Restart dev server (`pnpm dev` in `web/`)
- Click any activity row on `/workers/{id}/activity` without pre-warming
- Watch dev-server logs — 200+ second compile, then 700+ second first response

Suggested fix: this is a function of Next.js dev compile and won't block a production build, but the recording runbook should explicitly add `/audit/action/<known-id>` to the pre-warm list. Today the runbook says pre-warm `/compare + /workflows/build` only.

**Files:** N/A (Next.js compile graph). `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist.

---

### F10 [P1] — `/api/v1/workers` returns 404; gateway only knows `/api/v1/team`

`fetchTeam()` calls `apiGet("/api/v1/workers")` (web/lib/api/team.ts:80) because the comment block at the top of the file says "the gateway's `/api/v1/workers` is the canonical collection endpoint." But:

```
curl http://127.0.0.1:7400/api/v1/workers
→ {"error":"not found"} (404)

curl http://127.0.0.1:7400/api/v1/team
→ {"total_count":1, "workers":[{"name":"Maya", ...}]} (200)
```

The migration from `/api/v1/team` (deprecated) → `/api/v1/workers` (canonical) was done on the SPA but not on the gateway. The fallback path saves us in mock-mode but fails in any real-tenant scenario.

Also note: gateway has exactly **1 worker named "Maya"** (a marketing-intern persona, leftover from a hire-wizard run). The canonical Scene 1 roster (Mariah/Peta/Priya/Jamal/Nori) lives only as SAMPLE_TEAM in the SPA bundle; the live gateway has no idea about them. For a real-data demo path the gateway needs `argo demo seed --tenant <demo>` with those 5 personas, or the SPA needs to stay in mock-only mode (requires F2 fix).

Suggested fix: implement `GET /api/v1/workers` in the gateway as an alias for `/api/v1/team` (5-line PR) OR roll back the SPA's call site to `/api/v1/team`. The alias is the cleaner forward path.

**Files:** gateway side (the `/api/v1/team` handler — likely `crates/argo-gateway/src/routes/team.rs` or similar).

---

### F11 [P2] — Demo banner is sticky-top but its dismiss button can be obscured by the worker freeze button

`DemoBanner` (`web/components/skycot/DemoBanner.tsx:42`) is `sticky top-0 z-30 … pointer-events-none` with an internal `pointer-events-auto` on the X dismiss button. The worker page's `FreezeButton` lives at `z-?` and renders below the banner. On narrow viewports the freeze button can shift up into the banner's y-band and intercept clicks meant for the banner X button.

Not a Scene-1/2 demo-blocker but it's a real UX paper cut for any signed-in user trying to dismiss the banner.

Suggested fix: bump the dismiss button to `z-40` explicitly or move the freeze button outside the banner's vertical column.

**Files:** `web/components/skycot/DemoBanner.tsx:42`; `web/app/(app)/workers/[workerId]/FreezeButton.tsx`.

---

### F12 [P2] — `duty-schedule-{id}` test-id is a plain SPAN, not a button or link (Scene 2 has no interactive schedule affordance)

Each duty row exposes 3 testids: `duty-toggle-d-list`, `duty-edit-d-list`, `duty-schedule-d-list`. The first two are buttons (currently disabled). The third is a plain `<span>` showing the cadence string ("as items arrive", "daily 06:30") — NOT interactive. So clicking on the cron-shaped text does nothing.

Scene 2's "click Schedules" presumes the schedule text IS a click target (e.g. opens a cadence-edit popover). Today it isn't.

Reproduction:
- Open `/workers/{id}/overview`
- Click on the "daily 06:30" text — nothing happens
- Inspect element — `<span>`, no role, no onClick

Suggested fix: either (a) wrap the schedule text in a button that opens a cadence-edit dialog, or (b) accept that schedules are non-interactive read-only labels for v1 and update the runbook narration to not click them.

**Files:** `web/components/skycot/worker/DutiesCard.tsx` (DutyRow component).

---

### F13 [P2] — Add Responsibility dialog opens via raw `.click()` but not via the preview MCP `preview_click` tool

A real user clicking "Add responsibility" opens the AddResponsibilityDialog correctly (verified). But MCP-driven testing (`preview_click [data-testid=duties-add]`) does not always fire the onClick handler. The dialog opened only when I bypassed the MCP layer and did `document.querySelector('[data-testid=duties-add]')?.click()` directly. This isn't a customer bug, just an automation-friendliness one: it suggests the click handler may be attached at a non-standard layer (maybe pointer-events-style) that PointerEvents don't fully simulate.

Won't catch on camera in the recorded demo (humans operate the mouse, not Playwright), but flagging for the next playwright sweep so it can pin the right event type.

**Files:** `web/components/skycot/worker/DutiesCard.tsx` handleAddClick.

---

## Resolved since 2026-05-26 audit

- ✅ **Duty toggle is no longer a fake-mutation stub.** The toggle now renders `disabled=true` (per `canToggleDuty={false}` in `WorkerOverviewContent.tsx:145`). No `console.warn` brief-ID leak, no fake-save toast. Edit buttons (`duty-edit-*`) also disabled.
- ✅ **AddResponsibilityDialog is mounted.** Opens with a textarea + 5 cron presets (Every morning / Every hour / Weekdays 9am / Once a week / Custom) + Cancel/Continue. Heading is "Add a responsibility for Mariah".
- ✅ **`/workers/{id}/responsibilities/new` route exists.** This is the conversational author flow that the Add dialog navigates into after "Continue". Not a Scene-2 blocker but a real surface.
- ✅ **`/workers` no longer crashes.** It redirects to `/team` correctly. The follow-on infinite-skeleton (F1) is the new blocker.

---

## Orientation score: 38/100

Same headline number as the 2026-05-26 audit because the structural Scene-2 blockers are all still in place (no delegation surface, gateway has no real workers, audit feed has only HTTP meta-events), and a NEW blocker emerged: the team-page query never resolves, so Scene 1 step 1 is unrecordable from the nav-bar. The good news the +5 points worth of fixes that landed (duty toggle fix, dialog mount, responsibilities route, demo-banner pointer-events) are offset by the -5 points worth of regressions (auth-bypass env-var rename was forgotten in API clients, action-id PII redaction false-positive, `/audit/action` cold-compile time blowing out to 13 minutes, persistent contradictory empty-state + theatrical chain on drill-in).

**The one fix that unlocks the most**: rename `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` → `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` in 5-8 API-client files. That single rename activates the fast-path mock branches everywhere, makes /team render the 5 cards instantly (no gateway round-trip), makes /workers/{id}/{tab} cold-loads sub-second, and removes the slow-compile cascade.
