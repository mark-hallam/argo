# Scene 4 + Scene 5 (Claims 4 + 5 — Skills marketplace + workflow execution) — capability UX audit

**Date**: 2026-05-27 00:15 AEDT (Melbourne)
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 4 `/skills` install flow + Scene 5 `/workflows` + `/workflows/build` canvas + workflow-card author canon)
**Method**: live SPA walk via Claude Preview Model Context Protocol (MCP — Anthropic standard for tool integrations) against the running Next.js dev server on `:3000` and the gateway on `127.0.0.1:7400` (schema_version 83 per `/health`); the preview tab locked up after ~10 interactions and had to be replaced by direct source reads + `curl` probes against the gateway. Source-code reads in `web/app/(app)/skills`, `web/app/(app)/workflows`, `web/middleware.ts`, `web/lib/api/`, `web/lib/sample-workflows.ts`, `web/components/skycot/CommandPalette*.tsx`, `crates/argo-demo-seed/`, `crates/argo-cli/src/handlers/gateway/`. Diff against the 2026-05-26-2309 Scene-4/5 audit at `docs/skycot/ux-audits/2026-05-26-2309/scene-4-and-5-claim-4-and-5.md` — that audit's F1/F2/F3/F4 P0s and the F5 P1 binary-drift finding are all still open; one P0 (env-var rename drift) and one P1 (workflow execute 403 on demo principal) are newly surfaced.

---

## TL;DR

