# fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona`.
**Source**: ux-audit-2026-05-26-2214 `scene-1-and-2-claim-1-and-2.md` F2 [P0] + F3 [P0] + `scene-4-and-5-claim-4-and-5.md` F4 [P0] (substrate gap).

🔴 **P0 — Gateway returns `count=0` for `/api/v1/verticals/{v}/workflows` across all 6 demo verticals. Net consequence: the only Run-now button in the SPA is hard-disabled, no Scene-2 delegation chain can be triggered, no real agent-attributed audit rows exist (29/29 rows are HTTP self-traces), and Scene-2/4/5 cannot be recorded against real data. Root cause: the demo-seed CLI creates workers + skills but does not publish workflow_definition rows + does not fire one execution per persona at seed time.**

## Root cause

Per the audit's live curl walk:

```
curl -sS -H 'X-Argo-Principal: human:demo@acmeco.com' \
  http://127.0.0.1:7400/api/v1/verticals/gtm-engineer/workflows
→ {"count":0,"items":[],"vertical_id":"gtm-engineer"}
```

Same result for `people-ops`, `support`, `sales`, `finance`, `marketplace`. Zero published workflows across all 6 demo verticals.

`crates/argo-demo-seed` (the seed library introduced post #2272) has stages for:
- Pass A: tenant + workers
- Pass B: skills
- Pass C: workflows ← **this is where the gap lives**

Pass C either doesn't run, doesn't write `workflow_definition` rows, or writes them in a state that the per-vertical workflow-list endpoint doesn't surface. Need to confirm via `rg "workflow_definition" crates/argo-demo-seed/` which is currently empty according to the audit.

Downstream impact (all camera-visible):

- **Scene 2** "click Mariah's Schedules → Run now" — no Run-now is enabled, so the click can't fire (sibling brief `fix-workflow-detail-run-now-disabled-when-source-not-gateway` covers the SPA gate; THIS brief covers the data side).
- **Scene 2 talk-track** "inside 60 seconds the audit chain has the proof row" — no real proof row exists because no workflow has ever executed. The synthesised drill-in path is a function-side theatre violation (separate brief `fix-activity-row-audit-detail-synthesized-fixture-theatre`).
- **Scene 4** "search competitor pricing → click Install → workflow shows up on Mariah's roster" — even with PR #2407 landed (skill catalogue seed), the workflow that the skill plugs into doesn't exist.
- **Scene 5** "operator picks an MCP server and a CU step from the builder palette" — even if the palette gap fixes (`fix-scene-5-mcp-connector-catalog-to-builder-palette-bridge`) land, no published workflow exists to attach the steps to.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F2 [P0] — Gateway has zero workflows; Run-now button hard-disabled
- scene-1-and-2-claim-1-and-2 F3 [P0] — Audit feed has only HTTP meta-events; no agent-attributed rows
- scene-4-and-5-claim-4-and-5 F4 [P0] (data-side) — `Run now` button disabled because `source !== "gateway"` (gateway empty)
- whole-site F13 [P2] — Worker /duties + /schedules gateway routes 404 with no demo data behind them (partial — this brief seeds workflows; route brief covers the endpoint shape)

## Fix recipe

Pass C extension to `crates/argo-demo-seed`:

**Step 1 — Publish one `workflow_definition` per persona.**

For each canonical persona (Mariah / Peta / Priya / Jamal / Nori), insert a `workflow_definition` row keyed to the persona's vertical with a steps array that exercises the substrate appropriate to the persona:

- Mariah (marketplace-seller) → `customer_research_followup` workflow with `skill_invoke` + `tool_call(cu.run_macro)` (exercises Claim 5 macOS executor)
- Peta (project-coordinator) → `morning_pipeline_digest` workflow with `skill_invoke` + `delegate-task` to Patrick (exercises Claim 2 delegation)
- Priya (sales-sdr) → `qualify_lead` workflow with HTTP-tool step
- Jamal (people-ops) → `daily_check_in` workflow with email-notify step
- Nori (marketing-intern) → `draft_listing_copy` workflow with skill-invoke step

Each workflow must include a `name`, `vertical`, `description`, and `steps` array conforming to the live `workflow_definition` schema. Cross-reference `crates/argo-workflow/src/definition.rs` for the canonical struct.

**Step 2 — Fire one execution per persona.**

After publishing, invoke each workflow once via the in-process executor (or via `POST /api/v1/workflow-runs` using the seed runner's HTTP client). This produces:

- Real `audit.workflow_step_executed` / `audit.skill_invoked` rows on each persona
- Real `provenance.skill.id` + `provenance.model.id` + `provenance.hash_chain_anchor` for the camera to drill into
- A populated `decision_chain` for each row (`/audit/action/<id>` renders REAL content, not synthesised)

For executions that involve `cu.run_macro` (Mariah's `customer_research_followup`): the seed runner SHOULD bypass the macOS-perms gate in seed mode (or use a recorded macro fixture) so the seed can run on any host. If that's not feasible cleanly, skip the CU step in seed-time execution and document the gap — the substrate test `claim_5_live_mcp_and_cu` validates the executor separately.

**Step 3 — Verify post-seed.**

`argo demo seed --tenant <demo>` then:

```
curl -sS http://127.0.0.1:7400/api/v1/verticals/marketplace/workflows
# → count=1, items=[customer_research_followup]

curl -sS http://127.0.0.1:7400/api/v1/audit | jq '.items | map(select(.worker.name != "unknown"))'
# → 5+ rows with real worker names, real verbs, populated provenance
```

## Acceptance criteria

- [ ] `crates/argo-demo-seed/src/workflow.rs` (or equivalent) publishes ≥1 `workflow_definition` per canonical persona's vertical
- [ ] Each published workflow has ≥1 `skill_invoke` step (so audit rows include `skill_invoked` records)
- [ ] At least one persona's workflow includes a `delegate-task` call (exercises Claim 2 delegation chain)
- [ ] At least one persona's workflow includes a `cu.run_macro` step (exercises Claim 5 macOS substrate — may be elided in CI if perms unavailable)
- [ ] Seed runner fires one execution per persona at seed time
- [ ] Post-seed gateway probe: `/api/v1/verticals/{v}/workflows` returns ≥1 item for each of the 6 demo verticals
- [ ] Post-seed gateway probe: `/api/v1/audit` returns rows with `worker.name` matching the canonical roster + populated `decision_chain` + `provenance.skill.id`
- [ ] Rust integration test in `crates/argo-demo-seed/tests/` that asserts post-seed gateway state: workflows.count > 0 AND audit rows include non-`unknown` workers

## May-touch

- `crates/argo-demo-seed/src/workflow.rs` (NEW or extended, ~200 LOC for the 5 workflow definitions)
- `crates/argo-demo-seed/src/execute.rs` (NEW, ~100 LOC for the post-publish execution pass)
- `crates/argo-demo-seed/src/lib.rs` (~10 LOC wiring)
- `crates/argo-demo-seed/tests/post_seed_state.rs` (NEW, ~80 LOC)
- The 5 workflow TOML/spec files if those are externalised

## Out of scope

- The SPA gate `isLive = source === "gateway"` (covered by `fix-workflow-detail-run-now-disabled-when-source-not-gateway` / PR #2358)
- The MCP/CU step kinds in the builder palette (covered by `fix-scene-5-mcp-connector-catalog-to-builder-palette-bridge` + `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- The audit-row variety in the live feed (covered by `fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows`)
- The activity-row synthesised drill-in (covered by `fix-activity-row-audit-detail-synthesized-fixture-theatre`)
- Worker `/duties` + `/schedules` gateway routes (covered by `fix-gateway-worker-duties-schedules-routes-404`)

## Effort

~400 LOC across 3-5 files. ~3-4 hours. P0 because it's the single load-bearing unblock for Scene-2 + Scene-4 + Scene-5 real-data demos. Without this, the demo MP4 cannot show a real agent turn from a Run-now click anywhere in the SPA.
