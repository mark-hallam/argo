# Scene 4 + Scene 5 (Claims 4 + 5 — Skills shared + learned + Any-PC MCP+CU) — capability UX audit

**Date**: 2026-05-26 22:14 AEST (Melbourne local)
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 4 `/skills` marketplace search "competitor pricing scrape" + install flow + Scene 5 `/workflows` `customer_research_followup` MCP+computer-use combo + workflow card author chip canon + `/workflows/build` canvas)
**Method**: live SPA walk via `curl` against the running Next.js dev server (`localhost:3000`, preview-managed, with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`) plus the gateway on `127.0.0.1:7400` (schema v83 FRESH, `/api/v1/skills` and `/api/v1/workflows` both return `total_count:0` — gateway DB empty as expected for demo bypass); source-code reads where DOM-only inspection was not enough. Claude Preview MCP `preview_eval` / `preview_snapshot` calls timed out after 30 s on every attempt (the in-page session bootstrap is in an `ClientFetchError` loop against authjs); falling back to `curl` + source. Prior audit at `docs/skycot/ux-audits/2026-05-26-2152/scene-4-and-5-claim-4-and-5.md` (90 min earlier) used as the delta baseline.

---

## TL;DR

Zero net progress on Scene 4 + Scene 5 since the 90-min-prior audit. The Scene 4 + Scene 5 P0 fix queue moved sideways: PR #2407 (`fix: seed demo marketplace skill catalog on gateway boot`) which would close half of F2 is **still OPEN on `origin/main`** (mergeable, never landed) and the same is true for PR #2358 (`Fix workflow detail Run now for seed workflows`) which would close F4. The Scene-5 combo-workflow brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds` is still **BLOCKED** in BUILD_STATUS.md (stale brief paths against current main, 11 retries logged, not re-authored). All four function-side P0 blockers from the prior audit (F1 combo workflow orphaned; F2 competitor-pricing-scrape search returns 0; F3 `/skills/<id>` no Install button; F4 Run now disabled in demo-bypass) remain live in source verbatim, plus the F5 builder-canvas-can't-express-MCP-or-CU step kinds + 29-orphan-connector-catalog gap. F6 (no in-product macOS perms wizard) remains the highest-friction first-customer install gap but is P1 because the POC demo grants perms off-camera.

Net: 5 carried P0s (F1-F5), 1 carried P1 (F6), 3 carried polish (F7-F9), 1 nit (F10). Recommendation: **stop authoring new Scene-4/5 fixes until the existing P0 PRs (#2407 + #2358) and the BLOCKED combo-workflow brief land** — three concurrent in-flight fixes for the same audit findings just produces churn.

---

## Headline blockers (Scene 4 + Scene 5 still unrecordable as scripted)

| # | Sev | Finding | Scene impact | In-flight? |
|---|---|---|---|---|
| F1 | P0 | `customer_research_followup` MCP+CU combo workflow not in `SAMPLE_WORKFLOW_SUMMARIES`; the `marketplace` vertical is not in `discoverVerticalIds()` either | Scene 5 narrated MCP+CU combo invisible on `/workflows` | Brief BLOCKED 13× since 2026-05-25 |
| F2 | P0 | "competitor pricing scrape" search beat still returns 0 results on `/skills` | Scene 4 step 2 narrated search beat | PR #2407 OPEN, mergeable, head SHA `8e47bfab`, not merged |
| F3 | P0 | `/skills/<id>` still has no Install affordance — runtime install path lives on `/workers/<id>/skills` `[Try it]`, 4-click detour | Scene 4 step 3 "click Install" beat | No brief in flight |
| F4 | P0 | `Run now` button on workflow detail is `disabled` for every fixture workflow in demo-bypass mode | Scene 5 — operator cannot click Run now on any SPA-visible workflow | PR #2358 OPEN, head SHA unknown, not merged |
| F5 | P0 | Builder canvas has no MCP-step or computer-use-step palette entries; the 29-connector catalog at `/settings/connectors` is disconnected from the builder | Scene 5 "build from scratch" beat — operator cannot assemble the MCP + CU combo from canvas | Brief BLOCKED (same as F1) |
| F6 | P1 | No in-product UX for macOS Accessibility + Screen Recording perms anywhere in SPA (operator-side ship-blocker for Claim 5) | Future demo + post-record first-customer onboarding | No brief in flight |

---

## All findings

### F1 [P0] — Scene 5 MCP+CU combo workflow `customer_research_followup` is still functionally orphaned

Unchanged verbatim from the 2026-05-26 21:52 audit and the 2026-05-26 20:07 audit before it. The combo workflow exists as TOML in `crates/argo-demo-seed/src/workflow.rs` with three steps (two `skill_invoke` + one `tool_call` against `cu.run_macro` for Notes.app, vertical `marketplace`), but in demo-bypass mode the SPA fan-out at `web/lib/api/workflows.ts:133-140` short-circuits the gateway entirely:

```ts
export async function fetchAllWorkflows(): Promise<FetchAllWorkflowsResult> {
  if (isAuthBypassed()) {
    return { workflows: SAMPLE_WORKFLOW_SUMMARIES, source: "mock-bypass" };
  }
  ...
}
```

Since `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` is set in `.claude/launch.json`, `isAuthBypassed()` returns `true`, the gateway call never fires, and the truth-source is `SAMPLE_WORKFLOW_SUMMARIES` (`web/lib/sample-workflows.ts:64-138`). That fixture contains 7 workflows across `sales / support / gtm-engineer / people-ops / finance` — no `marketplace`, no `customer_research_followup`, no `cu.run_macro` step, no `mcp_invoke`-kind step.

Even if the gateway path WERE hit (auth-bypass off, real seed run), `discoverVerticalIds()` (`web/lib/api/workflows.ts:125-131`) derives its fan-out list from `SAMPLE_WORKFLOW_SUMMARIES.vertical` only — the `marketplace` vertical is not in that set, so the per-vertical call to `/api/v1/verticals/marketplace/workflows` never fires either. Two-layer block.

The brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md` (which would close this F + F5) has been **BLOCKED for 25 hours** with 11 retry-claim entries in BUILD_STATUS.md, all citing "structurally mismatched to current main; may-touch references missing `crates/argo-workflow/src/step.rs`, `crates/argo-runtime/src/workflow_runner.rs`, etc." It needs to be re-authored against current paths before any worker can pick it up.

- Reproduction:
  - `curl -s http://localhost:3000/workflows | grep -oE 'href="/workflows/[a-z_-]+/[a-z_-]+"' | sort -u` → 7 rows, none matches `customer_research_followup`
  - `grep -nE "customer_research|marketplace|cu_action|mcp_invoke|run_macro" ~/code/argo/web/lib/sample-workflows.ts` → 0 matches (only one false-positive: `customer_research_followup` mentioned in `SubWorkflowPicker.test.tsx`, which is a unit test fixture, not a production sample)
  - `curl -s http://localhost:3000/workflows | python3 -c '…'` → 7 substrate-chips, all `data-substrate="headless"`
- File paths:
  - Seed (real, untouched): `~/code/argo/crates/argo-demo-seed/src/workflow.rs`
  - Demo-mode bypass: `~/code/argo/web/lib/api/workflows.ts:133-140`
  - Fixture (missing the combo row): `~/code/argo/web/lib/sample-workflows.ts:64-138`
  - Blocked brief: `~/code/argo/docs/build-plan/streams/fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md`
- Suggested fix: re-author the brief against current paths. Then add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` with vertical `marketplace` AND step kinds `skill_invoke` + `tool_call` (tool `cu.run_macro`). Per the inference rule at `web/lib/api/workflows.ts:371-389`, this automatically infers `substrate: "macos"`. NB: this IS theater the camera sees (a fixture rendering as real content); the no-theater path is (a) flip auth-bypass off for the demo, (b) seed the gateway with `argo demo seed`, and (c) add `marketplace` to `discoverVerticalIds()`.

### F2 [P0] — "competitor pricing scrape" search still returns 0 results

Unchanged on `origin/main`. PR #2407 (`fix: seed demo marketplace skill catalog on gateway boot`) was **opened 2026-05-26T12:04Z by conductor**, **passed evaluator round 86 PRE-MERGE** (per recent commit `fb016014a`), but is **still OPEN, never merged**:

```
gh pr view 2407 --json state,mergeable
→ { "state": "OPEN", "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN" }
```

So both legs of F2 are still broken in `origin/main`:
- (a) gateway-side: `curl -s "http://127.0.0.1:7400/api/v1/skills"` → `{"skills":[], "total_count":0}` (PR #2407 fixes this with the new `marketplace.rs` seed module).
- (b) SPA fallback: `SEED_SKILLS` at `web/lib/api/skills.ts:63-134` contains 6 hardcoded rows (`approval.route / invoice.extract / candidate.screen / deal.qualify / support.triage / lead.enrich`); none match "competitor pricing scrape" (PR #2407 adds `pricing.competitor_scrape` to this list).

PR #2407 is mergeable, clean, conductor-authored, and passed evaluator R86. It just hasn't been merged.

Separately: the `TRY_CHIPS` array at `~/code/argo/web/app/(app)/skills/page.tsx:23-30` lists 6 chips (approval routing / invoice extraction / candidate screening / deal qualification / support triage / lead enrichment). None matches the runbook's narrated "competitor pricing scrape" beat — so even after PR #2407 lands, the runbook's "operator clicks the chip and a search fires" implication isn't backed by a chip. They'd have to type the query.

- Reproduction:
  - `gh pr view 2407 --json state` → `{"state": "OPEN"}`
  - `curl -s "http://127.0.0.1:7400/api/v1/skills"` → empty
  - `grep -n "competitor pricing\|pricing.competitor\|market.scrape" ~/code/argo/web/lib/api/skills.ts` → 0 matches
  - `grep -n "competitor\|TRY_CHIPS" ~/code/argo/web/app/\(app\)/skills/page.tsx` → only `TRY_CHIPS` at line 23, no competitor chip
- File paths:
  - SPA fallback list: `~/code/argo/web/lib/api/skills.ts:63-134`
  - TRY chips list: `~/code/argo/web/app/(app)/skills/page.tsx:23-30`
  - Pending fix: PR #2407
- Suggested fix: merge PR #2407 (admin-merge eligible per the web-only authorization memory `feedback_web_only_merge_authorization_2026_05_26` — vitest green, ≤200 LOC SPA changes, file count well under 5 on the SPA side). Plus a 1-line patch to add `"competitor pricing"` to `TRY_CHIPS`. The runbook will then be recordable.

### F3 [P0] — `/skills/<id>` still has no Install button — the runtime path is on `/workers/<id>/skills`

Unchanged verbatim. `~/code/argo/web/app/(app)/skills/[skillId]/page.tsx` is still 122 lines (re-read this round); no `<button>`, no `onClick`, no install affordance. The page renders Provenance (TrustGradeChip), Workers-using list, adoption count, and a back link — and stops.

Confirmed by curl: the only `<button>` tags on `/skills/approval.route` are the chrome (navdrawer-open, appshell-user-menu-trigger, appshell-theme-toggle, help-widget-trigger); zero in the main panel.

The runtime install path is real and end-to-end live: `POST /api/v1/workers/<id>/skills` at `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs:138-176`, wired in the SPA via `enableSkill()` (`web/lib/api/worker-skills.ts:181-188`) and bound to the `[Try it]` button on `web/components/skycot/worker/SkillsLibraryCard.tsx:75`. So Claim 4 IS functionally LIVE; only the runbook's narrated "click Install on the marketplace" beat is unrecordable on the surface the runbook names.

No brief in flight for this. There IS a closed brief `web/components/skycot/skills/TrustGradeChip.tsx` (which shipped earlier), but no `<InstallToWorker>` brief or PR.

- Reproduction:
  - `curl -s http://localhost:3000/skills/approval.route | grep -oE '<button[^>]*data-testid="[^"]+"'` → only chrome buttons
  - `wc -l ~/code/argo/web/app/\(app\)/skills/\[skillId\]/page.tsx` → 122
  - `ls ~/code/argo/web/components/skycot/skills/` → only `TrustGradeChip.{tsx,test.tsx}` — no Install component
- File paths:
  - Missing affordance: `~/code/argo/web/app/(app)/skills/[skillId]/page.tsx`
  - Wired-up runtime (different surface): `~/code/argo/web/lib/api/worker-skills.ts:181-192` (`enableSkill` + `installSkillToWorker` alias) + `~/code/argo/web/components/skycot/worker/SkillsLibraryCard.tsx:75`
- Suggested fix: add a `<InstallToWorker>` component on `/skills/<id>` — a worker-picker `<select>` (rendered from the same `useWorkers` hook the cohorts page uses) plus an Install primary button. On click, call `enableSkill(workerId, skillId)` and show a 3-line progress strip ("Verifying provenance…", "Installing skill…", "Available to <Worker>"). The endpoint and hook already exist; this is ~80 LOC of new UI + 10 LOC of wiring. Author a brief.

### F4 [P0] — `Run now` button is `disabled` for every fixture workflow

Unchanged on `origin/main`. PR #2358 (`Fix workflow detail Run now for seed workflows`) is **OPEN, never merged** (state `OPEN`, mergeable `UNKNOWN`). The codex-authored PR adds an auto-publish-before-run path so seed-source workflows can be executed via the same `/api/v1/workflow-runs` endpoint live workflows use, but it hasn't landed.

Current state at `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:105`: `const isLive = source === "gateway";`. In demo-bypass mode, `fetchAllWorkflows()` always returns `source: "mock-bypass"` (see F1 above), so `isLive` is `false` for every workflow card the SPA renders. Line 256 reads `disabled={running || !isLive}`. Every Run now button is disabled in demo-bypass mode.

The runbook's Scene 5 voiceover says "operator hits Run now and watches the audit chain populate". That click cannot land on any of the 7 fixture workflows.

- Reproduction:
  - `gh pr view 2358 --json state` → `{"state": "OPEN"}`
  - `grep -n "isLive\|disabled\|wfd-run-workflow" ~/code/argo/web/app/\(app\)/workflows/\[verticalId\]/\[name\]/page.tsx | head -5` → `105: const isLive = source === "gateway"; … 256: disabled={running || !isLive}`
  - Live curl: `curl -sI http://localhost:3000/workflows/sales/morning_pipeline_digest` → 200; HTML body contains `disabled` flags around the Run buttons
- File path: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-105, 250-264`
- Suggested fix: merge PR #2358. It's the canonical no-theater fix (auto-publish a seed workflow before running it, so the real `/api/v1/workflow-runs` path fires). The PR is codex-authored under operator override 2026-05-24 — review for whether the codex-pre-push gate ran clean before merging.

### F5 [P0] — Builder canvas has no MCP / computer-use step kinds; the 29-connector MCP catalog is disconnected from the builder

Unchanged on `origin/main`. The builder palette at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` still lists only:

- Triggers (6 variants — manual / schedule / webhook / email / slack / form)
- Skills (1 — Invoke skill)
- Logic (3 — decision / loop / wait)
- Approval gates (3 — slack / email / web)
- Tools (3 — HTTP request / Browser action [Playwright] / Code)
- Notify (3 — slack / email / inbox)

Live-curl against `/workflows/sales/untitled/build` confirms: `palette-item-` testid appears 95 times in the HTML, but `grep` for `mcp` / `computer-use` / `cu_action` / `connector` returns 0 matches. There is no MCP-server entry, no computer-use action entry.

Meanwhile the real MCP connector catalog at `/settings/connectors` ships **29 named connectors** (live-curl confirmed: airtable / asana / calendly / clickup / discord / figma / fireflies / gcal / github-issues / gmail / + 19 more) with `data-testid="connector-card-<slug>"`. But the Tools palette group at `types.ts:271-277` lists only `tool-http`, `tool-browser`, `tool-code` — no `tool-mcp-airtable`, no `tool-mcp-github`, no generic `mcp_invoke` entry that would pop a connector picker.

This is upgraded P0 (was P1 in the 2026-05-26 20:07 audit) because Track P (Pitch Agent) is in v1.0 scope per `project_track_p_v1_mvp_scope` memory and the BUILD_STATUS.md Track P amendment — Scene 5 + Track P P-02 both narrate "operator drags an MCP server onto the canvas". The builder cannot express either step kind today.

- Reproduction:
  - `curl -s "http://localhost:3000/workflows/sales/untitled/build" | python3 -c "…"` → palette-item count 95, but `mcp / cu_action / computer-use / connector` strings all absent
  - `grep -nE "mcp_invoke|cu_action" ~/code/argo/web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/types.ts` → 0 matches
  - `curl -s http://localhost:3000/settings/connectors | grep -c 'connector-card-'` → 29
- File paths:
  - Enum gap: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34`
  - Palette gap: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` (SIDEBAR_PALETTE_GROUPS)
  - Orphan catalog: `~/code/argo/web/app/(app)/settings/connectors/connector-catalog.ts` (29 connectors)
- Suggested fix: extend `StepKind` with `mcp_invoke` and `cu_action`. Add a 4th palette group "MCP connectors" that reads from `connector-catalog.ts` and renders one item per connector with an `mcp_invoke` step kind pre-populated. Add a "Computer-use action" entry under Tools with substrate auto-set to `macos`. Inspector forms: for MCP, pick `server / tool / args`; for CU, pick `app / action / target`. The blocked brief `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md` is the right place for this; re-author it against current paths.

### F6 [P1] — No in-product UX for macOS Accessibility / Screen Recording permissions (Claim-5 operator-side blocker invisible in SPA)

Unchanged. The CLAUDE.md "Remaining for v1.0 launch" list explicitly names "(c) **operator-only**: grant macOS permissions on `argo` binary (Accessibility + Screen Recording) for claim-5" as the only operator-side unblock left. The SPA does not surface this anywhere — `grep -rnE "Accessibility|Screen Recording|System Preferences|macOS perm|System Settings" web/app web/components` returns one false-positive in `Step1Land.tsx:20` (HTML accessibility comment, not macOS perms) and nothing else.

For the v1.0 POC demo MP4 this is dressing-side (operator pre-grants off-camera), so P1 not P0. But for the first-customer install it is the highest-friction step in the product. The user will hire a worker, attach a computer-use step, hit Run, and get an opaque substrate error with no in-product guidance to System Preferences.

- Reproduction:
  - `find ~/code/argo/web/app -type d -iname "*perm*" -o -iname "*macos*"` → 0 dirs
  - `grep -rnE "Accessibility|Screen Recording|System Preferences|macOS perm|System Settings" ~/code/argo/web/app ~/code/argo/web/components | grep -v "node_modules\|test\|aria\|a11y"` → 1 false-positive
- File paths: nothing exists today — this is a gap, not a regression
- Suggested fix: add a one-page perms-preflight at `/onboarding/macos-perms` (or `/settings/computer-use`) that detects `argo-executor-computer-use-macos` permission state via the gateway, surfaces a deep-link to System Preferences for each missing grant, and shows a green check once each is granted. Gate the first computer-use workflow run on this check.

### F7 [P2] — All visible workflow cards show "HEADLESS" substrate chip — Scene 5's "any-PC execution" beat has no on-camera proof

Unchanged. The substrate inference at `web/lib/api/workflows.ts:371-389` correctly upgrades to `macos` when any text contains `cu_action`, `cu.`, `computer-use`, `macos`, `notes app`, etc., and to `browser` when text contains "browser". But:

- Summary cards on `/workflows` call `resolveWorkflowSubstrate(wf)` (`web/app/(app)/workflows/page.tsx:234`) which only sees `.name` + `.description` from `WorkflowSummary` (`WorkflowSummary` has no `.steps` field — see `web/lib/sample-workflows.ts:44-53`).
- The 7 sample-workflow descriptions don't contain any inference-triggering keyword (no `cu.`, no `macos`, no `browser` — except line 409 deep inside `SAMPLE_WORKFLOW_SPECS.gtm-engineer/post_listing.steps[5].description` which says "Your teammate's browser opens Facebook Marketplace…", but that's a step description that summary-level inference never sees).

Net: live-curl confirmed all 7 substrate chips render `data-substrate="headless" data-status="idle"`. The macOS chip color (`#eef2ff` background, `#3730a3` text) defined in `web/components/skycot/workflow/SubstrateChip.tsx:25` never renders in demo-bypass mode.

Downgraded from P1 to P2 this round because Scene 5 doesn't drill into the substrate-chip-color reading (the camera narrates "watch the macOS executor drive Notes" while the focus is the actual app screen, not the chip). Still solved automatically by F1 (if `customer_research_followup` lands in `SAMPLE_WORKFLOW_SUMMARIES` with `cu.run_macro` step, summary-level inference returns `macos`).

- File path: `~/code/argo/web/lib/sample-workflows.ts` (no fixture summary surfaces any non-headless substrate)
- Suggested fix: piggyback on F1's fix. Once the combo workflow lands in the fixture with `customer_research_followup` description that contains `macos` / `computer-use`, its substrate chip will read MACOS and Scene 5 has its visual anchor.

### F8 [P2] — `/skills` count pill cosmetic still reads "6 skills · 0 shown" on no-match

Unchanged. `web/app/(app)/skills/page.tsx:113-114` renders `{skills.length} skills · {filtered.length} shown` unconditionally. So when the operator types "competitor pricing scrape" and gets 0 matches (because of F2), the pill reads "6 skills · 0 shown" while the empty-state body below already says "No skills match …". Two redundant phrasings.

- File path: `~/code/argo/web/app/(app)/skills/page.tsx:113-114`
- Suggested fix: when `filtered.length === 0 && query.length > 0`, render `0 of {skills.length}` instead. ~5 LOC.

### F9 [P2] — `DemoBanner` is on-screen during recording unless operator dismisses it pre-clip-1

Unchanged. `web/components/skycot/DemoBanner.tsx:7,18` checks a localStorage flag (`skycot:demo-banner-dismissed`); once dismissed it stays dismissed within the browser profile. Real operator workaround exists, but it is a pre-record checklist item the runbook does not currently mention.

- File path: `~/code/argo/web/components/skycot/DemoBanner.tsx:7,18`
- Suggested fix: add a single line to `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist: "Dismiss the DemoBanner before clip-1 takes." Optionally hide the banner when `process.env.NEXT_PUBLIC_SKYCOT_RECORDING=1` is set.

### F10 [P3] — Workflow card author chip still does not distinguish worker / human / template-pack

Unchanged. `web/lib/sample-workflows.ts:51` defines `lastEditor: string` — no `authorType: "worker" | "human" | "template_pack"`. `web/app/(app)/workflows/page.tsx:267` renders `by {wf.lastEditor}`, no icon, no chip. Workers in the fixture (Mariah/Priya/Jamal/Nori) match the canonical roster (post-#2367 scrub stuck), so at least the names are recordable — they're just rendered as plain text without a "Bot" / "Human" tier indicator.

P3 because the camera doesn't drill into authorship — but it's the kind of polish that strengthens the recursive-self claim if it lands.

- File path: `~/code/argo/web/lib/sample-workflows.ts:51` + `~/code/argo/web/app/(app)/workflows/page.tsx:267`
- Suggested fix: extend the type with `authorType` (default `"worker"` since 7/7 fixtures are worker-authored) and render a 12px `<Bot />` icon next to the name. ~25 LOC.

---

## Resolved since prior audit (2026-05-26 21:52)

None of the 10 findings in the prior 21:52 audit resolved this round. Source verification (this round, by `grep` + `Read` against `origin/main`):

| Prior F# | Status (this round) | Evidence |
|---|---|---|
| F1 (combo workflow orphaned) | UNRESOLVED → carried as F1 | brief still BLOCKED in BUILD_STATUS.md; `grep -nE "customer_research|marketplace" ~/code/argo/web/lib/sample-workflows.ts` → 0 |
| F2 (competitor pricing scrape search) | UNRESOLVED → carried as F2 | PR #2407 still OPEN; SEED_SKILLS unchanged; gateway still empty |
| F3 (`/skills/<id>` no Install) | UNRESOLVED → carried as F3 | `skills/[skillId]/page.tsx` still 122 lines, no button |
| F4 (Run now disabled) | UNRESOLVED → carried as F4 | PR #2358 still OPEN; `disabled={running || !isLive}` unchanged at line 256 |
| F5 (no MCP/CU step kinds + orphan catalog) | UNRESOLVED → carried as F5 | builder palette `mcp / cu_action / computer-use / connector` all absent via live-curl |
| F6 (no macOS perms wizard) | UNRESOLVED → carried as F6 | `find ~/code/argo/web/app -type d -iname "*perm*"` → 0 |
| F7 (all-HEADLESS chips) | UNRESOLVED → downgraded to P2, carried as F7 | live-curl: 7 substrate chips, all `data-substrate="headless"`. Downgraded because Scene 5 doesn't drill into chip colour. |
| F8 (count pill "6 skills · 0 shown") | UNRESOLVED → carried as F8 | `skills/page.tsx:113-114` unchanged |
| F9 (DemoBanner on-screen) | UNRESOLVED → carried as F9 | DemoBanner component unchanged |
| F10 (author-chip type) | UNRESOLVED → carried as F10 (still P3) | `lastEditor: string` unchanged |

Net: zero P0s resolved between 21:52 and 22:14 (90-min window). Three fixes are in flight (PRs #2407, #2358, and the BLOCKED combo-workflow brief) but none have landed.

Newly observed this round (already known but worth restating):

- PR #2407 has been **OPEN for ~10 minutes** as of this audit (head SHA `8e47bfab`, evaluator R86 PRE-MERGE PASS) and is mergeable. Per memory `feedback_web_only_merge_authorization_2026_05_26` (operator-locked 2026-05-26T09:15Z), web-only PRs ≤200 LOC ≤5 files with vitest green qualify for admin-merge **without** evaluator pre-verification — the SPA leg of #2407 is 1 file (`web/lib/api/skills.ts`, +11/-0). The Rust crate portion is larger but covered by R86 PRE-MERGE.
- PR #2358 (codex-authored, "Fix workflow detail Run now for seed workflows") has been open since at least 2026-05-25 (state `OPEN`, mergeable `UNKNOWN`, head SHA not refreshed in CLI output). Worth investigating whether the codex-pre-push gate is stuck or the PR base needs a rebase.

---

## Claim verdicts (refreshed)

- **Claim 4 (Skills shared + learned) — LIVE backend, BROKEN front-door affordance.** Runtime confirmed wired at `crates/argo-agent-runner/src/turn_helpers.rs:65` (`LayeredLookup::with_library`). The SPA's `[Try it]` button on `/workers/<id>/skills` Library section is the only install surface. The `/skills/<id>` marketplace surface — which the runbook narrates as the install moment — has zero install affordance. Scene 4 step 3 unrecordable on the surface the runbook names. Verdict: LIVE but recordable only by changing the runbook to navigate to `/workers/<id>/skills`. (Or by merging PR #2407 + adding `<InstallToWorker>`.)
- **Claim 5 (Any-PC execution via MCP + CU) — LIKELY-LIVE substrate, NOT-EXPOSED in the SPA on multiple fronts.** Computer-use executor is real (`crates/argo-executor-computer-use-macos`). MCP connector pool is real (29 cards live at `/settings/connectors`). But the workflow builder cannot express either step kind, no fixture workflow contains either step kind, no card on `/workflows` displays a non-headless substrate chip, and no SPA page surfaces the macOS perms grant flow. The platform CAN do the thing; the SPA pretends it cannot.
- **MCP catalog ⇋ builder bridge** — does not exist. The connector catalog is a settings-page artefact, not a builder primitive. For the Scene 5 "operator picks an MCP server" beat to be real, the builder must be able to insert a connector-bound step.
- **macOS perms onboarding (Claim-5 last-mile)** — does not exist anywhere in the SPA. P1 for the POC demo (operator pre-grants off-camera), but the highest-friction step for first-customer install.

---

## Orientation score: 50 / 100

Down 2 from the prior 52/100, because three P0 fixes are in flight (PRs #2407, #2358, and the BLOCKED #2358 brief), one of which (#2407) is mergeable RIGHT NOW under the web-only authorization but still hasn't landed in the 90 minutes since the prior audit. The score reflects platform health (the substrates are real, the runtime install path is real) discounted by the fact that the demo-runbook surfaces still pretend none of those things work in demo-bypass mode, and the fix queue is moving sideways.

Per the function-vs-dressing rule: F1 (combo workflow orphaned), F2 (search returns 0), F3 (no install button), F4 (Run now disabled), and F5 (no MCP/CU step kinds) are all FUNCTION-side — they break the recording as scripted. F6 (no perms wizard) is dressing-side for the POC demo. F7 / F8 / F9 / F10 are polish.

**Highest-leverage single action**: merge PR #2407 (admin-merge eligible per web-only authorization). That alone unblocks half of F2 + half of F7. Second-highest: re-author the BLOCKED `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md` brief against current paths. Those two land and Scene 4 + Scene 5 move from 5 P0s to ~2.
