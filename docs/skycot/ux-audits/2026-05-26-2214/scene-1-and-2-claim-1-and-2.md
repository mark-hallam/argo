# Scenes 1 + 2 — Claims 1 + 2 (Teammate-not-tool, Delegation-is-the-unit)

**Date:** 2026-05-26 22:14 AEST
**Scope:** `/workers` deck → `/workers/{id}` (all 7 tabs) → Schedules/Duties affordance + "Run now" + audit-row landing.
**Tester:** e2e-ux-journey-test-agent (preview MCP attempted but wedged after the auth redirect; remainder of evidence via curl against gateway + read-only source inspection).
**Lens:** function-vs-dressing per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.
**Prior baseline:** `docs/skycot/ux-audits/2026-05-26-2007/scene-1-and-2-claim-1-and-2.md` (10 findings, 52/100). Deltas since: #2404 (onboarding demo-mode bypass), #2405 (audit drill-in graceful fallback), #2406 (team/approvals waiting badge consistency) — none touch the Scene-1/2 SPA surfaces.
**Servers used:** gateway `http://127.0.0.1:7400` (schema_version 83, uptime ~7.7h), Next dev on the preview-MCP port (auto-allocated). Demo-mode env vars active (`NEXT_PUBLIC_ARGO_DEMO_MODE=1`, `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`).

---

## TL;DR

**3 P0, 5 P1, 3 P2 — 11 findings total.** No SPA changes have landed on Scene-1/2 surfaces since the 2026-05-26 20:07 audit — the three structural Scene-2 blockers carry forward unchanged: there is still no Schedules tab on `/workers/{id}` (tabs strip is `overview · activity · memory · skills · trust · escalation · settings`), there is still no Run-now button anywhere on the worker profile (`rg 'Run now' web/app/(app)/workers/ web/components/skycot/worker/` returns zero), the three duty handlers in `WorkerOverviewContent.tsx:74-92` are still `console.warn` stubs announcing "ships in E4-h/E4-g", and the gateway still returns `{"count": 0, "items": []}` for every demo vertical's workflows so the fallback /workflows/[verticalId]/[name] Run-now button is rendered DISABLED with tooltip "Available once this workflow is published to the gateway." There is currently **no SPA path** that fires a real agent turn against the gateway from inside the worker-profile journey.

**One material new finding this tick (F4):** clicking any activity row on `/workers/{id}/activity` produces a `/audit/action/<sample-id>` route that resolves to a synthesized fixture audit detail via `synthesizeAuditDetail()` in `web/lib/sample-audit-detail.ts`. The drill-in renders content that LOOKS like an agent turn (populated `provenance.skill.id`, `provenance.model.id`, multi-stage `decisionChain`) but is purely fabricated. **This is camera-visible function-side theatre** — Scene 2 narrates "inside 60 seconds the audit chain has the proof row" while the chain on screen is made-up. Worse than an empty state (F3 covers the empty case from the prior audit), because the fixture is plausible enough that a viewer would believe it.

**One newly-promoted P1 (F11):** the `TeamDataSourceChip` next to the Team header is gated only by `process.env.NODE_ENV !== "production"` (`web/app/(app)/team/page.tsx:319`). Recording with `pnpm dev` (the documented runbook command) will paint "MOCK-BYPASS" on camera next to "Team · 5 teammates working". I observed this on the first preview session this audit walked.

---

## Headline blockers (top 3 P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | No Schedules tab; no Run-now button anywhere on `/workers/{id}`; duty toggle is still `console.warn("[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h")`. | Scene 2 step 1 "click Schedules" impossible from the worker profile; step 2 "Run-now" impossible. |
| F2 | P0 | Gateway `/api/v1/verticals/{v}/workflows` returns count=0 for all 6 demo verticals → the only SPA Run-now button (on `/workflows/[verticalId]/[name]`) is hard-disabled. | Scene 2 fallback path also dead. No SPA way to fire a real agent turn. |
| F4 | P0 | Activity-tab row clicks resolve to `synthesizeAuditDetail()` placeholder content with fabricated `provenance.skill.id` + populated `decisionChain` stages — looks like real provenance, isn't. | Scene 2 talk-track "inside 60 seconds the audit chain has the proof row" — chain on screen would be theatre, function-side, camera-visible. |

