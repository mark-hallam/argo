# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Date**: 2026-05-26 14:41 (Melbourne)
**Auditor**: e2e-ux-journey-test-agent (read-only)
**Scope**: `/skills`, `/skills/[skillId]`, `/workflows`, `/workflows/<v>/<n>`, `/workflows/build`, `/workflows/<v>/__new__/build`; mapped against `docs/skycot/demos/v1-mission-mvp-runbook.md` Scenes 4 + 5.
**Backends observed**: gateway live on :7400, returning `skills: []`, `workflows: []`, and per-vertical 401-unauthorized. SPA on :3000. SPA falls through to seed (`SEED_SKILLS`, `SAMPLE_WORKFLOW_SUMMARIES`).
**Reference**: function-vs-dressing rule per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.
**Prior audit**: `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` (same scope, ~6 hours older). Re-verified rather than reproduced; this report calls out what changed + what's new since.

---

## TL;DR

Five P0 demo-blockers from the prior audit (2026-05-26-1028) **all still stand** — none of the function-side gaps have closed in the six hours since. The only Scene-4/5-adjacent commit in that window is #2349 (substrate chips on workflow cards + detail), which is visually nice but adds zero data-path or step-kind coverage. The runbook's Scene 4 search beat ("competitor pricing scrape → 3 results → Install"), Scene 5 workflow (`customer_research_followup` MCP+computer-use combo), and Scene 5 Run beat are all unrecordable in the current state. Two new findings this pass: a gateway CORS allowlist pinned to `http://localhost:3000` (any port-shifted dev fails silently with `net::ERR_FAILED` — surfaced when the Claude Preview MCP autoPort'd to 62927), and a typo in `.claude/launch.json` (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` is missing the `ARGO_` prefix the code expects, so the bypass branch never fires; the demo accidentally works because of the empty-gateway → seed-fallback path, not the bypass it claims to use). Three P0 (all carried), four P1 (two carried, two new), two P2.

## Headline blockers (carried from 2026-05-26-1028, all still failing)

1. **P0 — F1 (carried)**: `/skills` search "competitor pricing scrape" returns 0 results — `SEED_SKILLS` has 6 generic entries, none match.
2. **P0 — F2 (carried)**: No Install button on `/skills` or `/skills/[skillId]` — Scene 4 step 3 has no UI to land on.
3. **P0 — F3 (carried)**: `customer_research_followup` workflow doesn't exist in any seed file; no seed workflow has both an MCP step and a computer-use step.
4. **P0 — F4 (carried)**: Builder palette has no `mcp_call` or `computer_use` step kind. Scene 5's MCP-vs-CU narration has no on-canvas surface.
5. **P0 — F5 (carried)**: `/workflows/<v>/<n>` "Run now" hard-disabled when `source !== "gateway"`; live gateway is empty, so it's always disabled in the recording environment.

---

## What changed since 2026-05-26-1028

- **#2349 (fix(workflows): surface execution substrate chips)** added `SubstrateChip` to the workflow card meta row + detail page + the seed-substrate fields on the workflows API client. Visual progress only — the chips render (`macos / linux / windows / e2b / browser / headless`) but the underlying step kinds (F4) and the data shape (F3) are unchanged. All seed workflow text still infers substrate=`headless` (verified — none of the 7 seeds mention `macos`, `cu_`, `notes app`, `browser`, `sandbox`, or `e2b`).
- No edits to `web/lib/sample-workflows.ts`, `web/lib/api/skills.ts`, `web/lib/workflow-templates.ts`, or `web/app/(app)/skills/**` since the prior audit.

## All findings, in order

### Scene 4 — Skills marketplace + install + worker provenance

#### F1 [P0] — Search "competitor pricing scrape" returns 0 results (CARRIED)

State as of 14:41: unchanged from 2026-05-26-1028. `SEED_SKILLS` (`web/lib/api/skills.ts:55-116`) still has 6 entries — `approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich` — none with `competitor`, `pricing`, or `scrape` tokens. `TRY_CHIPS` (`web/app/(app)/skills/page.tsx:23-30`) still doesn't include the runbook phrase. Live gateway `/api/v1/skills` returns `{skills: [], total_count: 0}` so the page lands on seed-fallback.

- Repro: open `/skills`, type "competitor pricing scrape" → empty-state ("No skills match …").
- File: `web/lib/api/skills.ts:55` + `web/app/(app)/skills/page.tsx:23-30`.
- Suggested fix (unchanged): either rewrite the runbook to use one of the 6 seed phrases (cheapest), or add the runbook-named skills to `SEED_SKILLS` + a matching TRY chip.

#### F2 [P0] — No Install button anywhere in `/skills` or `/skills/[skillId]` (CARRIED)

