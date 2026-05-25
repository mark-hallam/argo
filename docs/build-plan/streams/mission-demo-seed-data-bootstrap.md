# Stream mission-demo-seed-data-bootstrap — seed REAL workspace data via argo demo seed CLI

**Owner**: claude.
**Depends on**: none (independent of the SPA-side fixes; the seeded data flows through the same gateway endpoints the SPA already consumes).
**Authored against**: origin/main `109214a70` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-data-bootstrap`.
🔴 **P0 DEMO-GATE OPERATOR-LOCKED — FUNCTION-VS-DRESSING NO-THEATER REQUIREMENT.** Per the `🚨 NO THEATER IN THE DEMO` amendment in BUILD_STATUS.md header (operator decision 2026-05-24) **as CLARIFIED 2026-05-25T08:10Z** (memory `feedback_no_theater_in_demo` + reframe doc `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`): the v1.0 demo MP4 is POC + investor (not public release). The **function** of the agents must be real (real LLM calls, real MCP API round-trips, real computer-use automation, real audit emission for rows the camera drills into, real cross-tenant SQL). The **dressing** around the demo (social proof, fictional KPI numbers, customer logos, workspace names, historical filler audit rows the camera doesn't click) can be theatrical fixtures.

**This re-authored scope** (2026-05-25T08:15Z, conductor tick T07-40) relaxes the original 2026-05-24 spec which mandated ~750 real LLM-driven historical-filler audit rows. Under the clarified rule, that compute cost drops ~10× — only the rows the demo actually drills into need real-turn provenance.

## Scope

Add a real CLI subcommand `argo demo seed` that inserts the voiceover-aligned demo data into a real workspace via the real gateway primitives. **Two passes**:

**Pass A — FUNCTION-SIDE (real, no theater):**
- 5 named workers Mariah / Peta / Priya / Jamal / Nori (per `v1-mission-mvp-runbook.md` Scene 1 canon, NOT "6 workers + 2 more" from the original spec — see fix-brief `mission-demo-seed-worker-roster-canon-peta-included` for the open Option A/B decision on whether to include Patrick as a 6th). Created via existing `WorkerStore::create` with real vertical pack assignments. Persona text (role / headline / traits) sourced from `web/lib/sample-personas.ts` — fictional characters are dressing-layer, fine.
  - Mariah — Marketplace seller; Scene 1 narration anchor and audit drill-in worker.
  - Peta — Project Coordinator L2; first non-vertical PM cohort worker per memory `project_pm_cohorts_peta_patrick`.
  - Priya — Customer Success / recruiting-adjacent operator; Scene 5 narration anchor.
  - Jamal — Sales SDR / support-facing collaborator for Scene 4 marketplace-search beats.
  - Nori — fifth persistent identity card for the Scene 1 restart proof.
- 10-20 real agent turns spread across the workers that the demo navigates into — enough to populate (a) the audit-row drill-in beat in Scene 6, (b) the Slack-approval audit chain in Scene 2, (c) the workflow-execution audit rows in Scene 5 if Scene 5's workflow run is pre-staged rather than live-fired. Each is a real `AgentRunner::run_turn` call producing real `WorkerAction` + `audit_record` rows.
- The MCP+computer-use combo workflow `customer_research_followup` (per `mission-demo-seed-scene-5-zendesk-to-macos-notes-combo-workflow`) — definition inserted. Workflow execution at recording time is **live** (Scene 5 runs it on camera), not pre-baked.
- The Scene 4 marketplace skill catalog `competitor pricing scrape` + 2 siblings (per `mission-demo-seed-scene-4-competitor-pricing-scrape-skill`).
- Cross-tenant isolation: at least one foreign-tenant row in `worker`, `audit_record`, `skill_ir` — so Scene 6's "tenant_b sees nothing" beat queries real SQL and gets a real empty result.

**Pass B — DRESSING (theatrical, fixture labels OK):**
- Mariah's headline KPIs (3 listings live / 11 buyer replies / 2 items flagged below floor) — these can be **theatrical labels** sourced from fixture data, NOT from real LLM turns. Camera doesn't drill into individual contributing turns.
- Historical audit-row filler — default **100-200 rows** (was 1000+ under the original spec) inserted with realistic timestamps spread across the past 14 days. Each is a valid `audit_record` row with real schema but **the payload can be synthetic** as long as the row is internally consistent (worker_id real, tenant_id real, payload structurally valid JSON). Rows the demo specifically drills into are excluded from this set and produced by Pass A's real turns.
- KPI counters surfaced on `/workers` cards — derived from the audit-row count (so the displayed number reflects real DB SUM even when underlying rows are dressing-tier).
- Workspace name, account names, fictional company stats: theatrical strings, fine.

The result: when the demo opens `/team`, the data shown is from real DB queries against a real workspace. When the operator clicks Approve on Slack, a real `decide_request` SQL UPDATE fires. When the demo clicks a row in the audit feed and drills in, that row is from Pass A (real turn). When the demo lingers on Mariah's KPI strip but doesn't drill, the displayed numbers are dressing-tier — fine.

This is a **substantive backend addition** — ~400-700 LOC in `argo-cli` + new `crates/argo-demo-seed/` library crate (was 600-1000 LOC under the original spec; the relaxed scope shaves ~30%). NOT a fixture file edit. NOT a TypeScript change.

## Parallelism

- Manager-worker allowed: yes
- Max children: 3
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: docs/build-plan/work-packages/mission-demo-seed-data-bootstrap.md (manager creates from PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md)
- Shared hotspots: [`crates/argo-cli/src/cli.rs` (one new subcommand variant)]

## Files you may touch

- `NEW: crates/argo-demo-seed/**` — new library crate hosting the seed orchestration. Takes a `tenant_id` + a `seed_profile` (default = "v1-demo-canonical") and orchestrates worker creation + agent turn execution + Slack channel registration.
- `crates/argo-cli/src/cli.rs` — narrowly scoped: add `DemoCmd::Seed { ... }` variant.
- `NEW: crates/argo-cli/src/handlers/demo_seed.rs` — CLI handler invoking `argo-demo-seed::run_canonical_v1_seed(...)`.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: add `mod demo_seed;` line.
- `Cargo.toml` — narrowly scoped: add workspace member `crates/argo-demo-seed`.
- `web/lib/sample-personas.ts` — read-only INPUT after `mission-demo-seed-worker-roster-canon-peta-included` lands: the seed CLI reads the `DEMO_SEED_PERSONAS` spine (name + role + traits + headline + vertical_id) from this file and inserts those as real worker rows. The runtime mock roster remains separate and must not be used as rendered demo truth.
- `docs/skycot/operator/demo-runbook.md` — add a pre-flight bullet: "Run `argo demo seed --tenant <demo-tenant-id>` before the recording session. Verify `/team` shows 5 workers via curl http://127.0.0.1:7400/api/v1/team and the count matches."
- `scripts/demo-dry-run.sh` — narrowly scoped: drop the SAMPLE_TEAM mock paths from the dry-run scene assertions. Scenes now verify real backend rows.

## Files you must NOT touch

- `web/lib/sample-personas.ts` for ANY edit after the roster-canon pre-brief has landed (read-only input)
- `web/lib/sample-*.ts` other than to confirm they are NOT used at runtime when seeded data is present
- `crates/argo-store/src/migrations/**` — no new migration is in scope; the seed uses existing tables/columns
- Anything in `crates/argo-skill/` or `crates/argo-orchestrator/` other than call-site invocations through existing public APIs

## Anti-theater acceptance criteria

The PR must demonstrate that:
- [ ] **(Function-side)** After `argo demo seed --tenant <demo>`, `curl http://127.0.0.1:7400/api/v1/team` returns 5 real `Worker` rows (Mariah/Peta/Priya/Jamal/Nori) with ULID worker IDs.
- [ ] **(Function-side)** After seeding, `curl http://127.0.0.1:7400/api/v1/approvals` returns ≥3 real pending `approval_request` rows, each linked to a real `WorkerAction` row, each WorkerAction created by a real agent turn (audit-traceable). The approval rows the camera DRILLS INTO must be real-turn-backed; additional dressing-tier pending rows are OK as long as no demo scene clicks them.
- [ ] **(Function-side)** After seeding, `curl http://127.0.0.1:7400/api/v1/audit?limit=100` returns at least 10 real `audit_record` rows that the demo's drill-in beats land on (Scene 6 hash-chain hover, Scene 2 Slack-approval audit chain), each with `payload` from a real `AgentRunner::run_turn`. Additional 100-200 dressing-tier rows OK to populate the audit-feed list visually; their payload can be synthetic JSON as long as the row is internally consistent (real `worker_id` + real `tenant_id` + valid schema).
- [ ] **(Dressing-side)** Mariah's `/workers/{id}/overview` page KPI numbers (listings live, buyer replies, flagged-below-floor) are populated — values can be theatrical fixture labels or DB-derived; whichever is simpler. The demo does NOT drill into individual contributing audit rows from this card, so these numbers are dressing-tier.
- [ ] **(Function-side)** Cross-tenant isolation: at least one foreign-tenant row in `worker` + `audit_record` + `skill_ir`. Scene 6's "tenant_b sees nothing" beat queries real SQL and gets a real empty result for the demo tenant's view of tenant_b's rows.
- [ ] The dry-run script's all-8 scenes pass when `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` is UNSET — i.e. the SPA is talking to the real backend.
- [ ] PR body includes a transcript of `argo demo seed --tenant <id>` showing the 10-20 real agent turns running + the dressing-tier rows being inserted (clearly labeled as Pass A vs Pass B).
- [ ] PR body includes the vacuous-pass guard (round-37 pattern): negative test asserting a DIFFERENT tenant sees ZERO rows AND the seeded tenant sees N≥5 workers (positive baseline).
- [ ] `cargo clippy -p argo-demo-seed -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-demo-seed` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-demo-seed-data-bootstrap`
- PR title: `demo: argo demo seed CLI for real-backend v1.0 demo workspace (mission-demo-seed-data-bootstrap)`

## Notes for the implementer

- **This brief was REWRITTEN 2026-05-24 from Option A (mock-bypass fixtures) to Option B (real backend seed) per the original `🚨 NO THEATER IN THE DEMO` amendment, then RE-AUTHORED 2026-05-25T08:15Z under the operator's function-vs-dressing clarification (memory `feedback_no_theater_in_demo`; reframe doc at `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`).** Do not revert to the mock-bypass approach for function-side data. Theatrical fixtures for dressing-side data are explicitly allowed.
- **Pass A (function-side) agent turns** MUST hit real LLMs. Use a sandbox provider key configured in the seed profile. Fail closed if no real provider is configured for Pass A turns.
- **Pass B (dressing-side) rows** can be inserted via direct `INSERT INTO audit_record VALUES (...)` with synthetic-but-valid payload JSON. That's NOT theater under the clarified rule because the camera doesn't drill into these rows.
- For Mariah's persona (marketplace seller), use a sandbox Facebook Marketplace API or a stub that emits realistic action payloads with `is_sandbox: true` metadata — Pass A turns are real but the external side effects are bracketed.
- Default counts under the relaxed rule: **10-20 Pass A real turns + 100-200 Pass B dressing rows** (was 1000+ all-real under the original spec).
- Manager-worker mode allowed because the work spans:
  - (1) the new `argo-demo-seed` crate
  - (2) the CLI handler wiring
  - (3) the runbook + dry-run updates
- Coordinate with `fix-web-mock-bypass-env-var-rename` — once that merges and the bypass mode is fixed, demo recording USES this seed CLI + the bypass mode DEFAULT-OFF (or removed entirely, operator's call).
- The pattern is similar to how SaaS apps "seed a demo account on signup" — Stripe, Notion, Linear all do this. Look at how OSS projects like Cal.com or Plane seed their demo workspaces for prior art.