---

## All findings

### F1 [P0] — No Schedules tab + no Run-now affordance + duty handlers are still stubs (unchanged across multiple audits)

`WORKER_TAB_IDS` at `web/components/skycot/WorkerTabs.tsx:19-27` is:
```
overview · activity · memory · skills · trust · escalation · settings
```
No "schedules" segment. The `DutiesCard` is rendered inside the Overview tab via `WorkerOverviewContent.tsx:151-159` titled "Responsibilities" (the operator-vocab is correctly "schedule" per `project_vocabulary_duty_vs_schedule` memory — UI says schedule, code says duty).

The three duty handlers at `web/components/skycot/worker/WorkerOverviewContent.tsx:74-92` are still explicit stubs:
```ts
const onAddDuty = useCallback(() => setAnnouncement("Add responsibility dialog ships in E4-h"), []);
const onEditDuty = useCallback(() => setAnnouncement("Edit schedule ships in E4-h"), []);
const onSaveCharter = useCallback((next: string) => {
  setAnnouncement("Charter changes persist in E4-g");
  console.warn("[E4-b] charter save pending wiring (E4-g):", next);
}, []);
const onToggleDuty = useCallback((dutyId, enabled) => {
  console.warn(`[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h`);
}, []);
```

`rg 'Run now' web/app/(app)/workers/ web/components/skycot/worker/` returns ZERO matches — no Run-now button anywhere on the worker profile.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/overview` (Mariah)
- Inspect tab strip — 7 chips, no Schedules
- Click any duty toggle — console gets `[E4-b] duty 'd-list' toggled to false — mutation lands with E4-h`; no network call
- `rg 'Run now' web/app/(app)/workers/ web/components/skycot/worker/` — empty

Suggested fix: either (a) re-author the Scene 2 talk-track to navigate /workflows/[verticalId]/[name] and Run-now from there (cheapest), OR (b) ship a `schedules` tab with `Run now` per duty wired to `executeWorkflow`. Path (a) is currently blocked by F2.

**Files:** `web/components/skycot/WorkerTabs.tsx:19-39`; `web/components/skycot/worker/WorkerOverviewContent.tsx:74-92`; `web/components/skycot/worker/DutiesCard.tsx`.

---

### F2 [P0] — Gateway has zero workflows; the only Run-now button in the SPA is disabled (unchanged)

The /workflows/[verticalId]/[name] page has a real Run-now button wired to `executeWorkflow` → `POST /api/v1/verticals/{vid}/workflows/{name}/execute` (`web/lib/api/workflow-execute.ts:34-44`, button at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:254`). Disabling is computed from `isLive = source === "gateway"` (line 105): when the gateway list returns no matching workflow, `source` is the static-fixture path, `isLive` is false, button is disabled with tooltip "Available once this workflow is published to the gateway" (line 260).

Confirmed live this tick:
```bash
curl -sS -H "X-Argo-Principal: human:demo@acmeco.com" \
  "http://127.0.0.1:7400/api/v1/verticals/gtm-engineer/workflows"
# → {"count":0,"items":[],"vertical_id":"gtm-engineer"}
```
Same result for `people-ops`, `support`, `sales`, `finance`, `marketplace`. Zero published workflows across all 6 demo verticals.

Reproduction:
- `curl -sS -H "X-Argo-Principal: human:demo@acmeco.com" http://127.0.0.1:7400/api/v1/verticals/<v>/workflows` for each v → all empty
- Open `/workflows/gtm-engineer/post_listing` — `[data-testid="wfd-run-workflow"]` has `disabled=true`, tooltip = "Available once this workflow is published to the gateway"

Suggested fix: gateway-side — run the workflow-publish step of `argo demo seed` (look at `crates/argo-demo-seed` for the Pass-C step that registers `workflow_definition` per persona). Even one published workflow per demo vertical unblocks Scene 2 + Scene 5. SPA side: already correctly gated, no change needed.

**Files:** `crates/argo-demo-seed/`; SPA wiring at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:105`,`:256-260`.

---

### F3 [P0] — Audit feed: only HTTP meta-events; provenance chain is null (mostly unchanged; partially mitigated by #2366)

Gateway returns only `audit.detail.viewed` HTTP rows. Confirmed live this tick:
```bash
curl -sS -H "X-Argo-Principal: human:demo@acmeco.com" \
  "http://127.0.0.1:7400/api/v1/audit?limit=5"