State unchanged. `/skills/[skillId]/page.tsx` (re-read at 14:39) still renders only Provenance + Workers-using + "← Back to the marketplace". No install affordance, no install hook in `lib/hooks/use-skills.ts`, no install helper in `lib/api/skills.ts`. The only "install" hit across `web/app/(app)/skills/` is the literal "No skills installed yet" empty-state copy (`page.tsx:171`). The recent #2374 ("replace View-source toast stub with /skills/<name> link") shipped on the worker-skills card, not the marketplace.

- Repro: open `/skills/approval.route` → no Install button visible; grep `web/app/(app)/skills/` for `install|Install` returns the one empty-state hit.
- File: `web/app/(app)/skills/[skillId]/page.tsx:38-122`.
- Suggested fix (unchanged): brief to add `installSkill()` mutation + progress-strip overlay ("Verifying provenance… Installing skill… Available to workers").

#### F6 [P1, NEW] — `/skills/learned-this-month` returns 404; "Skills my workers learned this month" filter chip silently mocks

Gateway returns `{"error":"not found"}` for `GET /api/v1/skills/learned-this-month` (verified `curl -H "Origin: http://localhost:3000"` at 14:38). The SPA client (`web/lib/api/skills.ts`, the `learnedQuery` consumer in `page.tsx:55`) treats 404 as "fall back to mock" so the filter chip works visually, but the camera-visible state is "I clicked 'learned this month' and skills filtered" without any real cross-tenant learning data underneath. Function-side per `feedback_no_theater_in_demo` — but Scene 4's pass-cue doesn't drill into this filter, so it's P1 not P0.

- File: gateway side (`crates/argo-cli/src/handlers/gateway/routes_skills.rs` or sibling) — no implementation. Web: `web/lib/api/skills.ts:232-280` (fetchLearnedThisMonth).
- Suggested fix: gateway implements the endpoint OR the SPA hides the chip when source === "mock-offline". Smaller surface to fix than adding the gateway feature.

#### F7 [P2, carried as F6 in prior audit] — Skill-detail page shows zero workers / zero adoption in mock-offline

`fetchSkillUsage` falls to `seedUsage()` which returns `workers_using: []`, `adoption_count: 0`. Card reads "No workers are using this skill yet" + "Used by **0** workers across the SKYCOT marketplace." Scene 4 doesn't drill into a skill detail page, so dressing-side. Worth seeding to ~2-3 workers + an adoption count >0 to avoid the empty look if the operator wanders.

- File: `web/lib/api/skills.ts:187-197`.

---

### Scene 5 — Workflows: MCP + computer-use combo

#### F3 [P0] — `customer_research_followup` workflow does not exist (CARRIED)

State unchanged. `web/lib/sample-workflows.ts` still has 7 entries: `post_listing`, `screen_candidate`, `approve_refund`, `qualify_lead`, `triage_ticket`, `morning_pipeline_digest`, `clean_customer_list`. `web/lib/workflow-templates.ts` has a `competitor_intel_digest` but no `customer_research_followup`. Live gateway `/api/v1/workflows` returns `{workflows: [], total_count: 0}` (verified 14:33). No workflow on disk or in the database has both an `mcp_call` step and a `computer_use` step.

- Repro: `curl http://localhost:3000/workflows` → no card named `customer_research_followup`. `grep -rn 'customer_research' web/lib/` → no hits.
- File: `web/lib/sample-workflows.ts:65-138`.
- Suggested fix (unchanged): depends on F4 (step kinds) landing first.

#### F4 [P0] — Builder palette + WorkflowStepKind has no `mcp_call` or `computer_use` (CARRIED)

State unchanged. `BuilderSidebarPalette` palette (verified live at 14:34 by curling `/workflows/sales/__new__/build` and grepping `palette-item-` data-testids): triggers (manual/schedule/webhook/email/slack/form), logic (decision/loop/wait), skills (skill-invoke), tools (browser/code/http), approvals (web/slack/email), notify (slack/inbox/email). **No MCP entry. No computer-use entry.** The closest semantic match is `tool-browser` ("Drive a real browser") but it has no MCP-pool routing semantics on the canvas.

- Repro: open `/workflows/sales/__new__/build` → palette has no MCP or computer-use tile.
- File: `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderSidebarPalette.tsx` + `types.ts:22-34, 236-286`.
- Suggested fix (unchanged): extend `WorkflowStepKind` union with `mcp_call` and `computer_use`; add palette tiles under **Tools** group; add `STEP_KIND_META` entries on detail-page viewer + run overlay so MCP-tuple and CU-cue render at runtime.

#### F5 [P0] — Detail page "Run now" disabled when source !== "gateway" (CARRIED)

