# Round 16 — Verify #2089 WorkerSessionRunner closes claim-3 broken wire

- **Date**: 2026-05-23
- **Round**: 16
- **Pillar / Claim**: Mission claim 3 — "the platform builds workers" — workflow EXECUTION side (Genesis-side was already 🟢 round 11).
- **Verdict**: 🟢 PASS (P0 broken-wire closed)

## Why this scenario

Round 11 (2026-05-23T11:30Z) found mission claim 3 was **HALF-LIVE**:
Genesis side SOLID (`argo-genesis::safety_judge` enforces approval on every
sensitive verb), execution side BROKEN — no `WorkerSessionRunner` existed
that gated MCP (Model Context Protocol — JSON-RPC standard for tool servers)
dispatch on capability + emitted audit rows. The conductor authored
`wire-worker-session-runner` (P0 demo-blocker), and #2089 MERGED at
`2026-05-23T02:08:56Z`. This round verifies the wire is real.

## Setup

```
cargo test -p argo-workflow --test execution_runs_real_step_test
```

Plus full read of [crates/argo-workflow/src/runner.rs](crates/argo-workflow/src/runner.rs) (360 LOC) and the
contract tests at [crates/argo-workflow/tests/execution_runs_real_step_test.rs](crates/argo-workflow/tests/execution_runs_real_step_test.rs).

## Observed

5 / 5 tests PASS in 2m 09s (cold cargo compile):

```
test executor_hands_resolved_inputs_to_the_runner_seam ... ok
test executor_surface_walks_a_single_task_and_emits_a_verdict ... ok
test happy_path_runner_invokes_dispatcher_and_writes_audit_row ... ok
test failure_injection_dispatcher_500_marks_step_failed_and_writes_audit ... ok
test cross_tenant_runner_refuses_foreign_principal ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured
```

Production `WorkerSessionRunner::run` walks the four-step contract in the
order documented:

1. `parse_step` — bail with `Failed{reason}` + step_failed audit if the task
   shape is malformed (lines 241-254 of runner.rs).
2. **Capability check fires BEFORE MCP dispatch** — runner.rs:267-271 calls
   `capabilities.check_dispatch(principal, tool)` and returns
   `TaskOutcome::Failed{reason: "capability denied: …"}` without touching
   the dispatcher.
3. MCP dispatch via `dispatcher.call(&tenant_id, server, tool, args)` —
   tenant_id is the pool-key argument, structurally impossible to omit.
4. `audit.step_executed(&ctx, &response)` returns `Result<(), String>` so
   audit-write failures collapse to Failed verdicts — runner.rs:292-295.
   The forensic invariant "no successful step without an audit row" is
   load-bearing in code, not just docs.

## Anti-theater check

Three things would make this test theater rather than verification, and
none of them obtain:

- **Tests inject stubs, not real MCP**, but the cross-tenant test asserts
  `dispatcher.calls().is_empty()` after a capability denial
  (test file lines 400-404). That is not "did the runner exist" — that is
  "the dispatcher was never reached." A regression that moved the
  capability check below the dispatcher would flip this assertion.

- **Order-of-operations is enforced in source, not just docs.** runner.rs
  comments are reinforced by the runner returning early at line 270
  before the dispatcher.call at line 275. A future refactor that swapped
  those would fail `cross_tenant_runner_refuses_foreign_principal`.

- **The failure-injection test pins audit-on-failure** —
  `dispatcher.fail("upstream MCP server returned 500")` → assert exactly
  one `workflow.step.failed` audit row with the upstream message in the
  reason (test file lines 437-487). A regression that silently swallowed
  MCP errors would fail this test.

## Bonus finding

`argo-workflow` carries a Cargo-dependency carve-out documented at
runner.rs:20-38 — the brief originally proposed adding `argo-audit /
argo-capability / argo-mcp` as direct deps, but that closes a cycle
(`argo-workflow → argo-audit → argo-store → argo-runtime → argo-workflow`,
E0466). The shipped design uses three trait seams (`ToolDispatcher`,
`WorkflowAuditEmitter`, `WorkflowCapabilityChecker`) implemented at
`argo-cli` / `argo-runtime` level. This is the correct dependency-graph
shape — calling out in case future evaluators see the seam pattern and
wonder why it isn't a hard dependency.

The brief's own docstring (runner.rs:50-57) flags a known follow-up:
**no production gateway route exists** for
`POST /api/v1/verticals/<v>/workflows/<n>/run`. The follow-up brief
`mission-workflow-execute-route` is named there. Without that route,
the runner is reachable from in-process tests but not from the SPA
demo path. **Brief candidate (P1)**: confirm
`mission-workflow-execute-route` is in the queue, escalate if missing.

## Mission-vs-code manifest update

The gap-manifest row for **claim 3 / argo-workflow execution layer**
flips from 🟡 P1 (round 7 finding) to 🟢 — but with a footnote that the
HTTP route to invoke the runner is still missing. Manifest will be
edited inline.

## Verdict

🟢 PASS. Mission claim 3 execution-side wiring is **real production code
with meaningful regression guards**. Claim-3 turn from HALF-LIVE
(round 11) → LIVE-PENDING-HTTP-ROUTE (round 16).