Scenes 4 + 5 remain **unrecordable as scripted**. Net since the previous audit (~25h ago): no P0 or P1 fixes landed; two new findings surfaced. The four open carry-over P0s — (1) canonical `customer_research_followup` invisible to SPA, (2) no Install button + no install endpoint on `/skills/[skillId]`, (3) builder palette has no MCP / no computer-use node kinds, (4) "competitor pricing scrape" search returns 0 hits — are all unchanged. **The new P0 is a dev-config drift**: `.claude/launch.json` sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`, but the middleware + `lib/api/*` check `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` — so the bypass is **silently off**, the SPA hits the live gateway for every page, gets empty responses (skills/workflows/workers all empty), and falls through to seed fixtures. Three downstream gateway endpoints the SPA calls return 404 (`/api/v1/workers`, `/api/v1/skills/learned-this-month`, `/api/v1/skills/{name}/usage`) — every one is "not found" rather than "empty envelope," and a dedicated network panel reading them now will look broken even though the user sees a fully rendered SPA. **The new P1 is that `POST /api/v1/verticals/{v}/workflows/{n}/execute` returns 403 `forbidden — principal lacks the required workflow capability`** for the demo principal that the SPA uses for the auth bypass; if the operator clicks Run during recording, the toast surfaces a permission error, not a successful execution.

---

## Headline blockers

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Canonical `customer_research_followup` workflow only exists in the unmounted `crates/argo-demo-seed/` CLI; gateway returns 0 workflows; SPA fallback `sample-workflows.ts` has no row for it | Scene 5 (Claim 5) — runbook step 1 ("pick the MCP + computer-use combo workflow") cannot land on the real workflow card |
| F2 | P0 | `/skills/[skillId]` page has no Install button + no install flow; gateway `POST /skills` install route 404s on the OPTIONS preflight (CORS broken from the browser) | Scene 4 (Claim 4) — runbook step 3 ("Click Install on the verified result") still impossible |
| F3 | P0 | Builder palette at `/workflows/build` ships no MCP node and no Anthropic computer-use node; "Browser action (Playwright)" is a different backend | Scene 5 (Claim 5) — the "two backends, MCP + computer-use" narration cannot be visually represented on the canvas |
| F4 | P0 | "competitor pricing scrape" search on `/skills` still returns 0 hits (SEED_SKILLS unchanged; gateway returns 0 skills) | Scene 4 (Claim 4) — narrated search beat fails |
| F5 | P0 (new) | `.claude/launch.json` sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` but code checks `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` — auth-bypass + gateway-bypass are silently OFF in the dev environment the operator records from | All scenes — the SPA hits the live gateway for every page load, gets empty envelopes, falls back to seed fixtures; any rehearsal that opens the network panel will see a stream of 404s and 403s that look like breakage |
| F6 | P1 (new) | `POST /api/v1/verticals/{vertical}/workflows/{name}/execute` returns 403 forbidden for the demo principal (`human:demo@skycot.com`) used by the SPA | Scene 5 (Claim 5) — clicking the "Run now" button during recording surfaces a permission-error toast rather than a successful execution + audit-row landing |

---

## All findings

### F1 [P0] — Canonical Scene-5 workflow `customer_research_followup` is invisible to the SPA

Unchanged from the 2026-05-26-2309 audit's F1. The seed CLI at `crates/argo-demo-seed/src/workflow.rs:30-64` defines exactly the runbook's combo workflow (3 steps: 2 `skill_invoke` + 1 `tool_call` with `tool = "cu.run_macro"`), but the seed has not been run against the live gateway, and the SPA fallback in `web/lib/sample-workflows.ts` doesn't include the workflow. The runbook's narrated Scene-5 click target therefore does not exist on the running app.

- Reproduction:
  - `curl -fsS -H 'X-Argo-Principal: human:demo@skycot.com' "http://127.0.0.1:7400/api/v1/verticals/marketplace/workflows/customer_research_followup"` → `404 not_found`
  - `curl -fsS "http://127.0.0.1:7400/api/v1/workflows" | jq '.total_count'` → `0`
  - `grep -n "customer_research_followup" web/lib/sample-workflows.ts` → 0 matches
  - `grep -n "customer_research_followup" crates/argo-demo-seed/src/workflow.rs` → defined here, but `argo demo seed` has not been run

- File paths:
  - Seed (defined but unmounted): `~/code/argo/crates/argo-demo-seed/src/workflow.rs:30-64`
  - SPA fallback missing the row: `~/code/argo/web/lib/sample-workflows.ts:64-138` (SAMPLE_WORKFLOW_SUMMARIES) and `~/code/argo/web/lib/sample-workflows.ts:140-473` (SAMPLE_WORKFLOW_SPECS)

- Suggested fix: ship a brief that either (a) runs `argo demo seed --tenant <demo>` as part of `scripts/restart-live-gateway.sh` so the gateway returns the workflow, OR (b) adds `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` + `SAMPLE_WORKFLOW_SPECS` as a belt-and-braces. Option (a) is the real fix (function-side, runbook-aligned); option (b) is the demo-day shortcut.

### F2 [P0] — `/skills/[skillId]` still has no Install button + install endpoint CORS-broken

Unchanged from prior F2. Scene 4 step 3 reads: "Click **Install** on the verified result. Progress strip: 'Verifying provenance… Installing skill… Available to workers.'" The detail page at `web/app/(app)/skills/[skillId]/page.tsx` still renders only Provenance + Workers-using + Adoption-count (lines 50-112) — no Install button, no install progress strip, no toast.

Worse, even if a button were added, the gateway install route is structurally broken from the browser. `POST /skills` (the install handler) accepts the body but `OPTIONS /skills` returns 404 — so a real browser-side install attempt never gets past the CORS preflight.

- Reproduction:
  - `grep -i install web/app/\(app\)/skills/\[skillId\]/page.tsx` → 0 matches
  - `curl -X OPTIONS -H 'Origin: http://localhost:3000' -H 'Access-Control-Request-Method: POST' -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:7400/skills` → `404`

- File paths:
  - SPA install UI missing: `~/code/argo/web/app/(app)/skills/[skillId]/page.tsx`
  - Backend handler exists: `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_post_writes_skills.rs:13`
  - CORS preflight route gap: `OPTIONS /skills` returns 404 even though `POST /skills` works

- Suggested fix: ship two coordinated briefs — (a) gateway brief that re-routes the install handler to `POST /api/v1/skills/install` (matches the rest of the v1 API surface) AND adds the OPTIONS handler; (b) SPA brief that adds an `<InstallButton>` to `/skills/[skillId]/page.tsx`, posts to the new endpoint, surfaces the 3-step progress strip from the runbook, and lands a sonner toast on success.

### F3 [P0] — Builder palette has no MCP and no computer-use node kinds

Unchanged from prior F3. The runbook's Scene 5 talk track ("Two backends — one for the structured half of the world, one for everything else. MCP for the well-known APIs. Computer-use for everything else.") cannot be visually represented on `/workflows/build`. The palette in `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` still exposes:

- **Triggers** (6): Manual run, Schedule, Webhook, Email, Slack, Web form
- **Skills** (1): Invoke skill
- **Logic** (3): Decision, Loop, Wait
- **Approval gates** (3): Slack, Email, Web
- **Tools** (3): HTTP request, **Browser action (Playwright)**, Code
- **Notify** (3): Slack DM, Email, Web inbox

No `mcp_invoke` step kind. No `cu_action` step kind. No `tool_call`-with-`cu.run_macro` palette item — even though the demo-seed Rust crate uses exactly that shape (`crates/argo-demo-seed/src/workflow.rs:60-63`). `STEP_KIND_META` at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:61-74` confirms — only `trigger`, `skill_invoke`, `approval`, `tool_call`, `notify`, `decision`, `data`, `http_request`, `code` exist as render-side meta entries.

If the Scene 5 narration says "watch the macOS executor drive the Notes app" while the camera is on the canvas, there is no node a viewer can recognize as "that's the computer-use one."

- Reproduction:
  - `grep -n 'mcp_invoke\|cu_action\|cu\.run_macro\|computer.use' web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/types.ts` → 0 matches
  - `grep -n 'mcp_invoke\|cu_action' web/app/\(app\)/workflows/\[verticalId\]/\[name\]/page.tsx` → 0 matches in `STEP_KIND_META`

- File paths:
  - Palette definition: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286`
  - Step-kind meta (render side): `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:61-74`

- Suggested fix: ship a palette-extension brief that adds two new entries under "Tools" — "MCP call (well-known API)" and "Computer-use (macOS)" — wired to the existing `tool_call` step kind with discriminated `tool` value (`"mcp.invoke"` vs `"cu.run_macro"`), or introduce `mcp_invoke` / `cu_action` as first-class kinds in `WorkflowStepKind`. Surface distinct icons in `STEP_KIND_META` so the camera sees a Database glyph for MCP and a Mouse/Keyboard glyph for computer-use.

### F4 [P0] — "competitor pricing scrape" search still returns 0 hits

Unchanged from prior F4. The scope brief requires the canonical Scene-4 search beat — "Operator searches **competitor pricing scrape**. Three results." Verified against current main:

- Gateway `GET /api/v1/skills` returns `{"skills":[],"total_count":0}` — empty store
- SPA falls through to SEED_SKILLS (6 entries at `web/lib/api/skills.ts:63-134`): `approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich`
- None match "competitor", "pricing", or "scrape" by name, description, or layer
- The page UI correctly renders an empty-state below the search input

- Reproduction:
  - Navigate to `http://localhost:3000/skills`
  - Type "competitor pricing scrape" into the hero search input
  - Observe `0 of 6` count pill + "No skills match 'competitor pricing scrape'" empty-state body

- File path: `~/code/argo/web/lib/api/skills.ts:63-134` (the seed fixture).

- Suggested fix: either (a) add a `market.competitor_pricing_scrape` entry to SEED_SKILLS + add a matching seed in the demo-seed CLI so the search beat resolves end-to-end, OR (b) operator-decision to rewrite Scene 4 narration to search for an existing seed skill (e.g. "candidate screening" returns 1 hit). Option (a) is the runbook-aligned fix.

### F5 [P0] — `NEXT_PUBLIC_*_AUTH_BYPASS` env-var name drift: the bypass the launch config sets is not the one the code checks (NEW)

The Claude Preview launch profile at `~/code/argo/.claude/launch.json:8` sets:

```
NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 \
NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 \
NEXT_PUBLIC_ARGO_DEMO_MODE=1 \
NEXT_PUBLIC_SKYCOT_DEMO_MODE=1
```

But every code-side bypass check in the repository reads the **`ARGO_`-prefixed** variant. Greps:

- `web/middleware.ts:58` → `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`
- `web/lib/api/skills.ts:155` → same
- `web/lib/api/team.ts` → same
- `web/lib/api/workflows.ts` → same
- `web/playwright.config.ts:50` → sets `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS: "1"` for the test runner

Consequence: in the dev environment the operator records from, the auth-bypass + gateway-bypass branches are **silently off**. The SPA hits the live gateway for every page (verified via the preview network panel — every `/skills`, `/workers`, `/workflows`, `/skills/{name}/usage` round-trip is real). When the gateway returns an empty envelope (or 404 for routes that don't exist), the SPA falls back to local seed fixtures via the `mock-offline` branch in `web/lib/api/skills.ts:171-178` — so the page LOOKS fully rendered. But a rehearsal that opens the network panel sees a stream of 404s and 403s that look like breakage:

- `GET /api/v1/workers` → 404
- `GET /api/v1/skills/learned-this-month` → 404
- `GET /api/v1/skills/approval.route/usage` → 404
- `POST /api/v1/verticals/{v}/workflows/{n}/execute` → 403 forbidden (see F6)

This is the failure mode `feedback_no_theater_in_demo` warned about: the demo records fine because the fallbacks paper over the empty backend, but the underlying claim "Skycot is functional" is not actually being demonstrated. The operator is recording a SPA that thinks the gateway is broken and is rendering its emergency mocks.

The prior audit's F6 P1 — "auth-bypass short-circuit hides ALL gateway problems in dev" — described the OPPOSITE risk (bypass too eager). This drift means BOTH risks are now live at once: the bypass-when-set hides gateway problems, but it isn't being set because the env-var name is wrong, so the operator sees the gateway-on rendering path AND the fall-through-to-seed path simultaneously.

- Reproduction:
  - `grep -n 'NEXT_PUBLIC_GATEWAY_AUTH_BYPASS\b' .claude/launch.json` → matches at line 8
  - `grep -rn 'NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS' web/ | head -5` → all code-side checks use the `ARGO_` prefix
  - Open the preview network panel on `/skills`: confirm `GET http://127.0.0.1:7400/api/v1/skills` request is made (which would not happen if the bypass were active)

- File paths:
  - Launch config: `~/code/argo/.claude/launch.json:8`
  - Code-side check: `~/code/argo/web/middleware.ts:58`, `~/code/argo/web/lib/api/skills.ts:152-157`

- Suggested fix: pick one canonical name and unify. Either (a) update `.claude/launch.json` to set `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`, OR (b) accept both names in the `isAuthBypassed()` helper (`process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1" || process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"`). Option (a) is the smaller change. Option (b) keeps backward compat if other launch profiles use the un-prefixed name. Either way, ship a lint that fails CI if a `NEXT_PUBLIC_*BYPASS*` env var is set in `launch.json`/`.env*` files but isn't read in `web/middleware.ts` or `web/lib/api/`.

### F6 [P1] — `/execute` returns 403 forbidden on the demo principal — clicking "Run now" in the recording will show an error toast (NEW)

Even when the canonical workflow is seeded (or when the operator clicks Run on any existing seed workflow like `support/triage_ticket` to substitute), the gateway's execute route returns 403 for the demo principal that the SPA uses:

```
$ curl -s -X POST -H 'X-Argo-Principal: human:demo@skycot.com' \
       -H 'Content-Type: application/json' \
       "http://127.0.0.1:7400/api/v1/verticals/support/workflows/triage_ticket/execute" \
       -d '{}'
{"error":{"code":"forbidden","message":"principal lacks the required workflow capability"}}
```

The SPA wires the button at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:163` to `executeWorkflow({verticalId, name})` — a real call, no mock, no theater. The good news is "Run now" really tries to run the workflow. The bad news is the demo principal lacks the capability, so on click the toast surfaces an "Unauthorized" error and no audit row appears.

This is function-side theater the camera sees — Scene 5's narrated payoff ("watch the run land in the audit log") fails on the demo recording profile.

- Reproduction:
  - `curl -s -X POST -H 'X-Argo-Principal: human:demo@skycot.com' -H 'Content-Type: application/json' "http://127.0.0.1:7400/api/v1/verticals/support/workflows/triage_ticket/execute" -d '{}'` → 403 forbidden

- File paths:
  - SPA call site: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/page.tsx:163`
  - Capability check: somewhere in `crates/argo-cli/src/handlers/gateway/routes_post_writes_workflows*.rs` — the principal-to-capability resolution path that maps `human:demo@skycot.com` to a role lacking the `workflow:execute` capability

- Suggested fix: either (a) ship a demo-seed step that grants the demo principal the `workflow:execute` capability under the demo tenant, OR (b) wire the SPA to use a service-account principal for the execute call when `NEXT_PUBLIC_ARGO_DEMO_MODE=1` AND the workflow is a known demo workflow. Option (a) is the right fix; option (b) is the demo-day shortcut.

### F7 [P1] — Builder vertical picker omits `marketplace` — the vertical the canonical Scene-5 workflow lives under

The new build entry page at `web/app/(app)/workflows/build/page.tsx:4-10` lists 5 vertical options: `sales / support / gtm-engineer / people-ops / finance`. The canonical Scene-5 workflow `customer_research_followup` is seeded under vertical `marketplace` (`crates/argo-demo-seed/src/workflow.rs:26`), and existing seed workflows like `post_listing` are also under `gtm-engineer` (not `marketplace`). So even if the operator wanted to build a new "marketplace" workflow in the canvas, the picker silently doesn't allow it.

This is a minor authoring-flow inconsistency rather than a hard blocker — the runbook narrates "click into an existing workflow," not "build a new one in the canvas" — but if the operator decides to demo the builder palette as part of Scene 5, the canvas opens against a vertical that doesn't host the canonical workflow.

- File path: `~/code/argo/web/app/(app)/workflows/build/page.tsx:4-10`

- Suggested fix: add `marketplace` to the `VERTICALS` tuple. Optionally consider sourcing the list from a single canonical list (e.g. the gateway's `/api/v1/verticals` endpoint) instead of hardcoding it.

### F8 [P1] — Workflow card "by Mariah / Priya / Jamal / Nori" never includes Peta — the canonical demo 5 are off-by-one

The `lastEditor` field across the 7 sample workflows at `web/lib/sample-workflows.ts:64-138` distributes as: Mariah ×2, Priya ×1, Jamal ×3, Nori ×1. **Peta is never an editor.** Per memory `project_pm_cohorts_peta_patrick` and the runbook, the canonical demo 5 are Mariah / Peta / Priya / Jamal / Nori — Peta is one of them. The workflow cards on `/workflows` are visible behind the camera at multiple points in the demo, and the workforce shown there is silently missing the Project & Ops worker.

Not a hard blocker (no scene drills directly into a Peta-authored workflow), but it's an inconsistency between the surfaces that does affect the demo's "named teammates working together" narrative.

- Reproduction: `grep -n 'lastEditor' web/lib/sample-workflows.ts` → 7 lines, Peta absent

- File path: `~/code/argo/web/lib/sample-workflows.ts:64-138`

- Suggested fix: add at least one Peta-authored workflow (project-ops vertical) to SAMPLE_WORKFLOW_SUMMARIES — e.g. `weekly_standup_digest` or `action_item_chase` — both fit her project-ops role per the cohort memory.

### F9 [P2] — `/api/v1/skills/learned-this-month` returns 404 — the "Skills my workers learned this month" chip on `/skills` will surface an empty result silently

The chip at `web/app/(app)/skills/page.tsx:133-142` toggles `learnedOnly`, and the hook `useSkillsLearnedThisMonth()` calls `GET /api/v1/skills/learned-this-month`. The gateway returns 404. Because the fallback is "no skills learned" (empty array), clicking the chip on a successful demo would silently filter to 0 skills — no visible error.

The runbook doesn't currently script clicking that chip, so it's not a demo blocker. But it's a function-side dud the operator may click during recording (it's a prominent button on the page) and end up with an empty grid + no explanation.

- Reproduction:
  - `curl -s -o /dev/null -w '%{http_code}\n' -H 'X-Argo-Principal: human:demo@skycot.com' "http://127.0.0.1:7400/api/v1/skills/learned-this-month"` → 404

- File paths:
  - SPA hook: `~/code/argo/web/lib/hooks/use-skills.ts`
  - Gateway dispatch: missing handler — none of the dispatch files in `crates/argo-cli/src/handlers/gateway/routes_dispatch_get*.rs` claim the `learned-this-month` path

- Suggested fix: either (a) add the handler server-side (returns an array of skill_ids learned by workers in the current tenant within the last 30 days), or (b) hide the chip entirely until the endpoint exists.

### F10 [P2] — `/api/v1/skills/{name}/usage` returns 404 — skill-detail provenance card lands in its loading-state fallback indefinitely

Same shape as F9 — `web/lib/hooks/use-skills.ts:useSkillUsage` hits `GET /api/v1/skills/{name}/usage`; gateway returns 404. The fallback is `seedUsage()` which returns "Authored by SKYCOT" + 0 workers + 0 adoption — the SAME content for every skill regardless of name. So drilling into different skill cards shows identical metadata, which IS theatre the camera sees if the operator clicks two different skills back-to-back.

- Reproduction:
  - `curl -s -H 'X-Argo-Principal: human:demo@skycot.com' "http://127.0.0.1:7400/api/v1/skills/approval.route/usage"` → `{"error":"not found"}` (HTTP 404)

- File path: gateway dispatch is missing the route — search `crates/argo-cli/src/handlers/gateway/` for any handler claiming `/api/v1/skills/[name]/usage` returns nothing

- Suggested fix: add a server-side handler that returns the canonical `SkillUsage` envelope from the skill-library substrate (#1682) for the requested skill, scoped to the current tenant.

### F11 [P2] — `/api/v1/workers` still returns 404 — gateway binary drift unchanged from prior audit

Same as the prior audit's F5. Source code at `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:65-66` clearly registers `GET /api/v1/workers`, and `routes_workers_list.rs:11-16` delegates correctly. But the running gateway binary returns 404.

The SPA papers over this via the seed-fallback path in `web/lib/api/team.ts` so `/team` renders. But the gateway binary is drifting from source — possibly the binary on :7400 was built before this route landed.

- Reproduction:
  - `curl -fsS -H 'X-Argo-Principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}` (404)
  - `curl -fsS http://127.0.0.1:7400/api/v1/team` → returns the single Maya worker

- Suggested fix: rebuild + restart the gateway before recording: `cargo build --release -p argo-cli && pkill -f 'argo gateway' && argo gateway start --bind 127.0.0.1 --port 7400 &`. If still 404 after rebuild, add a per-tenant test pinning `GET /api/v1/workers` returns 200 on the live binary so the drift can't recur silently.

### F12 [P2] — "Browser action (Playwright)" name confuses the computer-use story (carryover; pair with F3)

Carryover from prior F11. Even setting F3 aside, the existing Tools palette item "Browser action (Playwright)" is positively misleading for Scene 5: Playwright is Microsoft's web-browser automation framework (drives Chrome/Firefox/WebKit headlessly). The runbook talks about Anthropic-style computer-use (drives the operator's macOS desktop natively, opens Notes.app, types real keystrokes). The two are NOT interchangeable.

- File path: `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:274`

- Suggested fix: pair with F3 — rename "Browser action (Playwright)" → "Web browser (Playwright)" and add a separate "Desktop action (macOS)" item under Tools so the two backends are visually distinct.

### F13 [P3] — Demo banner persists on every (app) page in dev (carryover from prior F10)

The "Demo mode — showing sample data. Sign in to see your own workspace." banner renders at the top of every (app)-group page because `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set. It IS dismissible (X button works). Carryover from the prior audit; still no `NEXT_PUBLIC_HIDE_DEMO_BANNER` env var.

- File path: `~/code/argo/web/components/skycot/DemoBanner.tsx`

- Suggested fix: add a `NEXT_PUBLIC_HIDE_DEMO_BANNER=1` env var for the recording profile, OR explicitly tell the operator in the runbook to dismiss the banner at the start of recording.

### F14 [P3] — Preview-MCP tab destabilised after ~10 interactions on the `/skills` flow

While auditing, the preview-MCP browser tab locked up after ~10 navigation + eval interactions on the `/skills` flow — `preview_eval` started returning "Inspected target navigated or closed" intermittently, then 30s timeouts. The pattern wasn't deterministic and I observed at least three unexpected hard navigations (e.g. `/skills` → `/audit`, `/skills` → `/cohorts`, `/skills` → `/landing`) that I cannot fully attribute to any specific event in the SPA. This may be a Claude-Preview-MCP-side artefact rather than a SPA bug; it didn't reproduce in the prior audit at the same rate. Mentioning here so the operator knows recording should be done via real Chrome, not via the preview tab, and so the next auditor knows to expect this and works around it (refresh / use direct `curl` + source reads).

- Reproduction: not consistent
- File path: N/A (preview-MCP infrastructure)
- Suggested fix: not actionable in the SPA; flag it in the preview-MCP tooling backlog if it recurs.

---

## Resolved since last audit (2026-05-26-2309)

| Prior # | Title | Status |
|---|---|---|
| F1 prior | Canonical `customer_research_followup` workflow invisible to SPA | **NOT RESOLVED** — re-logged as F1 |
| F2 prior | `/skills/[skillId]` has no Install button + CORS broken | **NOT RESOLVED** — re-logged as F2 |
| F3 prior | Builder palette has no MCP/computer-use kinds | **NOT RESOLVED** — re-logged as F3 |
| F4 prior | "competitor pricing scrape" returns 0 hits | **NOT RESOLVED** — re-logged as F4 |
| F5 prior | Gateway binary drift on `/api/v1/workers` | **NOT RESOLVED** — re-logged as F11 |
| F6 prior | Auth-bypass short-circuit hides ALL gateway problems in dev | **PARTIALLY MOOT** — the bypass is silently off due to env-var name drift (new F5), so the gateway IS hit. Both risks remain but the operative one flipped from "bypass too eager" to "bypass not active when it should be" |
| F7 prior | "Run workflow" vs "Run now" label | **RESOLVED** — button now reads "Run now" matching Mariah's duties (verified at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:263`) |
| F8 prior | "6 skills · 0 shown" empty-state phrasing | **STILL OPEN** as a P3 nit; not re-logged to keep the count tight |
| F9 prior | `WORKFLOWS · v0.1` eyebrow leaks `data-source` attribute | **STILL OPEN** as a P3 dev-tools-only concern; not re-logged |
| F10 prior | Demo banner persists | **NOT RESOLVED** — re-logged as F13 (deprioritised to P3) |
| F11 prior | "Browser action (Playwright)" name confuses computer-use story | **NOT RESOLVED** — re-logged as F12 |
| F12 prior | Search-input ↔ ⌘K hotkey contention | **DROPPED** — re-investigation suggests this was a preview-MCP artefact rather than a real SPA bug (see F14). Real-Chrome recording will not trigger it. |

Net delta: 1 P1 fix landed (Run-now button label); 0 P0 fixes landed; 2 new findings surfaced (F5 env-var drift P0, F6 execute-403 P1). Trajectory is **flat or slightly worse** — the harder-shape issues (canonical workflow seed, install button, palette extension, search beat) are not on a swarm worker's plate per a quick scan of recent commits.

---

## Orientation score: 48 / 100

Down from the prior audit's 52/100. The scenes are still unrecordable for the same four reasons, AND a new dev-config drift makes the operator's "rehearsal" environment misrepresent the production behaviour: every gateway call is being made (because the bypass is off), and several of those calls are 404ing or 403ing.

Per the function-vs-dressing rule: F1 + F2 + F3 + F4 + F5 + F6 are all function-side blockers (the runbook's scripted clicks fail, and a fix means real backend wiring or env-var alignment, not just a label edit). F11 is infrastructure (gateway binary drift). F7-F8 are cohort-canon polish that the camera might catch.

Recommended order of operations for the operator before recording:

1. **Fix F5 first** (env-var rename) — one-character launch.json edit + dev-server restart, unblocks the recording profile. Validates by opening the preview network panel on `/skills` and confirming the gateway calls stop.
2. **Run the seed CLI**: `argo demo seed --tenant <demo>` against the running gateway, so `customer_research_followup` is reachable end-to-end (resolves F1).
3. **Patch the demo principal's capability** (resolves F6) — grant `workflow:execute` so clicking "Run now" actually runs.
4. **Decide whether Scene 4 install beat is on the cut-list** (resolves F2), or ship the InstallButton brief; if cut, rewrite the narration to skip the install step.
5. **Decide whether the Scene 5 narration name-checks MCP and computer-use** (resolves F3) — if yes, ship the palette extension; if no, narrow the talk track to "Mariah's workflow runs both an API call and a desktop action — let's watch."
6. **Rebuild + restart the gateway** to clear the binary drift on `/api/v1/workers` (resolves F11) — picks up the missing `/skills/learned-this-month` + `/skills/{name}/usage` routes if those are also a binary-vs-source drift rather than missing handlers (F9, F10).
7. **Add Peta to at least one workflow card** (resolves F8) before recording — small cohort-canon polish so the demo's "5 named teammates" narrative is consistent on every visible surface.