# All 5 rows: action_type="audit.detail.viewed", worker={id:"",name:"unknown"},
#   verb="http.request", channel="web"
```

#2366 + #2405 mitigated the rendering side — the SPA now shows a graceful "Decision chain unavailable — This row records a meta-event, not an agent turn." empty state in `DecisionChainCard` instead of 8 empty stage rows or a hard 500. But the underlying data is still meta-events only; no agent-attributed rows have ever been written because no path fires a turn (F1 + F2).

Reproduction: same curl as above. Drill-in via `/api/v1/audit/<id>` returns `decision_chain: []`, `provenance.signature: null`.

Suggested fix: same root cause as F2 — publish workflow definitions + fire one execution per persona at seed time so there's at least one agent-attributed audit row with populated decision chain + provenance for the camera to drill into.

**Files:** `crates/argo-demo-seed/`; SPA side already correctly handling empty state.

---

### F4 [P0] — Activity-tab row click resolves to a FABRICATED audit detail (with synthesized provenance + decision chain) — function-side theatre the camera sees (NEW since prior audit)

`ActivityRow` at `web/components/skycot/ActivityRow.tsx:40` links each row to `/audit/action/<activity.id>`. The activity ids are built at `web/lib/sample-worker-activity.ts:185-186` as `act_${workerId}_${i.toString().padStart(3, "0")}` — for Mariah, that's `act_018f67d0-0000-7000-8000-000000000001_000`, `_001`, etc.

`fetchAuditDetail()` in `web/lib/api/audit-detail.ts:96-124` resolves the id via `findSampleAuditDetail`:
1. First checks the hand-shaped `SAMPLE_AUDIT_DETAILS` array — those use ids like `act_01J0MARIAH_001` so no match
2. Then checks `SAMPLE_AUDIT_ROWS` — those use the SAME `act_<workerId>_<i>` scheme (`web/lib/sample-audit.ts:134`) so MATCH found
3. Returns `synthesizeAuditDetail(row)` — which fills in `provenance.skill.id`, `provenance.model.id`, `decisionChain` stages, etc.

So a Scene-2 click into Mariah's most-recent activity row would land at `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000` and render a populated-looking provenance card with: model = `claude-opus-4-7`, skill = `recruiter-screen` (or whatever the synthesize seed-mapped), 8-stage decision chain. **All of it fabricated by the SPA fixture pipeline** — there is no real agent turn behind any of it.

Scene 2's talk-track says "inside 60 seconds the audit chain has the proof row" — if the operator narrates that line and drills into the activity tab's audit row, the camera reads fixture content as if it were the proof. Per the function-vs-dressing rule in `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` table: "audit rows the camera DRILLS INTO are from real agent turns" — function-side rule, NO theatre.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/activity`
- Click any visible row → URL becomes `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000`
- Render shows a populated decision chain + provenance fields — none of it came from a real agent turn

Suggested fix: same root cause as F2/F3 — seed a real workflow execution per persona so the activity tab carries gateway-real audit ids that resolve to gateway-real details. OR (cheaper) gate `findSampleAuditDetail`'s synthesized path behind a `NEXT_PUBLIC_ARGO_ALLOW_THEATRE_PROVENANCE` env so production / demo-recording defaults to the placeholder ("Action detail unavailable — drill into another row for full provenance.") rather than fabricated chains. Either way, the runbook should narrate which row to click and that row's content must be real.

**Files:** `web/lib/api/audit-detail.ts:96-124`; `web/lib/sample-audit-detail.ts:270-276`; `web/lib/sample-worker-activity.ts:185-186`; `web/components/skycot/ActivityRow.tsx:40`.

---

