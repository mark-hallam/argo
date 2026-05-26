# Scene 4 + Scene 5 (Claims 4 + 5 — Skills marketplace + workflow execution) — capability UX audit

**Date**: 2026-05-26 23:09 AEDT (Melbourne)
**Auditor**: e2e-ux-journey-test-agent (scope: Scene 4 `/skills` install flow + Scene 5 `/workflows` + `/workflows/build` canvas)
**Method**: live SPA walk via Claude Preview Model Context Protocol (MCP — Anthropic standard for tool integrations) against the running Next.js dev server on `:3000` and the gateway on `127.0.0.1:7400` (schema_version 83 per `/health`); source-code reads in `web/app/(app)/skills`, `web/app/(app)/workflows`, `web/lib/api/`, `crates/argo-demo-seed/`, and `crates/argo-cli/src/handlers/gateway/`; direct `curl` to confirm gateway endpoint shapes; first-pass deltas against the prior 2026-05-25 Scene-4/5 audit at `docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`.

---

## TL;DR

Scenes 4 + 5 are **still unrecordable as scripted**, but four of the previous audit's five P0s have shipped fixes. Net: 3 new P0 findings (one carried-over + two surfaced this audit), 3 P1s, 4 P2s. The merge train resolved the on-camera "WORKFLOWS · MOCK" label (F1 prior), the non-canon Rio/Sam author chips (F4 prior), the missing `/workflows/build` entry point (F8 prior), and the SubWorkflowPicker SEED leak (F9 prior). What's NOT fixed: **(1) the runbook's canonical Scene-5 workflow `customer_research_followup` exists ONLY in the unmounted `argo-demo-seed` Rust CLI and is invisible to the running SPA because `argo demo seed` has not been run against the live gateway** (gateway returns `workflows: []`; SPA falls back to `SAMPLE_WORKFLOW_SUMMARIES` which has 7 workflows, none of them the canonical one). **(2) the `/skills/[skillId]` detail page STILL has no Install button** — the runbook's narrated install beat is structurally absent and the gateway install route at `POST /skills` still 404s on OPTIONS preflight (CORS — Cross-Origin Resource Sharing — broken). **(3) the `/workflows/build` palette has NO MCP and NO Anthropic-style computer-use node types**; the closest analogue is "Browser action (Playwright)" which is a different automation tech. **(4) the canonical Scene-4 search query "competitor pricing scrape" still returns 0 cards** because the SEED_SKILLS fixture list (6 entries) doesn't include any competitor/pricing/scrape skill, and the gateway returns 0 skills. The `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` flag in `.claude/launch.json` short-circuits the gateway call entirely in dev — both `/skills` and `/team` render their seed fixtures without ever hitting the gateway, even when the gateway is up.

---