State unchanged. `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:105` still computes `isLive = source === "gateway"`, line 283 still has `disabled={running || !isLive}`, tooltip on line 287 still reads *"Available once this workflow is published to the gateway."* The live gateway returns 0 workflows → all detail pages load seed-source → button always greyed out. The build-canvas test-run path is separate and the runbook's "hit Run" beat references the catalog detail page, not the builder.

- Repro: `/workflows/sales/qualify_lead` → "Run now" button visibly disabled.
- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:103-105, 283-290`.
- Suggested fix (unchanged): seed the live gateway with the canonical demo workflows (`argo demo seed`-style) before the recording, OR relax the gate to allow seed-source runs that emit a real handler-fired audit row. Cleaner route is seeding the gateway because the NO-THEATER rule wants the actual MCP + CU executors firing.

#### F8 [P1] — Workflow card "by Rio" + "by Sam" violate Scene-1 worker-roster canon (CARRIED, was F10)

State unchanged. `web/lib/sample-workflows.ts`: `qualify_lead` lastEditor="Rio" (line 103), `morning_pipeline_digest` lastEditor="Rio" (line 124), `clean_customer_list` lastEditor="Sam" (line 135). Canonical roster per Scene 1 + memory `project_workers_page_design_locked` is **Mariah / Peta / Priya / Jamal / Nori**. Rio and Sam are neither. The card meta row renders "by Rio" / "by Sam" wherever those workflows surface on /workflows (verified by reading `page.tsx:267` `<span className="workflow-card-meta-item">by {wf.lastEditor}</span>`).

- Repro: `/workflows` → search "qualify" → card meta shows "by Rio".
- File: `web/lib/sample-workflows.ts:103, 124, 135`.
- Suggested fix (unchanged): change to one of `Mariah / Peta / Priya / Jamal / Nori`. Three single-word edits.

#### F9 [P1, NEW] — Per-vertical workflow endpoint returns 401 unauthorized; SPA silently masks the error

`curl http://127.0.0.1:7400/api/v1/verticals/sales/workflows -H "Origin: http://localhost:3000"` returns `{"error":{"code":"unauthorized","message":"X-Argo-Principal header is required"}}` (verified 14:36). `fetchAllWorkflows` (`web/lib/api/workflows.ts:113-128`) catches 401 as `isRecoverable` and returns empty bucket, then falls through to `SAMPLE_WORKFLOW_SUMMARIES` as `source: "mock-offline"`. So the page renders fine but every workflow card is mock data — the camera looking at /workflows is seeing fixtures. The demo banner (F11) does flag this with "showing sample data", but the cards themselves look real (substrate chips, risk labels, version pills, last-edited timestamps) so the function-side claim is undermined.

- Repro: with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` unset (current launch.json state — see F12), every per-vertical fetch 401s.
- File: gateway side (handler in `crates/argo-cli/src/handlers/gateway/`) — principal injection should fire when `ARGO_GATEWAY_AUTH_BYPASS=1` is set server-side. Verify whether the gateway is actually running with that flag.
- Suggested fix: confirm gateway booted with `ARGO_GATEWAY_AUTH_BYPASS=1`; if it did, the auth middleware is skipping that flag for this route. Quickest mitigation for the demo: real session cookie via signin → seed gateway with workflows → run from that authenticated tab.

#### F10 [P1, NEW] — Gateway CORS allowlist pinned to `http://localhost:3000` only; any port-shifted dev fails silently