### F5 [P1] — `WorkerPersonalityCard` `PERSONALITIES` map missing BOTH Peta AND Nori — both fall through to DEFAULT_PERSONALITY (expansion of prior F4)

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` defines `PERSONALITIES` with entries ONLY for Mariah, Priya, Jamal. Both Peta (`018f67d0-...000007`) and Nori (`018f67d0-...000004`) — though listed in the canonical roster via `SAMPLE_PERSONAS` — fall through to `DEFAULT_PERSONALITY` at line 98-111 ("Reliable / Curious / Helpful" / "Working through the responsibilities you set...").

This is a worse expansion of the prior audit's F4: that one flagged Peta-only because she lacked entries in `TRAITS_BY_NAME` + `LEVEL_BY_NAME` (in `TeammateCard.tsx:51-67`). On the detail page Nori is ALSO degraded — even though her card on /team shows correct traits chips (Creative / On-brand / Concise / Curious).

Camera impact: on Scene 1, if the runbook clicks into Peta OR Nori, the personality card reads generic boilerplate. On Scene 2, if the duty narration drills into Peta or Nori, both render as placeholders with no signature behaviours, no escalate-when, no great-at content matching their /team card. Mariah (the canonical Scene-2 worker) is fine.

Also: residual `Sam` + `Rio` entries still sit in `TRAITS_BY_NAME` + `LEVEL_BY_NAME` (`web/components/skycot/TeammateCard.tsx:56-57, 65-66`) — leftover from the #2367 scrub. Dead code; could be deleted in the same PR that adds Peta + Nori.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000007/overview` (Peta) — body says "Reliable / Curious / Helpful"
- Open `/workers/018f67d0-0000-7000-8000-000000000004/overview` (Nori) — body says "Reliable / Curious / Helpful"
- Compare to `/workers/018f67d0-0000-7000-8000-000000000001/overview` (Mariah) — body says "Patient / Detail-oriented / Picture-perfect listings / Polite"
- `rg 'Peta|Nori' web/components/skycot/worker/WorkerPersonalityCard.tsx` → zero matches

Suggested fix: refactor `WorkerPersonalityCard`, `TeammateCard`'s `TRAITS_BY_NAME`, and `LEVEL_BY_NAME` to all consume `findSamplePersona(workerId).traits` etc. from `web/lib/sample-personas.ts` (single source of truth). The sibling-table sprawl is the bug pattern that produced this regression in the first place. Until that refactor: add `Peta` + `Nori` entries to `PERSONALITIES`, `TRAITS_BY_NAME`, `LEVEL_BY_NAME`, delete the `Sam` + `Rio` rows.

**Files:** `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96, 98-111`; `web/components/skycot/TeammateCard.tsx:51-67`; `web/lib/sample-personas.ts` (the source-of-truth target).

---

### F6 [P1] — Duplicate nav entries "Team" + "Workers" both resolve to the same surface (unchanged)

`web/components/skycot/nav-items.tsx:22-30` still ships both `/team` and `/workers` in `NAV_ITEMS`. `/workers` is a client-side redirect (`web/app/(app)/workers/page.tsx:25-26`: `if (hasWorkers) router.replace("/team")`). On a recorded demo the viewer sees two nav items stacked: "Team · Workers" — suggests two distinct screens. Click either → land on /team.

Reproduction: load the SPA, observe left nav (lines 23-30 of `nav-items.tsx`). Click "Workers" — URL flips to /team after a spinner beat (see F7).

Suggested fix: drop `/workers` from `NAV_ITEMS` (keep the route accessible from onboarding for empty-tenant first-touch). Or invert and make /workers canonical, redirect /team to /workers.

**Files:** `web/components/skycot/nav-items.tsx:22-30`.

---

### F7 [P1] — Spinner-then-redirect flash on /workers (unchanged)

`web/app/(app)/workers/page.tsx:25-35` renders a `Loader2` spinner first, the `useEffect` then checks team-data, then `router.replace("/team")`. Clicking the Workers nav item shows a brief spinner for ~200-800ms before the URL flips. On a recorded demo it reads as "this page is slow" rather than "this is a deliberate redirect."