## Headline blockers

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Canonical `customer_research_followup` workflow only exists in `crates/argo-demo-seed/` — seed CLI has not been run, gateway returns 0 workflows, SPA fallback seed has no row for it | Scene 5 (Claim 5) — runbook step 1 ("pick the MCP + computer-use combo workflow") cannot land on the real workflow card |
| F2 | P0 | `/skills/[skillId]` detail page has no Install button + no install flow + `POST /skills` install route 404s on OPTIONS preflight | Scene 4 (Claim 4) — runbook step 3 ("Click Install on the verified result") still impossible |
| F3 | P0 | Builder palette at `/workflows/build` ships no MCP node and no Anthropic computer-use node; "Browser action (Playwright)" is a different backend | Scene 5 (Claim 5) — the "two backends, MCP + computer-use" narration cannot be visually represented on the canvas |
| F4 | P0 | "competitor pricing scrape" search on `/skills` still returns 0 hits (SEED_SKILLS unchanged from prior audit; gateway returns 0 skills) | Scene 4 (Claim 4) — narrated search beat fails |
| F5 | P1 | Gateway `GET /api/v1/workers` returns 404 (the running binary doesn't have the route the source code added), the SPA only survives because `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits to SAMPLE_TEAM | All scenes that touch worker data — the live binary needs a rebuild before recording |

---

## All findings

### F1 [P0] — Canonical Scene-5 workflow `customer_research_followup` is invisible to the SPA

The runbook's Scene 5 narration ("pick a workflow with two steps: step 1 = MCP call `zendesk:search_tickets`, step 2 = computer-use action `macos_notes:create_note`") requires a workflow whose `/workflows` card the camera can click. The seed CLI at `crates/argo-demo-seed/src/workflow.rs:30-64` defines exactly this workflow (named `customer_research_followup`, under the `marketplace` vertical, with a `tool_call` step using `tool = "cu.run_macro"`). But:

1. The gateway returns `{"workflows":[],"total_count":0}` on `GET /api/v1/workflows` — the seed has not been run against the current tenant.
2. The SPA fallback `web/lib/sample-workflows.ts` has 7 workflow summaries (post_listing, screen_candidate, approve_refund, qualify_lead, triage_ticket, morning_pipeline_digest, clean_customer_list) — none is `customer_research_followup`.
3. The SPA's `/workflows` page therefore renders 7 cards, none of which has the right `(vertical, name)` pair for the runbook to land on.
4. `GET /workflows/marketplace/customer_research_followup` on the dev server returns 200 (Next.js dynamic route catches it), but `useWorkflow("marketplace","customer_research_followup")` hits the gateway, gets 404, and falls through to the seed — which doesn't have it either — so the page renders its "workflow not found" edge state.

- Reproduction:
  - `curl -fsS http://127.0.0.1:7400/api/v1/workflows | jq '.total_count'` → `0`
  - `curl -fsS -H 'X-Argo-Principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/verticals/marketplace/workflows/customer_research_followup` → `{"error":{"code":"not_found","message":"no head workflow definition for (marketplace, customer_research_followup)"}}`
  - `open http://localhost:3000/workflows` → 7 cards, none containing the substring "customer_research"
  - `grep -n "customer_research_followup" web/lib/sample-workflows.ts` → 0 matches

- File paths:
  - Seed (works but unmounted): `crates/argo-demo-seed/src/workflow.rs:30-64`
  - SPA fallback missing the row: `web/lib/sample-workflows.ts:64-138` (SAMPLE_WORKFLOW_SUMMARIES) and `web/lib/sample-workflows.ts:140-473` (SAMPLE_WORKFLOW_SPECS)
- Suggested fix: either (a) ship a brief that runs `argo demo seed --tenant <demo>` as part of `scripts/restart-live-gateway.sh` so the gateway actually returns the workflow, OR (b) add `customer_research_followup` to `SAMPLE_WORKFLOW_SUMMARIES` + `SAMPLE_WORKFLOW_SPECS` so the SPA fallback covers it. Option (a) is the real fix; option (b) is the demo-day shortcut.

### F2 [P0] — `/skills/[skillId]` still has no Install button + install endpoint CORS-broken

Scene 4 step 3 reads: "Click **Install** on the verified result. Progress strip: 'Verifying provenance… Installing skill… Available to workers.'" The detail page at `web/app/(app)/skills/[skillId]/page.tsx` still renders only Provenance + Workers-using + Adoption-count (lines 50-112) — no Install button, no install progress strip, no toast.

Worse, even if a button were added, the gateway install route is structurally broken from the browser. `POST /skills` (the install handler, mounted at `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_paths.rs:109-111`) is at the un-prefixed `/skills` path, and:

```
$ curl -X POST -H 'X-Argo-Principal: human:demo@skycot.com' \
       -d '{"raw":"..."}' http://127.0.0.1:7400/skills → 200 (validation error, but route exists)
$ curl -X OPTIONS -H 'Origin: http://localhost:3000' \
       -H 'Access-Control-Request-Method: POST' \
       -I http://127.0.0.1:7400/skills → 404 Not Found
```

The CORS (Cross-Origin Resource Sharing — browser rule that controls when a webpage on one origin can talk to a server on another origin) preflight returns 404, so a real browser-side install attempt would never get past the OPTIONS check. This is the same CORS-preflight bug logged as F6 in the prior audit — still unfixed for the install path.

Note: the worker-skills "Try it" affordance at `/workers/{id}/skills` DOES wire to `POST /api/v1/workers/{id}/skills` and surfaces correctly (`web/lib/api/worker-skills.ts:181-193`, `useTryLibrarySkill` in `web/lib/hooks/use-worker-skills.ts:150`). But that's a different operation — "enable an existing workspace-library skill on this worker" — not "install a new SKILL.md from the marketplace." Scene 4 narrates the latter.

- Reproduction:
  - `grep -i install web/app/(app)/skills/\[skillId\]/page.tsx` → 0 matches
  - `curl -X OPTIONS -H 'Origin: http://localhost:3000' -I http://127.0.0.1:7400/skills` → 404
- File paths:
  - SPA install UI missing: `web/app/(app)/skills/[skillId]/page.tsx`
  - Backend handler exists: `crates/argo-cli/src/handlers/gateway/routes_post_writes_skills.rs:13`
  - Dispatch line: `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_paths.rs:109-111`
  - CORS preflight route gap: anywhere the un-prefixed `/skills` path is bound — `OPTIONS /skills` returns 404 even though `POST /skills` works
- Suggested fix: ship two coordinated briefs — (a) gateway brief that re-routes the install handler to `POST /api/v1/skills/install` (matches the rest of the v1 API surface) AND adds the OPTIONS handler; (b) SPA brief that adds an `<InstallButton>` to `/skills/[skillId]/page.tsx`, posts to the new install endpoint, surfaces the 3-step progress strip from the runbook, and lands a sonner toast on success.

### F3 [P0] — Builder palette has no MCP and no computer-use node kinds

The runbook's Scene 5 talk track ("Two backends — one for the structured half of the world, one for everything else. MCP for the well-known APIs. Computer-use for everything else.") cannot be visually represented on `/workflows/build`. The palette in `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-285` exposes:

- **Triggers** (6): Manual run, Schedule, Webhook, Email, Slack, Web form
- **Skills** (1): Invoke skill
- **Logic** (3): Decision, Loop, Wait
- **Approval gates** (3): Slack, Email, Web
- **Tools** (3): **HTTP request**, **Browser action (Playwright)**, **Code**
- **Notify** (3): Slack DM, Email, Web inbox

No `mcp_invoke` step kind. No `cu_action` step kind. No `tool_call`-with-`cu.run_macro` palette item even though the demo-seed Rust crate uses exactly that shape (`crates/argo-demo-seed/src/workflow.rs:60-63`). "Browser action (Playwright)" is a third-party browser automation tech — NOT the Anthropic-style native desktop control the runbook describes.

If the Scene 5 narration says "watch the macOS executor drive the Notes app" while the camera is on the canvas, there is no node a viewer can recognize as "that's the computer-use one."

- Reproduction:
  - `open http://localhost:3000/workflows/sales/untitled/build`
  - Read the palette aside (left rail) — confirm Tools section lists HTTP request / Browser action (Playwright) / Code
  - `grep -n 'mcp\|computer.use\|cu_action\|cu\.run_macro' web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` → 0 matches in palette / step-kind enum
- File paths:
  - Palette definition: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-285`
  - Step kind enum: `web/lib/sample-workflows.ts:19-28` (no `mcp_invoke` / `cu_action`)
- Suggested fix: ship a palette-extension brief that adds two new entries to SIDEBAR_PALETTE_GROUPS under "Tools" — "MCP call (well-known API)" and "Computer-use (macOS)" — wired to the existing `tool_call` step kind with discriminated `tool` value (`"mcp.invoke"` vs `"cu.run_macro"`). The render-side STEP_KIND_META in `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:65-74` should ideally surface them with distinct icons so the camera sees a Database-icon for MCP and a Mouse/Keyboard-icon for computer-use.

### F4 [P0] — "competitor pricing scrape" search still returns 0 hits

The scope brief requires the canonical Scene-4 search beat — "Operator searches **competitor pricing scrape**. Three results." Verified against current main:

- Gateway `GET /api/v1/skills` returns `{"skills":[],"total_count":0}`
- SPA falls through to SEED_SKILLS (6 entries at `web/lib/api/skills.ts:63-134`): `approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich`
- None of these match "competitor", "pricing", or "scrape" by name, description, or layer
- The page UI correctly renders an empty-state below the search input

This is unchanged from F2 of the prior audit.

- Reproduction:
  - `open http://localhost:3000/skills`
  - Type "competitor pricing scrape" into the hero search input
  - Observe `0 of 6` count pill + "No skills match 'competitor pricing scrape'" empty-state body

- File path: `web/lib/api/skills.ts:63-134` (the seed fixture). Also affects the demo-seed CLI — `crates/argo-skill/tests/claim_4_live_install_invoke.rs` tests use generic skill names, not the runbook's canonical "competitor pricing scrape" string.
- Suggested fix: either (a) add a `competitor.pricing.scrape` (or `market.scrape`) entry to SEED_SKILLS + ship a brief that makes the demo-seed CLI also seed it server-side so the search beat resolves end-to-end, OR (b) operator-decision to rewrite the Scene 4 talk track to search for an existing seed skill (e.g. "candidate screening" already returns 1 hit). Option (a) is the runbook-aligned fix.

### F5 [P1] — Live gateway is missing the `/api/v1/workers` collection route (binary drift vs source)

The dispatcher in `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:65-66` clearly registers `GET /api/v1/workers` and `routes_workers_list.rs:11-16` delegates correctly to `routes_team_list::list_response`. But the running gateway returns 404 for both `GET /api/v1/workers` and `GET /api/v1/skills/learned-this-month`. Possible causes (in order of likelihood): (1) the gateway binary running on :7400 was built before these routes were added — the operator needs to `cargo build --release -p argo-cli && pkill argo && argo gateway start ...` to pick up current main; (2) a route-ordering bug where an earlier `if` block catches `/api/v1/workers` as a per-worker singleton (single ID match) before the bare-collection branch fires.

The reason this isn't a P0: the SPA's `fetchTeam()` short-circuits to `SAMPLE_TEAM` (5 workers: Mariah/Peta/Priya/Jamal/Nori) via `isAuthBypassed()` when `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` is set (`web/lib/api/team.ts:59-60`). So `/team` renders correctly. But:
- The fallback masks a real binary-vs-source drift problem
- Any Scene-1/2 segment that drills into worker activity / overview / memory will hit one of the per-worker endpoints that all 404, and the in-SPA seed fallback for those may not match the runbook's expectations

- Reproduction:
  - `curl -fsS -H 'X-Argo-Principal: human:demo@skycot.com' http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}`
  - `curl -fsS http://127.0.0.1:7400/api/v1/team | jq '.total_count'` → `1` (returns just Maya from the live store)
  - Source route exists: `grep 'GET /api/v1/workers ' crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:65`
- Suggested fix: rebuild + restart the gateway before recording. Run `cargo build --release -p argo-cli && pkill -f 'argo gateway' && argo gateway start --bind 127.0.0.1 --port 7400 &`. If still 404 after rebuild, add a per-tenant test pinning `GET /api/v1/workers` returns 200 on the live binary.

### F6 [P1] — Auth-bypass short-circuit hides ALL gateway problems in dev

The `.claude/launch.json` config sets both `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` AND `NEXT_PUBLIC_ARGO_DEMO_MODE=1` AND `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. The `isAuthBypassed()` branch in `web/lib/api/skills.ts:152-157` (and identical in `web/lib/api/team.ts:50-56`) returns the local SEED before hitting the gateway. Result: the dev environment is structurally incapable of seeing what the live gateway returns. The pages render with the same 6 skills + 5 workers regardless of whether the gateway is running, empty, populated, or down.

This is appropriate for offline development but **dangerous for demo-recording preparation**. The operator could record an entire 8-minute demo with the gateway returning empty `{}` envelopes and never know. Specifically, F1 (no `customer_research_followup` in the SPA fallback) only matters in the demo because the SPA never calls the gateway during recording — if it did, even the empty gateway would route the camera through the empty-list empty-state rather than the 7-card fallback.

- Reproduction:
  - Stop the gateway: `pkill -f 'argo gateway'`
  - Reload `/skills` and `/team` — both still render fully
  - Network panel shows zero gateway calls for the skills/team endpoints
- File paths: `web/lib/api/skills.ts:152-157`, `web/lib/api/team.ts:50-56`, `web/lib/api/workflows.ts` (same pattern)
- Suggested fix: for the demo-recording build, the operator should export `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0` while keeping `NEXT_PUBLIC_ARGO_DEMO_MODE=1` — so the SPA hits the live gateway and the empty-store fallback path is exercised in the same shape the camera will see. The `scripts/demo-dry-run.sh` precheck should also assert at least one skill + one workflow are present in the gateway response.

### F7 [P1] — Workflow detail page button label is "Run workflow"; runbook says "Run now" / "Hit Run"

Minor vocab mismatch carried over from the prior audit (F7 prior). The button at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` labels itself "Run workflow" / "Running…" — runbook narration ("Hit **Run**") and Mariah's duties tab ("Run now") both differ. The button does call the real endpoint (`executeWorkflow` → `POST /api/v1/verticals/{v}/workflows/{n}/execute` returns 401 without principal, 400 with bad principal — a real endpoint, not a stub).

- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`
- Suggested fix: pick a single string ("Run now") for both the duty action and the workflow detail action.

### F8 [P2] — `/skills` page hero-count phrasing inconsistency on filtered state

Cosmetic carry-over from F11 prior. When the operator types a non-matching query, the count pill reads `6 skills · 0 shown`, while the body below reads `No skills match "<query>"`. The two phrasings are redundant.

- File path: `web/app/(app)/skills/page.tsx` (the count-pill render)
- Suggested fix: when `filtered.length === 0`, render `0 of 6` instead of `6 skills · 0 shown` to match the empty-state body language.

### F9 [P2] — `/workflows` source-pill is `WORKFLOWS · v0.1` but `data-source` attribute leaks fallback state on inspect

F1 from the prior audit (the on-camera `WORKFLOWS · MOCK` label) is RESOLVED — the eyebrow now reads `WORKFLOWS · v0.1` regardless of source. But the underlying `data-source` attribute is still emitted on the same DOM element (`web/app/(app)/workflows/page.tsx:65-67`) with values `"gateway"` / `"mock-offline"` / `"mock-bypass"`. The camera won't see it without opening dev tools, so it's not a P0 demo-blocker — but the attribute is structurally leaking the seed-fallback state into the production DOM. Either keep it (it's behind a `data-testid` for test reuse) or scope it to dev builds.

- File path: `web/app/(app)/workflows/page.tsx:63-70`
- Suggested fix: leave as-is for now; revisit if customer-facing dev-tools inspections become a thing.

### F10 [P2] — Demo banner persists on every (app) page in dev

The "Demo mode — showing sample data. Sign in to see your own workspace." banner renders at the top of every page in the (app) group because `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set. It IS dismissible (the X button works). For the demo recording, this is fine as long as the operator dismisses it once at recording start — but if they forget, it's on camera for 8 minutes. There's no auto-dismiss-when-recording-flag mechanism.

- Reproduction:
  - `open http://localhost:3000/skills` → banner visible at top
  - `open http://localhost:3000/workflows` → banner visible at top
- File path: `web/components/skycot/DemoBanner.tsx`
- Suggested fix: add a `NEXT_PUBLIC_HIDE_DEMO_BANNER=1` env var for the recording profile, OR have the recording-prep step in the runbook explicitly say "click X on the demo banner at the start of every scene transition."

### F11 [P2] — Builder canvas "Browser action (Playwright)" name confuses the computer-use story

Even setting F3 aside, the existing Tools palette item "Browser action (Playwright)" is positively misleading for Scene 5: Playwright is Microsoft's web-browser automation framework (drives Chrome/Firefox/WebKit headlessly). The runbook talks about Anthropic-style computer-use (drives the operator's macOS desktop natively, opens Notes.app, types real keystrokes). The two are NOT interchangeable — Playwright can't open Notes.app or any native macOS app. If a viewer sees "Browser action" in the palette while hearing "anything a human can do on a Mac," they will reasonably conclude SKYCOT only automates browsers.