CORS preflight (`OPTIONS /api/v1/skills`) returns `Access-Control-Allow-Origin: http://localhost:3000` for origin=3000, but **no `Access-Control-Allow-Origin` header at all** when origin=`http://localhost:62927` (the port Claude Preview MCP autoPort'd to). The browser then surfaces every gateway call as `net::ERR_FAILED` and the page sits on its skeleton or empty-state. Confirmed by reading the failed-request log in the wedged preview window: ~25 gateway calls in 30 seconds, every one `[FAILED: net::ERR_FAILED]`. Doesn't affect the demo recording (operator records on :3000), but is a footgun for anyone running the SPA on a different port (CI, sandbox, ngrok, second dev browser). P1 not P0 because the demo path is unaffected.

- Repro: `curl -sI -H "Origin: http://localhost:9999" -X OPTIONS http://127.0.0.1:7400/api/v1/skills` → no ACAO header.
- File: gateway CORS config (likely `crates/argo-cli/src/handlers/gateway/cors.rs` or similar).
- Suggested fix: allow `http://localhost:*` or `http://127.0.0.1:*` in dev (NODE_ENV !== production). Keep prod whitelist strict.

#### F11 [P1] — Demo banner "showing sample data. Sign in to see your own workspace" reads on camera (CARRIED, was F13)

State unchanged. `web/components/skycot/DemoBanner.tsx:44-46` renders the same copy. Fires whenever `NEXT_PUBLIC_SKYCOT_DEMO_MODE === "1"` (which the launch.json sets). The banner is dressing-side, but its **copy** ("sample data … Sign in to see your own workspace") explicitly tells the viewer the rest of the screen is fake — which clashes with the function-side narration the operator delivers.

- Suggested fix (unchanged): record with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=0` after seeding the live gateway. Until then, rewrite the banner copy to something investor-safe like "Demo workspace — Skycot's hosted POC. [Talk to sales]".

#### F12 [P1, NEW] — `.claude/launch.json` env-var typo: `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` should be `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`

`.claude/launch.json` line 8 sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`. Both the middleware (`web/middleware.ts:39`) and the API clients (`web/lib/api/skills.ts:128`, `web/lib/api/workflows.ts:111`) check `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (with the `ARGO_` prefix). The launch.json value is silently ignored. The dev workflow accidentally works because:
1. `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` triggers the middleware demo-bypass branch (line 51).
2. Gateway returns empty data → SPA falls to seed.
Neither path is the bypass the comment in the code documents ("no network call"). The SPA makes a network call, gets 0 results, falls to seed. The audit confirms this by counting failed network requests in `preview_network` output.

- Repro: with current launch.json, `/skills` page WILL fetch `GET /api/v1/skills` and get 200 with empty array, then mock-fallback. The `mock-bypass` source label is never set.
- File: `.claude/launch.json:8`.
- Suggested fix: rename the env var in launch.json to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. One-word fix.

#### F13 [P2] — Workflow card lastEditor "Sam" is also off-canon (covered in F8, mentioned again because two of the 7 seed editors are off-canon — Rio twice, Sam once — and the audit thought it was one)

(Roll-up of F8 detail; no separate finding.)

---

### Cross-cutting

#### F14 [P1] — Gateway audit feed has no `SkillInvoked` or `workflow.step.executed` rows; Scene 4 + 5 pass-cues won't fire (CARRIED, was F15)

`curl http://127.0.0.1:7400/api/v1/audit?limit=5` at 14:52 returns only `audit.detail.viewed` rows from this audit's own browsing. Scene 4 pass-cue: *"a `SkillInvoked` audit row is written carrying the same provenance."* Scene 5 pass-cue: *"two `workflow.step.executed` rows for the run, one per backend."* Without F2 (Install flow firing the runtime) and F5 (Run button firing the runtime), no such rows are emitted. Even if F2 + F5 land in the SPA, the runtime + gateway audit emitter need to produce the expected `verb` strings. Verifiable only end-to-end.

#### F15 [P2] — Workflow detail "Open builder / Edit in chat / Version history / Add as Schedule / Run now" action row dilutes the Run beat (CARRIED, was F14)

5 CTAs in a row; "Run now" is fifth. During Scene 5's "Hit Run" beat, the operator's eye lands on four other buttons first. Polish-grade — not blocking.

- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:239-282`.

---

## Resolved since 2026-05-26-1028

None of the prior P0s. The substrate-chip PR #2349 added visual signal on workflow cards + detail (the chips for `macos`/`linux`/`browser`/`e2b`/`headless` now render), but the underlying step-kind union and seed data are unchanged, so it doesn't move any of the function-side blockers.

## Orientation score

**38 / 100.** Unchanged from 2026-05-26-1028. The Scene 4/5 function surface hasn't moved in the last 6 hours. The five P0s are the same five. New observations this pass (F9 401-mask, F10 CORS port-pin, F12 env-var typo) are quality-of-engineering points, not demo-recording blockers — they don't move the score. To take Scenes 4 + 5 from unrecordable to recordable, the shortest path is:

1. Seed the live gateway with the runbook-named skills + workflow (`argo demo seed` extension).
2. Add `mcp_call` + `computer_use` step kinds + palette tiles + viewer chips (F4).
3. Add `installSkill()` mutation + progress strip on `/skills/[skillId]` (F2).
4. Drop the `isLive` gate on the detail page Run button (F5).
5. Fix the three off-canon editor names (F8) — single-line.

That's roughly one engineer-workday of focused brief work, contingent on a runtime that actually emits the expected `verb` strings on the audit chain (F14). Until then, the cleanest mitigation is to **rewrite the Scene 4 + 5 runbook beats** to match what currently ships: replace "competitor pricing scrape" with "deal qualification", swap `customer_research_followup` for `qualify_lead` (which already has a 5-step approval-required flow), and drop the MCP+computer-use combo callout from Scene 5 until F4 lands.