Suggested fix: convert to a server component + `redirect('/team')`, or middleware rewrite. Removes the spinner. (Will pair naturally with F6's fix.)

**Files:** `web/app/(app)/workers/page.tsx:19-35`.

---

### F8 [P1] — Hardcoded gendered copy on the Overview tab — "What she does" / "What she works on" — for non-female workers too

`web/components/skycot/worker/WhatSheDoesCard.tsx:103` hardcodes `<CardTitle>What she does</CardTitle>`. Also the component filename embeds the bias (`WhatSheDoesCard.tsx`). This is rendered on every worker overview tab including Jamal — viewer would see "What she does" on Jamal's profile page, an inclusivity own-goal during an investor watch.

Also: Peta's overview body text appears to contain "Peta hasn't picked up Slack or email yet" / "Connect a channel so Peta can respond where your team already works" (per the prior audit) — gendered copy may or may not be operator-intentional but is currently implicit-by-name.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000003/overview` (Jamal) — Card title reads "What she does" while the body refers to him by name
- `rg "What she does" web/` returns the hardcoded title

Suggested fix: (a) rename to "What they do" + rename file to `WhatTheyDoCard.tsx` (lowest-friction inclusive default), OR (b) thread the worker's pronoun through `SamplePersona.pronouns` + render accordingly. Operator decision.

**Files:** `web/components/skycot/worker/WhatSheDoesCard.tsx:103` (file-rename candidate).

---

### F9 [P1] — `next-auth` SessionProvider polls `/api/auth/session` continuously and floods console with `ClientFetchError: There was a problem with the server configuration` errors (NEW — observed this audit)

On the first preview-MCP session (port 51153), with `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` set, the Next dev server's console showed dozens of `[error] ClientFetchError: There was a problem with the server configuration. Check the server logs for more information.. Read more at https://errors.authjs.dev#autherror` errors from `SessionProvider.useEffect [as _getSession]`. The errors come from `node_modules__pnpm_6d4507c7._.js:1019` (next-auth's `fetchData` → `getSession`).

The functional pages still render (the demo-mint bypass cookie path works), but the error spam fills the dev-server console + the browser DevTools console. If a viewer opens DevTools mid-demo to "look at the audit hash chain" or any other devtools-adjacent moment, they read 20+ red lines of authjs errors. Camera-visible if DevTools is open at any point.

Reproduction:
- Start `pnpm dev` with `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`
- Load any page (e.g. `/team`)
- Open DevTools → Console — see repeated `ClientFetchError: There was a problem with the server configuration`

Suggested fix: investigate why `auth()` is misconfigured in dev / bypass mode. Likely missing `NEXTAUTH_URL` or `NEXTAUTH_SECRET` env when bypass is on, but the SessionProvider still mounts and polls. Either (a) skip mounting `<SessionProvider>` when bypass is on, OR (b) export a `NEXTAUTH_SECRET` default in dev to silence the misconfigure path.