- File path: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:274`
- Suggested fix: pair with F3 — rename to "Browser action (Playwright)" → "Web browser (Playwright)" and add a separate "Desktop action (macOS)" item under Tools so the two backends are visually distinct.

### F12 [P3] — Search input on `/skills` triggers what looks like global-hotkey contention with the ⌘K command palette

While auditing, I observed that filling text into the `/skills` search input occasionally caused the page to navigate away (to `/approvals`, `/hire`, `/team`, etc. — destinations matching partial substrings of the typed query). This appears to be the ⌘K command palette grabbing global keystrokes when the input loses focus during a re-render. Hard to reproduce deterministically (intermittent, only seen during the JS-eval tool's interaction). Probably a focus-management bug — likely cmd palette's open-on-keypress vs input's onChange race.

- Reproduction (intermittent):
  - `open http://localhost:3000/skills`
  - Click into the search input, type slowly, observe whether focus stays in the input
- File path: `web/components/skycot/CommandPalette.tsx` (suspected)
- Suggested fix: don't act on this directly — needs a real user click + keystroke test before deciding it's a real bug vs preview-MCP artefact. Adding here for the operator to be aware of during recording (avoid typing too fast in any input).

---

## Resolved since last audit (2026-05-25-1100)

Comparing against `docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`:

