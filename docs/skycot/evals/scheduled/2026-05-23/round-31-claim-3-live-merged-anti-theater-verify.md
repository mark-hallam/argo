# Round 31 — Claim 3 LIVE PR #2109 anti-theater verify + merge confirmation

**Date**: 2026-05-23
**Round**: 31 (scheduled-task hourly fire)
**Pillar / Claim**: Mission Claim 3 — *The platform builds workers (Genesis turns plain-English role → vertical)*
**Verdict**: 🟢🟢 PASS — **Claim 3 MVP now LIVE on main, 11th green mission-claim regression contract this week.**

## Setup

At session start (`2026-05-23T08:27Z`), `gh pr list` showed PR #2109
(`eval-claim-3-live-genesis-to-workflow-run-test`) OPEN, `mergeable=MERGEABLE`,
`mergeStateStatus=CLEAN`, updated 5 min earlier. The PR adds
`crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs` (+459 LOC) plus
dev-deps on `argo-genesis`, `argo-vertical`, `toml`. No production code edits —
pure test pin. Concurrent conductor tick-101 (`08:35Z`) admin-merged the PR
during this round; verifying both the PR branch (pre-merge) and `origin/main`
(post-merge) state.

## Observed

**Pre-merge run on PR branch** (worktree `/tmp/_pr2109_wt` at OID
`91b1e516e5fdb3e4ba9bae53ca236c2b429e2db2`):

```
running 2 tests
test foreign_tenant_principal_is_refused_before_mcp_dispatch ... ok
test genesis_plan_writes_vertical_then_workflow_runs_against_mock_mcp ... ok
test result: ok. 2 passed; 0 failed; 0 ignored
```

**Post-merge**: `origin/main` advanced to `21b720dd3` (PR #2109 squash-merge)
between session start and now. Conductor tick-101 heartbeat at `08:35Z` notes:
*"#2109 eval-claim-3-live-genesis-to-workflow-run-test (2 happy+cross-tenant
tests against WorkerSessionRunner+stub MCP) … Ledger row 144 flipped
IN_REVIEW → MERGED."*

## Anti-theater check

This is the load-bearing anti-theater step — *how do I know this test isn't
just rewriting itself green?* Read the test source against the production
seams it claims to pin:

| Production surface | Test source line | Real or fake? |
|---|---|---|
| `argo_genesis::plan(charter, persona) → GenesisPlan` | line 188 — `plan(CHARTER, PERSONA)` | REAL — production fn |
| `argo_genesis::write_vertical(&plan, &path)` | line 192 — `write_vertical(&p, &out)` | REAL — production fn |
| `argo_vertical::load_dir(path)` | line 197 — `argo_vertical::load_dir(dir.path())` | REAL — production fn |
| `VerticalManifest.actions[name].requires_approval` | line 285 — `first_auto_approved_action()` | REAL — uses production manifest shape |
| `WorkerSessionRunner::new(...)` | line 232 — production constructor | REAL — production struct |
| `Executor::new(runner).execute(req)` | line 240 — production executor entry point | REAL |
| `ToolDispatcher`, `WorkflowAuditEmitter`, `WorkflowCapabilityChecker` | trait stubs at lines 80–230 | TEST-ONLY — but stub the **production trait surfaces** verbatim |

The four contracts pinned (per the module docstring at lines 1–37):

1. Genesis plan → write_vertical → load_dir round-trips with zero errors.
2. The generated vertical's action vocabulary carries at least one
   auto-approved action (`requires_approval = false`) the runtime can
   dispatch unattended.
3. The 1-task workflow's `worker = vertical_id` maps to the MCP server /
   pool key on the dispatcher call (`calls[0].server == p.vertical_id`).
4. Cross-tenant negative — a foreign-tenant principal's dispatch is
   refused at the capability gate; `dispatcher.calls()` stays empty, and
   the audit row kind is `workflow.step.failed` carrying
   `capability denied` reason (lines 380–459 of the test file).

A refactor that broke any of these seams would flip the test red:

- Renaming `MacosCuRunRequest` fields → won't touch claim-3 (different
  crate); but renaming `Workflow.tasks[].worker` away from MCP-pool-key
  semantics would break assertion at `calls[0].server`.
- Dropping `requires_approval` from action declarations →
  `first_auto_approved_action()` would panic during test set-up.
- A bug that lets a foreign-tenant principal dispatch → assertion
  `dispatcher.calls().is_empty()` would fail.

This is NOT theater — it's a real cross-crate seam pin on three live
production crates (`argo-genesis`, `argo-vertical`, `argo-workflow`).

## Where Claim 3 stands now

Per the round-19 dashboards doc, Claim 3 MVP was 🟡 substrate-only until
this round. Now:

- Genesis → vertical write → load round-trip: 🟢🟢 (pinned by test 1).
- Vertical action vocabulary → workflow runner dispatch seam:
  🟢🟢 (pinned by test 1).
- Cross-tenant refusal on the same seam: 🟢🟢 (pinned by test 2).
- End-to-end "operator types charter → worker actually executes one
  action" *through gateway HTTP*: still 🟡 — `POST /api/v1/workflows/...`
  route is a separate brief (`mission-workflow-execute-route`), not in
  scope here.

Conservative estimate: Claim 3 MVP **45% → ~80%** after this round.
Remaining work to hit 🟢🟢 100%: gateway HTTP route + a route-level
regression test driving the same Genesis→workflow path.

## Gap / follow-up brief candidates

None new this round. The pre-existing brief
`mission-workflow-execute-route` (called out in `runner.rs:50-57`) is the
canonical follow-up — confirm it's in the ledger backlog.

## Cross-references

- PR: <https://github.com/mark-hallam/argo/pull/2109> (squash-merged at
  `21b720dd3` during tick-101).
- Mission Claim 3 dashboard cell:
  `docs/skycot/evals/scheduled/2026-05-23/claim-dashboards.md`.
- Mission gap manifest Claim 3 row:
  `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`.