**Files:** likely `web/auth.config.ts` / `web/auth.ts` + the layout that mounts `<SessionProvider>`. (Prior audit's F7 notes the env-var name mismatch `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` vs `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` — may be the same root cause.)

---

### F10 [P1] — Demo banner copy ("Demo mode — showing sample data. Sign in to see your own workspace.") visible at top of /team (unchanged)

`web/components/skycot/DemoBanner.tsx` renders a full-width banner with text "Demo mode — showing sample data. Sign in to see your own workspace." + dismiss button (`data-testid="demo-banner-dismiss"`). Storage key `skycot:demo-banner-dismissed` persists dismissal — but re-mount on hard reload re-shows it.

Scene 1 narration ("here are your five teammates") fights the banner copy ("actually it's sample data") if the camera doesn't dismiss first. Pre-record checklist item per `v1-mission-mvp-runbook.md` should call out "Dismiss demo banner before take 1."

Suggested fix: either (a) add a runbook line "Click dismiss-demo-banner before recording," OR (b) auto-hide the banner after 2s in demo-mode, OR (c) replace the full-width banner with a corner pill so the camera doesn't read it.

**Files:** `web/components/skycot/DemoBanner.tsx`.

---

### F11 [P1] — `TeamDataSourceChip` paints "MOCK-BYPASS" next to "Team" header in dev — the runbook records with `pnpm dev` so the chip ships on camera (NEW)

`web/app/(app)/team/page.tsx:316-329`:
```ts
// Dev-only diagnostic chip; hidden in production so the demo MP4
// doesn't catch a stray chip.
function TeamDataSourceChip({ source }: { source: TeamDataSourceLabel }) {
  if (process.env.NODE_ENV === "production") return null;
  return (
    <span data-testid="team-data-source-chip" data-source={source}
      className="rounded-full border ... px-2 py-0.5 text-[10px] uppercase ...">
      {source}
    </span>
  );
}
```

The gate is `NODE_ENV === "production"`. The `v1-mission-mvp-runbook.md` pre-flight checklist runs `pnpm dev` (or `pnpm --dir web dev` per `.claude/launch.json`), which sets `NODE_ENV=development`, so the chip RENDERS during recording. I observed this on the first preview session this audit walked — clearly visible as "MOCK-BYPASS" next to "Team · 5 teammates working" on the first /team screenshot.

Camera impact: viewer sees a dev-mode internal status pill. Worse, the label "MOCK-BYPASS" telegraphs that the data is mock — directly undermining Scene 1 narration "here are your five teammates."

Reproduction:
- Start `pnpm dev` (dev NODE_ENV)
- Load `/team`
- Observe the chip rendered right after the "Team" header text

Suggested fix: either (a) gate the chip on a more conservative env var so demo-mode dev sessions don't render it (e.g. `process.env.NEXT_PUBLIC_SHOW_DEV_CHIPS === "1"`), OR (b) add to runbook pre-flight: "Set `SHOW_DEV_CHIPS=0` or run with `NODE_ENV=production` for recording." (a) is the safer default.

**Files:** `web/app/(app)/team/page.tsx:316-329`.

---

### F12 [P2] — Worker detail document.title shows the UUID, not the worker name (unchanged)

`/workers/018f67d0-0000-7000-8000-000000000001/overview` produces a `document.title` of `"018f67d0-0000-7000-8000-000000000001 · Overview"` (the H1 in-page is correct as "Mariah"). `web/app/(app)/workers/[workerId]/layout.tsx` does not export `metadata`, so the title falls through to the route-segment default.

Recording a screencast with browser chrome (tab bar) visible exposes the UUID. Per the runbook the operator uses a fresh Chrome profile + 1920×1080 — depending on whether tab bar is in frame, the UUID could appear on camera.

Reproduction: navigate to any worker overview tab, check `document.title` or the browser tab name.

Suggested fix: in `web/app/(app)/workers/[workerId]/layout.tsx`, add `export async function generateMetadata({ params })` that resolves the worker's name from the gateway / fixture and returns `{ title: "${name} · ${tab}" }`.

**Files:** `web/app/(app)/workers/[workerId]/layout.tsx`.

---

### F13 [P2] — Preview MCP browser wedges after a `/team`-then-navigate sequence; eval calls hang for 30s and the renderer becomes unresponsive (tooling note, observed this audit)

Browser sandbox repeatedly went into a stuck state mid-audit — `preview_eval('1+1')` timed out at 30s after a /team load. `preview_screenshot` failed with `UnknownVizError`. The dev-server logs showed continuous successful GET /'s but no /team / /workers routes — suggests the browser was still on /auth/signup but I had attempted to navigate to /team via `location.href =`.

Root cause is likely the `next-auth` `ClientFetchError` loop (see F9): SessionProvider polls + errors, the harness's stricter timeout fires before the page settles. Stopping + restarting the preview server gave a fresh sandbox once but ground to a halt again on the next navigation cycle.

Workaround for the operator (not a SKYCOT bug): use `pnpm dev` directly in a terminal + Chrome from the host machine for the demo recording — DO NOT rely on the preview MCP for the actual recording. The MCP is fine for spot-checks but not for sustained drive-the-page sessions.

**Files:** N/A — tooling, not product. F9's fix may indirectly help (silencing the SessionProvider error loop).

---

## Resolved since prior audit (2026-05-26-2007)

Cross-checked the prior 10 findings:

- **F3 (audit drill-in returns 0 stages + decision-chain unavailable empty state) — STILL TRUE for meta-event rows but mitigated by #2405 (`fix: graceful audit drill-in fallback in demo mode`).** The hard 500 boundary is gone; placeholders + synthesized fixtures now fill the gap. The downside is F4 (this audit) — synthesized fixtures cross the function-side theatre line.
- **F10 (preview MCP redirect flake) — STILL TRUE this audit (see F13 of this report).** Not a SKYCOT bug.
- All other prior findings (F1, F2, F4-Peta-traits, F5-duplicate-nav, F6-demo-banner, F7-spinner-flash, F8-UUID-title, F9-pronouns) — UNCHANGED. Re-stated with current line numbers + carried forward.

One regression / clarification this audit:
- **NEW F4 surfaces a new dimension of the audit-drill problem:** the click-from-activity-tab path produces synthesized "real-looking" content, not just empty state. This is a function-side theatre violation that wasn't caught by the prior audits because they focused on the /audit page entry, not the /workers/{id}/activity entry.

---

## Orientation score

**Scenes 1 + 2 ship-readiness: 50 / 100.** (Down from 52/100 at 20:07.)

The drop comes from F4 — newly-surfaced function-side theatre risk via the activity-tab → audit-detail path. F4 is the kind of finding that, if not caught, would land in the recording itself. The prior 52/100 assumed Scene 1 was camera-ready; that's still mostly true for the deck cards themselves, but the moment the camera drills into a worker's Activity tab → row → audit-detail, the synthesized provenance card violates the no-theatre-in-function rule.

Scene 1 still has the same camera-ready 5-card deck (good) modulo F5 (Peta + Nori personality cards degrade to defaults — worse than the prior audit which only flagged Peta), F8 (gendered card titles), F10 (demo banner), F11 (MOCK-BYPASS chip).

Scene 2 still has no recordable trigger surface (F1 + F2 unchanged). If the operator works around by clicking an existing activity row instead of firing a fresh one, F4 makes the drill-in theatrical. There is no path from /workers/{id} that lands on real provenance.

**Recommended sequence to unblock Scene 2 (same as prior audit, restated):**
1. Run `argo demo seed --tenant <demo-tenant-uuid>` and verify it publishes at least one `workflow_definition` per persona (currently it doesn't — gateway returns count=0 for all 6 verticals)
2. Fire one execution per persona at seed time so the audit feed carries at least one row with populated provenance per worker
3. Rewrite Scene 2 talk-track to navigate /workflows/[verticalId]/[name] (since the /workers/{id} Schedules tab still doesn't exist), pick a duty-shaped workflow, click Run-now
4. Verify the resulting audit row has populated `decision_chain`, `provenance.skill.id`, `provenance.signature`

Step (1)+(2) are gateway-side seed CLI work. Step (3) is a runbook edit. Step (4) is verification. **Until step (1)+(2) ship there is no camera-recordable Scene-2 path that respects the function-side no-theatre rule.**

**Additional independent fixes that materially improve Scene 1 quality:**
- F5 (10-line PR adding Peta + Nori to PERSONALITIES; better: refactor to consume sample-personas.ts)
- F11 (one-line change: gate the dev chip on an explicit env var instead of NODE_ENV)
- F10 (runbook addition: "Dismiss demo banner before take 1")
- F8 (one-line text change: "What she does" → "What they do")

---

## Answers to the critical questions

**Q1 — Claim 1 LIVE: does the worker feel like a teammate with persistent identity?**
**YES on Scene 1 modulo F5.** The 5-card deck shows the canonical roster with personality chips (4 of 5 cards). Each card name links to a detail page with charter / responsibilities / channels / KPIs / activity. The C2-holographic-showcase locked design is rendered. **Caveat: both Peta AND Nori's detail pages degrade to generic "Reliable / Curious / Helpful" boilerplate (F5)** — two of the five canonical workers read as placeholders on their own profile page, not colleagues. Mariah, Priya, Jamal are correctly personalised.

**Q2 — Claim 2 LIVE: does delegation actually invoke `delegate-task` with fallback-chain inheritance (PR #2256), or is the UI button a sonner-toast stub?**
**Backend wired, SPA NOT demonstrable.** The runtime IS wired per #2256 (`ToolContext.current_handoff_id` is plumbed; `delegate_task_inherits_active_inbound_handoff_chain` test exists). The SPA's workflow-execute path (`web/lib/api/workflow-execute.ts`) is correctly wired to the gateway. **But there is no live trigger surface that produces a delegation chain in the camera's view:** the worker-profile Run-now button does not exist (F1), the workflow-page Run-now button is disabled (F2, gateway has zero workflows), the duty toggle is a console.warn stub (F1), and if the operator falls back to clicking an Activity-tab row the drill-in renders SYNTHESIZED fixture provenance (F4) — function-side theatre. So while Claim 2 is "LIVE" per CLAUDE.md's backend-cold-test definition, it is **not demonstrable through the SPA without crossing the no-theatre line.**