| Prior # | Title | Status |
|---|---|---|
| F1 prior | `/workflows` "WORKFLOWS · MOCK" eyebrow pill on camera | **RESOLVED** — now reads `WORKFLOWS · v0.1` (`web/app/(app)/workflows/page.tsx:69`) |
| F2 prior | "competitor pricing scrape" returns 0 results | **NOT RESOLVED** — re-logged as F4 in this audit |
| F3 prior | `customer_research_followup` workflow doesn't exist | **PARTIALLY RESOLVED** — now defined in `crates/argo-demo-seed/src/workflow.rs` BUT seed not run + no SPA fallback row, so still invisible (re-logged as F1) |
| F4 prior | Workflow cards by non-canon "Rio" and "Sam" | **RESOLVED** — `web/lib/sample-workflows.ts` now uses Mariah/Priya/Jamal/Nori only (verified in source + screenshot) |
| F5 prior | `/skills/[skillId]` has no Install button | **NOT RESOLVED** — re-logged as F2 in this audit |
| F6 prior | Gateway OPTIONS `/skills` returns 404 (CORS preflight broken) | **PARTIALLY RESOLVED** — `OPTIONS /api/v1/skills` returns 204 now, but `OPTIONS /skills` (the install path) still 404s; re-logged as part of F2 |
| F7 prior | "Run workflow" vs "Run now" label | **NOT RESOLVED** — re-logged as F7 |
| F8 prior | `/workflows/build` 404s | **RESOLVED** — `web/app/(app)/workflows/build/page.tsx` now renders a vertical picker that routes into the existing builder |
| F9 prior | `SubWorkflowPicker` uses SEED_WORKFLOWS as primary | **RESOLVED** — now prefers `useWorkflows()` live data, falls back to seed only when empty + not loading (`web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx:89-95`) |
| F10 prior | No roster-check unit test on `lastEditor` | **STILL OPEN as a P3 nit** (not re-logged — minor hygiene) |
| F11 prior | "6 skills · 0 shown" empty-state phrasing | **NOT RESOLVED** — re-logged as F8 |

