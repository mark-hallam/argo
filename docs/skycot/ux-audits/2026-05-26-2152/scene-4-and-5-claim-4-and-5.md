# Scene 4 + Scene 5 (Claims 4 + 5 — Skills shared + learned + Any-PC MCP+CU) — capability UX audit

**Date**: 2026-05-26 21:52 AEST (Melbourne local)
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 4 `/skills` + `/skills/<id>` + Scene 5 `/workflows` + `/workflows/build` + `/workflows/templates` + `/settings/connectors`)
**Method**: live SPA walk via Claude Preview MCP (Model Context Protocol — Anthropic's standard tool-integration channel) against the dev server on `localhost:3000` (the preview-managed instance with `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` and `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`) plus the gateway on `127.0.0.1:7400` (schema v83 FRESH); source code reads where the live page would not hold still long enough to inspect (a global ⌘K listener kept hijacking focus during deep interaction); direct `curl` against both ports to confirm route resolution and gateway-endpoint shapes. Prior audit at `docs/skycot/ux-audits/2026-05-26-2007/scene-4-and-5-claim-4-and-5.md` used as the delta baseline.

**Note on port**: operator prompt says dev server is on `:3002`, but the running preview-controlled dev process is on `:3000`. `curl` confirms both ports return identical SPA HTML; the `:3002` instance is a separate dev process without the gateway env vars (its CSP `connect-src` points at `127.0.0.1:8787`, not `7400`). All evaluation was performed against `:3000` because that is the instance that can actually call the gateway.

---

## TL;DR

Zero net forward progress on Scenes 4 + 5 since the 2026-05-26 20:07 audit. All four P0 blockers identified there are still live in source: the `customer_research_followup` MCP+CU combo workflow remains absent from `web/lib/sample-workflows.ts` (which is the truth-source in demo-bypass mode); the SEED_SKILLS catalog still does not include "competitor pricing scrape"; `/skills/<id>` still has zero Install affordance (`web/app/(app)/skills/[skillId]/page.tsx` is 122 lines and contains no `<button>` in `<main>`); and the `Run now` button on every fixture workflow is still `disabled={running || !isLive}` with `isLive` only set to `true` for gateway-sourced rows (in demo-bypass mode, no row is gateway-sourced). On top of that, the builder canvas `StepKind` enum at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` still has no `mcp_invoke` or `cu_action` kind, and the sidebar palette at `types.ts:236-286` lists `tool-browser` ("Browser action (Playwright)") but no MCP-server picker and no "Computer-use action (macOS)" entry. Newly surfaced this round: the `/settings/connectors` page DOES ship a real 29-connector MCP catalog (Airtable / Asana / GitHub / Gmail / HubSpot / Jira / Linear / Notion / Slack / etc., live-curl confirmed) — but it is NOT reachable from the workflow builder, so the operator cannot insert an MCP step from the canvas. And there is zero in-product UX for macOS Accessibility / Screen Recording permissions, which the operator memo flags as the only remaining operator-side blocker for Claim-5 LIVE — the SPA does not even mention these permissions, let alone deep-link System Preferences.

Net: 4 P0s carried + 1 newly classified P0 (the MCP catalog ⇋ builder gap is no longer cosmetic now that Pitch Agent is in v1.0 scope and Scene 5 narrates "operator picks an MCP server and a computer-use action") + 1 P1 newly observed (no macOS perms onboarding for Claim 5).

---

## Headline blockers (Scene 4 + Scene 5 unrecordable as scripted)

| # | Sev | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | `customer_research_followup` MCP+CU combo workflow not in `SAMPLE_WORKFLOW_SUMMARIES`; gateway DB empty + demo-mode short-circuits the gateway call entirely | Scene 5 — the canonical MCP+CU combination workflow is invisible on `/workflows` |
| F2 | P0 | "competitor pricing scrape" search beat still returns 0 results on `/skills` | Scene 4 step 2 narrated search beat |
| F3 | P0 | `/skills/<id>` still has no Install affordance — runtime path is on `/workers/<id>/skills` `[Try it]` two clicks away | Scene 4 step 3 "click Install" beat |
| F4 | P0 | `Run now` button on workflow detail is `disabled` for every fixture workflow in demo-bypass mode | Scene 5 — operator cannot click Run now on any SPA-visible workflow |
| F5 | P0 | Builder canvas has no MCP-step or computer-use-step palette entries; the 29-connector catalog at `/settings/connectors` is disconnected from the builder | Scene 5 "build from scratch" beat — operator cannot assemble the MCP + CU combo from canvas |
| F6 | P1 | No in-product UX for macOS Accessibility + Screen Recording perms anywhere in SPA (operator-side ship-blocker for Claim 5) | Future demo + post-record onboarding |

---

## All findings

### F1 [P0] — Scene 5 MCP+CU combo workflow `customer_research_followup` is still functionally orphaned

Unchanged from prior audit. The combo workflow exists as TOML in `crates/argo-demo-seed/src/workflow.rs` with three steps (two `skill_invoke` + one `tool_call` against `cu.run_macro` for Notes.app, vertical `marketplace`), but in demo-bypass mode the SPA fan-out at `web/lib/api/workflows.ts:133-140` short-circuits the gateway entirely:

```ts
export async function fetchAllWorkflows(): Promise<FetchAllWorkflowsResult> {
  if (isAuthBypassed()) {
    return { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-bypass" };
  }
  ...
}
```

Since `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` is set in `web/.claude/launch.json`'s `web` config (line 7), `isAuthBypassed()` returns `true`, the gateway call never fires, and the truth-source is `SAMPLE_WORKFLOW_SUMMARIES` (`web/lib/sample-workflows.ts:64-141`). That fixture contains 7 workflows across `sales / support / gtm-engineer / people-ops / finance` — no `marketplace`, no `customer_research_followup`, no `cu.run_macro` step, no `mcp_invoke`-kind step.

Even if the gateway path WERE hit (auth-bypass off, real seed run), `discoverVerticalIds()` (`web/lib/api/workflows.ts:125-131`) derives its fan-out list from `SAMPLE_WORKFLOW_SUMMARIES.vertical` only — the `marketplace` vertical is not in that set, so the per-vertical call to `/api/v1/verticals/marketplace/workflows` never fires either. This is a two-layer block.

- Reproduction:
  - `curl -s http://localhost:3000/workflows | grep -oE 'href="/workflows/[a-z_-]+/[a-z_-]+"' | sort -u` → 7 rows, none matches `customer_research_followup`
  - `grep -nE "customer_research|marketplace" web/lib/sample-workflows.ts` → 0 matches
  - `grep -nE "marketplace" web/lib/api/workflows.ts:125-131` → only appears as the function-name comment, not in the discovered vertical set
- File paths:
  - Seed (real, untouched): `~/code/argo/crates/argo-demo-seed/src/workflow.rs`
  - Demo-mode bypass: `~/code/argo/web/lib/api/workflows.ts:133-140`
  - Fixture (missing the combo row): `~/code/argo/web/lib/sample-workflows.ts:64-141`
- Suggested fix (per the no-theater rule + function-vs-dressing rule): add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` with vertical `marketplace` AND step kinds `skill_invoke` + `tool_call` (tool `cu.run_macro`). Per the inference rule at `web/lib/api/workflows.ts:371-389` this will automatically infer `substrate: "macos"`. That makes the card render with a macOS substrate chip — the Scene 5 voiceover's "any-PC execution" beat then has a visible anchor. NB: this IS theater the camera sees (a fixture rendering as real content); if operator wants no theater, the only alternative is to (a) flip auth-bypass off for the demo, (b) seed the gateway with `argo demo seed`, and (c) add `marketplace` to `discoverVerticalIds()`.

### F2 [P0] — "competitor pricing scrape" search still returns 0 results

Unchanged. The SPA's `SEED_SKILLS` at `web/lib/api/skills.ts:63-134` contains 6 hardcoded rows (`approval.route / invoice.extract / candidate.screen / deal.qualify / support.triage / lead.enrich`); none match "competitor pricing scrape". The gateway endpoint `/api/v1/skills` returns `{"skills":[],"total_count":0}` (verified by `curl`), so the SPA's fallback to SEED_SKILLS triggers — but the catalog itself has no row for the runbook's narrated search.

The skill DOES exist in `crates/argo-demo-seed/src/skills.rs:38` ("competitor pricing scrape") with two sibling rows, but the demo-seed crate is not run against the gateway and is not mirrored in the SPA fixture. So the runbook's "Search the marketplace for 'competitor pricing scrape' — 3 results land" beat cannot be recorded.

- Reproduction:
  - `curl -s "http://127.0.0.1:7400/api/v1/skills"` → `{"skills":[],"total_count":0}`
  - `grep -n "competitor pricing\|pricing.competitor\|market.scrape" web/lib/api/skills.ts` → 0 matches
  - Live `/skills` page typing "competitor pricing scrape" into the hero-search box yields the `state-panel` "No skills match …" empty state
- File path: `~/code/argo/web/lib/api/skills.ts:63-134`
- Suggested fix: add a `pricing.competitor_scrape` row to `SEED_SKILLS` matching the 3-row shape in `crates/argo-demo-seed/src/skills.rs:36-69`. Keep the hidden foreign-tenant sibling for cross-tenant verification once the gateway path is real.

### F3 [P0] — `/skills/<id>` still has no Install button — the runtime path is on `/workers/<id>/skills`

Unchanged. `~/code/argo/web/app/(app)/skills/[skillId]/page.tsx` is 122 lines (re-read verbatim this round); no `<button>`, no `onClick`, no install affordance. The page renders Provenance (TrustGradeChip), Workers-using list, adoption count, and a back link — and stops.

The runtime install path is real and end-to-end live: `POST /api/v1/workers/<id>/skills` at `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs:138-176`, wired in the SPA via `enableSkill()` (`web/lib/api/worker-skills.ts:181-188`) and bound to the `[Try it]` button on `web/components/skycot/worker/SkillsLibraryCard.tsx:75`. So Claim 4 IS functionally LIVE — `LayeredLookup::with_library` (`crates/argo-agent-runner/src/turn_helpers.rs:65`) plus the `[Try it]` install action is the operator-visible loop.

But the runbook's Scene 4 step 3 says "click Install on the verified result" on the marketplace detail. On `/skills/approval.route` there is nothing to click. The operator must remember which worker they want, navigate to `/workers/<id>/skills`, scroll to the Library section, and click the row's `[Try it]`. That's a 4-click detour on a beat the runbook narrates as a single click.

- Reproduction:
  - `curl -s http://localhost:3000/skills/approval.route | grep -oE '<button[^>]*>'` → no `<button>` tags inside the main panel
  - Verified live preview (this round): `/skills/approval.route` renders Provenance + Workers-using + back link only
- File paths:
  - Missing affordance: `~/code/argo/web/app/(app)/skills/[skillId]/page.tsx`
  - Wired-up runtime (different surface): `~/code/argo/web/lib/api/worker-skills.ts:181-192` (`enableSkill` + `installSkillToWorker` alias) + `~/code/argo/web/components/skycot/worker/SkillsLibraryCard.tsx:75`
- Suggested fix: add a `<InstallToWorker>` component on `/skills/<id>` — a worker-picker `<select>` (rendered from the same `useWorkers` hook the cohorts page uses) plus an Install primary button. On click, call `enableSkill(workerId, skillId)` and show a 3-line progress strip ("Verifying provenance…", "Installing skill…", "Available to <Worker>"). The endpoint and hook already exist; this is ~80 LOC of new UI + 10 LOC of wiring.

### F4 [P0] — `Run now` button is `disabled` for every fixture workflow

Unchanged. `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:105` sets `const isLive = source === "gateway"`. In demo-bypass mode, `fetchAllWorkflows()` always returns `source: "mock-bypass"` (see F1 above), so `isLive` is `false` for every workflow card the SPA renders. Line 256 reads `disabled={running || !isLive}` and the title attribute reads "Available once this workflow is published to the gateway". Every Run now button is disabled in demo mode.

The runbook's Scene 5 voiceover says "operator hits Run now and watches the audit chain populate". That click cannot land on any of the 7 fixture workflows.

- Reproduction:
  - `curl -sI http://localhost:3000/workflows/sales/morning_pipeline_digest` → 200
  - Live preview DOM check (this round, while it held still): `document.querySelector('[data-testid=wfd-run-workflow]').disabled` → `true`
- File path: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-105, 250-264`
- Suggested fix: (a) for demo-bypass mode, treat fixture workflows as `isLive=true` and route the Run now click to a recorded-execution playback (cheap, but theater the camera DOES drill into if Scene 5 later opens the audit feed — see the function-vs-dressing rule, this would be a violation); OR (b) flip `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=0` for the recording session, seed the gateway with `argo demo seed`, and let the real `POST /api/v1/workflow-runs` path fire. Option (b) is the no-theater path and aligns with the operator memo `feedback_no_theater_in_demo`.

### F5 [P0] — Builder canvas has no MCP / computer-use step kinds; the 29-connector MCP catalog is disconnected from the builder

Newly classified as P0 this round because Pitch Agent is now in v1.0 scope (per `project_track_p_v1_mvp_scope` memory + BUILD_STATUS.md Track P amendment) and Scene 5 + Track P P-02 narrate "operator drags an MCP server onto the canvas". The builder cannot express either step kind today.

Two layers:

1. **`StepKind` enum at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` is missing `mcp_invoke` and `cu_action`.** Available variants: `trigger | skill_invoke | approval | tool_call | notify | decision | data | loop | wait | sub_workflow | http_request | code`. The closest match is `tool_call`, which the builder palette labels as "Browser action (Playwright)" via the `tool-browser` palette item — there is no inspector form that lets the operator pick `cu.run_macro` and target `Notes.app` from a dropdown. They would have to know to type the tool name `cu.run_macro` into a free-text field that does not exist.

2. **There is a real MCP-connector catalog at `/settings/connectors`** (`web/app/(app)/settings/connectors/connector-catalog.ts` is 280 lines, 29 named connectors with `data-testid="connector-card-airtable"` etc., live-curl confirmed) but the builder cannot link to it. The Tools palette group at `types.ts:271-277` lists only `tool-http`, `tool-browser`, `tool-code` — no `tool-mcp-airtable`, no `tool-mcp-github`, no generic `mcp_invoke` entry that would pop a connector picker.

Net: the operator cannot author a workflow from scratch that uses any of the 29 MCP connectors the platform supports. The builder pretends MCP doesn't exist.

- Reproduction:
  - `grep -nE "mcp_invoke|cu_action" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/types.ts` → 0 matches
  - `grep -nE "connector|airtable|github" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/*.tsx` → 0 matches
  - `curl -s http://localhost:3000/settings/connectors | grep -oE 'connector-card-[a-z-]+' | wc -l` → 29
  - `grep -E "tool-mcp|mcp.*palette" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/types.ts` → 0 matches
- File paths:
  - Enum gap: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34`
  - Palette gap: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` (SIDEBAR_PALETTE_GROUPS)
  - Orphan catalog: `~/code/argo/web/app/(app)/settings/connectors/connector-catalog.ts` (29 connectors)
- Suggested fix: extend `StepKind` with `mcp_invoke` and `cu_action`. Add a 4th palette group "MCP connectors" that reads from `connector-catalog.ts` and renders one item per connector with an `mcp_invoke` step kind pre-populated. Add a "Computer-use action" entry under Tools with substrate auto-set to `macos`. Inspector forms: for MCP, pick `server / tool / args`; for CU, pick `app / action / target`. 2-3 day brief; touches inspector + palette + step-encoder + propose-toml emitter.

### F6 [P1] — No in-product UX for macOS Accessibility / Screen Recording permissions (Claim-5 operator-side blocker invisible in SPA)

Newly observed this round. The CLAUDE.md "Remaining for v1.0 launch" list explicitly names "(c) **operator-only**: grant macOS permissions on `argo` binary (Accessibility + Screen Recording) for claim-5" as the only operator-side unblock left. But the SPA does not surface this anywhere — `grep -rnE "Accessibility|Screen Recording|System Preferences|macOS perm|grant.*permission" web/app web/components` returns one false-positive in `Step1Land.tsx:20` (HTML accessibility, not macOS perms) and nothing else.

For a v1.0 demo / first-customer onboarding, this is the moment a real user gets stuck — they hire a worker, attach a computer-use step, hit Run, and get an opaque substrate error from the executor with no guidance to System Preferences. There is no permissions wizard, no deep-link to `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`, no preflight check. For the v1.0 POC demo MP4 this is dressing-side (operator pre-grants off-camera), so it is P1 not P0 — but for the first-customer install it is the highest-friction step in the product.

- Reproduction:
  - `grep -rnE "Accessibility|Screen Recording|System Preferences|macOS perm" web/app web/components` → 1 false-positive match (`Step1Land.tsx:20` is HTML a11y), no perms wizard
- File paths: nothing exists today — this is a gap, not a regression
- Suggested fix: add a one-page perms-preflight at `/onboarding/macos-perms` (or `/settings/computer-use`) that detects `argo-executor-computer-use-macos` permission state via the gateway, surfaces a deep-link to System Preferences for each missing grant, and shows a green check once each is granted. Gate the first computer-use workflow run on this check until all grants land.

### F7 [P1] — All visible workflow cards show "HEADLESS" substrate chip — Scene 5's "any-PC execution" beat has no on-camera proof

Unchanged from prior F6. The substrate inference at `web/lib/api/workflows.ts:371-389` correctly upgrades to `macos` when any step kind / name / description / tool string contains `cu_action`, `cu.`, `computer-use`, `macos`, `notes app`, etc. But none of the 7 fixture workflows in `web/lib/sample-workflows.ts` contains any of those strings — so every card collapses to `headless`. The macOS chip color (`#eef2ff` background, `#3730a3` text) defined in `web/components/skycot/workflow/SubstrateChip.tsx:25` never renders in demo mode.

Solved automatically by F1 (if `customer_research_followup` lands in `SAMPLE_WORKFLOW_SUMMARIES` with `cu.run_macro` step, inference returns `macos`).

- File path: `~/code/argo/web/lib/sample-workflows.ts` (no fixture surfaces any non-headless substrate)
- Suggested fix: piggyback on F1's fix. Once the combo workflow lands in the fixture, its substrate chip will read MACOS and Scene 5 has its visual anchor.

### F8 [P2] — `/skills` count pill cosmetic still reads "6 skills · 0 shown" on no-match

Unchanged. `web/app/(app)/skills/page.tsx:114` renders `{skills.length} skills · {filtered.length} shown` unconditionally — so the pill reads "6 skills · 0 shown" while the empty-state body below already says "No skills match …". Two redundant phrasings.

- File path: `~/code/argo/web/app/(app)/skills/page.tsx:113-114`
- Suggested fix: when `filtered.length === 0 && query.length > 0`, render `0 of {skills.length}` instead. ~5 LOC.

### F9 [P2] — `DemoBanner` is on-screen during recording unless operator dismisses it pre-clip-1

Unchanged. `web/components/skycot/DemoBanner.tsx:33` checks a localStorage flag (`skycot:demo-banner-dismissed`); once dismissed it stays dismissed within the browser profile. Real operator workaround exists, but it is a pre-record checklist item the runbook does not currently mention.

- File path: `~/code/argo/web/components/skycot/DemoBanner.tsx:7,33,49`
- Suggested fix: add a single line to `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist: "Dismiss the DemoBanner before clip-1 takes." Optionally hide the banner when `process.env.NEXT_PUBLIC_SKYCOT_RECORDING=1` is set.

### F10 [P3] — Workflow card author chip still does not distinguish worker / human / template-pack

Unchanged from prior F5. `web/lib/sample-workflows.ts:51` defines `lastEditor: string` — no `authorType: "worker" | "human" | "template_pack"`. `web/app/(app)/workflows/page.tsx:267` renders `by {wf.lastEditor}`, no icon, no chip.

Scene 5 narration "workflows can be edited by the AI teammate themself" has no visual anchor on the card. P3 because the camera does not drill into authorship — but it is the kind of polish that strengthens the recursive-self claim if it lands.

- File path: `~/code/argo/web/lib/sample-workflows.ts:51` + `~/code/argo/web/app/(app)/workflows/page.tsx:267`
- Suggested fix: extend the type with `authorType` (default `"worker"` since 7/7 fixtures are worker-authored) and render a 12px `<Bot />` icon next to the name. ~25 LOC.

---

## Resolved since prior audit (2026-05-26 20:07)

None of the 10 findings in the prior audit resolved this round. Source verification (this round, by `grep` + `Read`):

| Prior F# | Status | Evidence |
|---|---|---|
| F1 (combo workflow orphaned) | UNRESOLVED → carried as F1 | `grep -nE "customer_research\|marketplace" web/lib/sample-workflows.ts` → 0 |
| F2 (competitor pricing scrape search) | UNRESOLVED → carried as F2 | SEED_SKILLS unchanged, gateway empty |
| F3 (`/skills/<id>` no Install) | UNRESOLVED → carried as F3 | `skills/[skillId]/page.tsx` still 122 lines, no button |
| F4 (Run now disabled) | UNRESOLVED → carried as F4 | `disabled={running || !isLive}` unchanged at line 256 |
| F5 (author-chip type) | UNRESOLVED → carried as F10 (downgraded P1 → P3 — POC demo doesn't read this) | `lastEditor: string` unchanged |
| F6 (all-HEADLESS chips) | UNRESOLVED → carried as F7 | inference logic correct, no fixture has trigger strings |
| F7 (builder lacks MCP/CU step kinds) | UNRESOLVED → carried as F5 (upgraded P1 → P0 because Track P + connector-catalog gap) | StepKind enum unchanged |
| F8 (untitled redirect to onboarding) | INCONCLUSIVE this round | live preview was unstable due to ⌘K keyboard focus interception; could not re-confirm. Source path at `build/page.tsx:69,87` defines `NEW_WORKFLOW_SENTINEL = "untitled"` and the `isNewWorkflow` branch — when held still, the page DOES render the builder (palette + canvas observed). The redirect-to-onboarding behavior observed in the prior audit may have been resolved or may be intermittent. Not re-classifying without a clean repro. |
| F9 (count pill "6 skills · 0 shown") | UNRESOLVED → carried as F8 | `skills/page.tsx:113-114` unchanged |
| F10 (DemoBanner on-screen) | UNRESOLVED → carried as F9 | DemoBanner component unchanged |

Newly observed positives (not in prior audit):
- `/settings/connectors` ships 29 real MCP-connector cards with `data-testid="connector-card-<slug>"` (curl-confirmed). The MCP-connector catalog UX exists; it just is not wired to the workflow builder yet.
- `BuilderToolbar` exposes `Test run` / `Save as draft` / `Publish` buttons with `data-testid="builder-test-run / -save / -publish"` (`web/app/(app)/workflows/[verticalId]/[name]/build/BuilderToolbar.tsx:108-152`). So when the builder is reachable on a desktop viewport, dry-run / save / publish exist as concrete UI surfaces.

---

## Claim verdicts (refreshed)

- **Claim 4 (Skills shared + learned) — LIVE backend, BROKEN front-door affordance.** Runtime confirmed wired at `crates/argo-agent-runner/src/turn_helpers.rs:65` (`LayeredLookup::with_library`). The SPA's `[Try it]` button on `/workers/<id>/skills` Library section is the only install surface. The `/skills/<id>` marketplace surface — which the runbook narrates as the install moment — has zero install affordance. Scene 4 step 3 unrecordable on the surface the runbook names. Verdict: LIVE but theater-recordable only by changing the runbook to navigate to `/workers/<id>/skills`.
- **Claim 5 (Any-PC execution via MCP + CU) — LIKELY-LIVE substrate, NOT-EXPOSED in the SPA on multiple fronts.** Computer-use executor is real (`crates/argo-executor-computer-use-macos`). MCP connector pool is real (29 cards live at `/settings/connectors`). But the workflow builder cannot express either step kind, no fixture workflow contains either step kind, no card on `/workflows` displays a non-headless substrate chip, and no SPA page surfaces the macOS perms grant flow. The platform CAN do the thing; the SPA pretends it cannot.
- **MCP catalog ⇋ builder bridge** — does not exist. The connector catalog is a settings-page artefact, not a builder primitive. For the Scene 5 "operator picks an MCP server" beat to be real, the builder must be able to insert a connector-bound step.
- **macOS perms onboarding (Claim-5 last-mile)** — does not exist anywhere in the SPA. P1 for the POC demo (operator pre-grants off-camera), but the highest-friction step for first-customer install.

---

## Orientation score: 52 / 100

Down from 56/100 in the prior audit because of two findings I had to upgrade: (a) the MCP-connector-catalog ⇋ workflow-builder gap is no longer "polish for v1.1" given Track P is in v1.0 scope and Scene 5 now narrates MCP+CU directly (was prior P1 F7, now P0 F5), and (b) the macOS-perms onboarding gap is a meaningful new P1 even though the POC demo dodges it. The previous score gave the platform credit for having the substrates real; the gap I had not weighted is that none of those substrates are addressable from the workflow surface where Scene 5 demonstrates them. That gap is the difference between "Skycot supports MCP + computer-use" and "an operator can build a workflow that uses MCP + computer-use." Today only the first claim is defensible.

Per the function-vs-dressing rule: F1 (combo workflow orphaned), F2 (search returns 0), F3 (no install button), F4 (Run now disabled), and F5 (no MCP/CU step kinds) are all FUNCTION-side — they break the recording as scripted. F6 (no perms wizard) is dressing-side for the POC demo (operator pre-grants off-camera). F7 (HEADLESS chips) and F8/F9/F10 are polish. None of the 5 P0s are dressing.
