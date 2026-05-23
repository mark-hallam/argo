# Workflow e2e coverage roadmap — post-v1.0

- **Date**: 2026-05-23 (round 35, scheduled-task evaluator)
- **Ship gate**: v1.0 demo MP4 (target moved forward — recording **2026-05-24**)
- **Scope**: end-to-end test coverage for the workflow surface beyond what shipped with v1.0
- **Status**: roadmap proposal; operator-decisional whether to promote to `docs/build-plan/`

## Where we are at v1.0 ship

| Layer | e2e proof | Status |
|---|---|---|
| Workflow execution engine (Rust) | 15 tests across 5 files in [`crates/argo-workflow/tests/`](crates/argo-workflow/tests/) | 🟢🟢 LIVE |
| Gateway execute route `POST /api/v1/verticals/{v}/workflows/{n}/execute` | [`routes_workflow_execute.rs`](crates/argo-cli/src/handlers/gateway/routes_workflow_execute.rs) | 🟢 SHIPPED |
| Duty cron → agent runner (Claim 2) | [PR #2137](https://github.com/mark-hallam/skycot/pull/2137) (merged 2026-05-23) | 🟢🟢 LIVE |
| Genesis → vertical → workflow (Claim 3) | [PR #2109](https://github.com/mark-hallam/skycot/pull/2109) (merged 2026-05-23) | 🟢🟢 LIVE |
| MCP cap+audit+pool (Claim 5) | [PR #2112](https://github.com/mark-hallam/skycot/pull/2112) (merged 2026-05-23) | 🟢🟢 LIVE |
| Cross-tenant audit chain (Claim 6) | [PR #2113](https://github.com/mark-hallam/skycot/pull/2113) (merged 2026-05-23) | 🟢🟢 LIVE |
| SPA workflow builder (`/workflows/[v]/[n]/build`) | round-25 Chrome walkthrough | 🟡 2 bugs queued |

**The substrate is honest.** Workflows run, tenants stay isolated, audit chains hold. Demos are recordable.

## What v1.0 doesn't yet prove

Seven test classes that the current 15 tests don't touch. Each represents a real customer scenario.

| # | Test class | What it proves | Why it matters |
|---|---|---|---|
| 1 | **Durability** — workflow survives gateway restart | A 6-hour workflow doesn't lose its place when the gateway is updated or the machine reboots | Customers won't trust workflows that lose state on a routine deploy |
| 2 | **Branching DAGs** — "if step 3 says X, do A; else do B" | A workflow can fan out into parallel paths, fan in to a merge step, or branch on a runtime decision | Today's workflows are linear; real business logic branches |
| 3 | **Computer-use executor inside a step** — workflow step opens a Mac app + clicks | The cu-macos executor is reachable from a workflow, not just standalone | This is the "any PC task" half of Claim 5 — currently only the MCP half is proven |
| 4 | **Multi-worker handoff** — workflow hops from worker A to worker B | Mariah qualifies a lead → hands to Sam to write the proposal → hands to Patrick to log the deal | This is the load-bearing "team of workers, not solo worker" claim |
| 5 | **Anomaly + circuit-breaker** — workflow auto-pauses when cost spikes 10x | Customers don't get burned by runaway $ spend | Without this, every customer needs a 24/7 babysitter |
| 6 | **Skill A/B inside a step** — same step runs skill v1 (50%) + skill v2 (50%) | The platform actually gets better over time | This is what makes the system worth more in month 6 than in month 1 |
| 7 | **Policy-as-code gates** — step pauses if it touches a PII (personally-identifiable info) field | Workflows respect compliance rules without a human reviewer in the loop for every action | Required for HIPAA (US health) + GDPR (EU privacy) + SOC2 (US enterprise audit) industries |

## Recommended phasing — post-v1.0

Ordered by **cheapest + highest customer-trust signal first**, not by `BUILD_PLAN_V11.md` phase number. The build plan is right that v1.1 reliability comes before everything else; this ordering refines which reliability investments unlock the most trust the fastest.

### Phase 1 — weeks 1-4 post-ship (the cheap wins)

| Investment | Brief shape | LOC est | Customer signal unlocked |
|---|---|---|---|
| Workflow + computer-use executor stitched together | `eval-workflow-step-invokes-computer-use-executor` (P1, ~150 LOC test only — the executor exists, the wire is the work) | ~250 | Demo can show "workflow opens Mariah's Slack and posts the lead summary in her own voice" — the "any PC task" half of Claim 5 lands |
| 3 demo-ops briefs (already queued) | `demo-ops-add-magic-link-bypass-for-recording` + `docs-demo-runbook-add-skycot-db-key-section` + frontend `fix-workflow-build-layout-default-export` | ~120 | Demo recording is bulletproof on a fresh machine |
| Workflow checkpoint-resume contract test | `eval-workflow-survives-gateway-restart` (P1, ~200 LOC) — kill gateway after step 3, restart, assert step 4 picks up + audit chain unbroken | ~200 | First customer sleep-test — "I deployed and it didn't lose state" |

### Phase 2 — weeks 4-12 (the trust-building wave)

| Investment | Brief shape | LOC est | Customer signal unlocked |
|---|---|---|---|
| Branching DAG e2e | `eval-workflow-3-way-branch-on-step-result` (P1, ~250 LOC) | ~250 | "Workflows do real business logic, not just linear scripts" |
| Anomaly + circuit-breaker | `claude-anomaly-detector` + `codex-circuit-breaker` + `claude-spend-anomaly-alerts` (already in v1.1 plan) | ~600 across 3 briefs | "I don't need to babysit it" — the #1 enterprise procurement question |
| Trajectory replay UI | `claude-trajectory-replay-ui` (v1.1) — visual replay of any workflow run | ~800 | Post-incident debugging without grepping audit_record |

### Phase 3 — months 3-6 (the team-of-workers wave)

| Investment | Brief shape | LOC est | Customer signal unlocked |
|---|---|---|---|
| Worker roster + task distribution | `claude-worker-roster` + `claude-task-distribution` (v1.3) | ~1500 across 2 briefs | "Workers actually work as a team" — load-bearing for the brand promise |
| Multi-worker handoff e2e | `eval-workflow-hands-off-worker-a-to-worker-b` (P1, ~300 LOC) | ~300 | Sales SDR → Customer Support handoff demo becomes recordable |
| New executor kinds | `claude-filesystem-executor` + `claude-code-execution-executor` + `claude-document-creation-executor` (v1.2) | ~2400 across 3 briefs | "Any PC task" becomes true for more task shapes |

### Phase 4 — months 6-12 (the enterprise wave)

| Investment | Brief shape | LOC est | Customer signal unlocked |
|---|---|---|---|
| Skill A/B test framework | `claude-skill-versioning-ab-test` (v2.0) | ~600 | "The system gets measurably better over time" — load-bearing for renewal |
| Policy-as-code | `claude-policy-as-code` + `claude-data-classification` (v2.1) | ~1200 across 2 briefs | HIPAA / GDPR / SOC2 customers can actually buy |
| Compliance-grade audit reports | `codex-audit-report-cli` (v2.1) | ~400 | "We can answer the auditor's questions" — required for procurement |
| Demo verticals 2-5 | `codex-vertical-customer-support` + `-sales-sdr` + `-marketing-intern` + `-finance-ar` (v2.2) | ~3000 each | Proves the platform claim "ANY role" with 4 more data points |

## Specific brief shapes for the Phase 1 wins

The 3 cheapest investments to do **right after the demo MP4 ships** — these unlock customer trust faster than the broader v1.1 work.

### `eval-workflow-step-invokes-computer-use-executor` (P1, ~150 LOC test only)

The cu-macos executor exists at [`crates/argo-executor-computer-use-macos/`](crates/argo-executor-computer-use-macos/) and has 1 smoke test gated on `ARGO_NATIVE_MACOS_INTEGRATION` env. The workflow execution engine exists at [`crates/argo-workflow/src/runner.rs`](crates/argo-workflow/src/runner.rs). No test stitches the two together.

The test class: a workflow definition with a step `kind: "computer-use"` → the runner dispatches to the cu-macos executor → executor runs (or is stubbed for CI) → step's output flows into the next step → audit chain holds.

### `eval-workflow-survives-gateway-restart` (P1, ~200 LOC)

A 5-step workflow → kill the gateway after step 3 lands its audit row → restart the gateway → resume the workflow → assert step 4 picks up where step 3 left off → assert the audit chain's parent_hash field still validates end-to-end. Today this would fail because workflow state lives in process memory between dispatches; the brief author has to choose between (a) persisting the in-flight DAG cursor to `workflow_run` rows, or (b) requiring workflows to be designed as resumable from any step (the simpler-but-restrictive path).

### `eval-workflow-3-way-branch-on-step-result` (P1, ~250 LOC)

The DAG (Directed Acyclic Graph) code at [`crates/argo-workflow/src/dag.rs`](crates/argo-workflow/src/dag.rs) supports branching but the e2e tests don't exercise it. The test class: a workflow definition with step 2's `on_result` mapping to 3 distinct step IDs → run with input that triggers branch A → assert only branch A's audit rows exist + workflow_run carries `chosen_branch="A"` → repeat for B + C.

## Demo recording — what's in the way of shipping tomorrow

Round-30 Chrome walkthrough proved the demo IS recordable today; what's missing is **operator-recording ergonomics**. Two queued briefs close it:

1. **`demo-ops-add-magic-link-bypass-for-recording`** (P2, ~80 LOC) — env-var-gated `ARGO_DEMO_MODE=1` dev signup that mints a verified `human:<email>` principal without an SMTP (email-sending) round-trip. Without this, demo Scene 1 (signup → magic-link verify) requires a real email loop or manual JWT (JSON Web Token — a signed credential) minting.
2. **`docs-demo-runbook-add-skycot-db-key-section`** (P2, ~20 LOC) — adds the `SKYCOT_DB_KEY_BASE64` (database encryption key) bootstrap line to the runbook §5; without it, the gateway refuses to start on a fresh machine because it can't decrypt the SQLite (lightweight file-based database) store.

Frontend bug `fix-workflow-build-layout-default-export` (P1) does NOT block the demo per round-25 walkthrough (the page renders fine; the console error is intermittent + non-fatal). Defer to Phase 2.

## Recommendation

**Ship the demo MP4 tomorrow** with the 2 demo-ops briefs landed tonight. Treat Phase 1 of this roadmap as the **immediate post-ship work** — the 3 briefs (cu-executor stitched into workflow + checkpoint-resume + branching DAG) are the cheapest investments that unlock the biggest customer-trust signals, and they're all in the workflow surface that v1.0 already proved at the substrate level.

The remaining post-v1.0 phases line up with the existing [`BUILD_PLAN_V11.md`](docs/build-plan/BUILD_PLAN_V11.md) sequence — this roadmap just refines **which workflow-e2e investments come first within each phase**, ordered by the customer-trust ROI.