Net delta: 4 of 5 P0s shipped fixes; 2 of 4 P1s shipped fixes; new P0 surfaced (F3 — builder palette has no MCP/computer-use); new P1 surfaced (F5 — gateway binary drift).

---

## Orientation score: 52 / 100

Up from the prior audit's 38/100. Scene 4 + Scene 5 are still unrecordable as the runbook scripts them, but the trajectory is positive — the merge train has shipped the easy theatre fixes (MOCK pill, non-canon names, missing routes) and exposed the harder shape question underneath: **the demo recording is supposed to demonstrate two execution backends (MCP + computer-use), but the builder canvas — the surface the operator would point at to introduce the concept — exposes neither as a first-class node, AND the workflow the runbook tells the operator to click on (`customer_research_followup`) is structurally absent from the SPA's view of the world.**

Per the function-vs-dressing rule: F1 + F2 + F3 + F4 are all function-side blockers (the runbook's scripted clicks fail, and a fix means real backend wiring, not just a label edit). F5 + F6 are infrastructure issues that the demo recording's bypass-flag config currently hides — they would surface immediately if the operator switched to a "talk to the real gateway" recording profile.

Recommended order of operations for the operator before recording:
1. **Run the seed CLI**: `argo demo seed --tenant <demo>` against the running gateway, so `customer_research_followup` is reachable end-to-end.
2. **Add the missing SPA fallback row** for `customer_research_followup` to `web/lib/sample-workflows.ts` as a belt-and-braces — that way the card renders even if the seed CLI step is skipped.
3. **Decide whether Scene 4 install beat is on the cut-list**, or ship the InstallButton brief; if cut, rewrite the narration.
4. **Decide whether the Scene 5 narration name-checks the MCP and computer-use backends visually on the builder canvas** — if yes, ship the palette extension; if no, narrow the talk track to "Mariah's workflow runs both an API call and a desktop action — let's watch."
5. Rebuild + restart the gateway to fix the binary-vs-source drift on `/api/v1/workers`.
6. For recording, run with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=0` so the SPA hits the live gateway and any remaining drift surfaces on rehearsal not on take-1.
