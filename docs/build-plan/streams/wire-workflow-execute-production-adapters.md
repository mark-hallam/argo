# wire-workflow-execute-production-adapters (FINAL SPRINT) — wire real workflow runner

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh wire-workflow-execute-production-adapters`.

## Why (evidence)
`POST /api/v1/verticals/{vid}/workflows/{name}/execute` still binds the
**test-only** `MockRunner::echo()` as its executor. The fallback is hard-coded at
`crates/argo-cli/src/handlers/gateway/routes_workflow_execute.rs:230`
(`Executor::new(MockRunner::echo())`), and the module header (lines 35, 221) admits
"v1 ships `MockRunner::echo`" / "default path keeps the deterministic MockRunner".

The **real production adapters already exist and are tested** in
`crates/argo-agent-runner/src/workflow_adapters.rs`:
- `McpToolDispatcher` (line 108) — `impl ToolDispatcher`, resolves a step's `server`
  name to a per-tenant MCP tool call.
- `AuditWorkflowEmitter` (line 186) — `impl WorkflowAuditEmitter`, writes one
  hash-chained audit row per step.
- `CapabilityWorkflowChecker` (line 252) — `impl WorkflowCapabilityChecker`, gates
  each step's tool on a capability.

So the wiring gap is: a live workflow run does **echo theater** instead of dispatching
real per-tenant MCP steps + emitting audit rows. Wire the production adapters
(driven by the canonical `argo_workflow::WorkerSessionRunner` path) into the
execute route, keeping the deterministic `MockRunner` as the **provider-error
fallback** only (NO_THEATER: do not silently echo on the happy path).

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
Confirm the surfaces below BEFORE declaring anything new — extend, do not fork:
- `rg -n 'MockRunner|Executor::new' crates/argo-cli/src/handlers/gateway/routes_workflow_execute.rs`
- `rg -n 'pub struct (McpToolDispatcher|AuditWorkflowEmitter|CapabilityWorkflowChecker)' crates/argo-agent-runner/src/workflow_adapters.rs`
- `rg -n 'WorkerSessionRunner|pub trait (ToolDispatcher|WorkflowAuditEmitter|WorkflowCapabilityChecker)' crates/argo-workflow/src crates/argo-agent-runner/src`
- `rg -n 'McpClientPool|tenant_id' crates/argo-agent-runner/src/workflow_adapters.rs` (re-use the canonical per-tenant pool; do NOT introduce a parallel pool type — the `McpConnectionPool` vs `McpClientPool` cross-tenant leak is a known P1).

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_workflow_execute.rs` (swap the executor
  construction: build the production-adapter executor from `AgentDeps` /
  per-tenant MCP pool + audit sink + capability engine; keep `MockRunner::echo()`
  ONLY in the `Err(provider…)` arm). Keep the file < 500 LOC — if it would exceed,
  add a sibling `routes_workflow_execute_adapters.rs` helper module.
- new (only if needed) `crates/argo-cli/src/handlers/gateway/routes_workflow_execute_adapters.rs`
  (executor assembly helper).
- a test module sibling (`routes_workflow_execute_tests.rs` or extend the existing one).

Do NOT modify `workflow_adapters.rs` (already tested) beyond making any needed
constructor `pub` if it isn't already.

## Acceptance (truth-bounded + testable)
- An integration test on the demo seed: execute a 1-step workflow whose step names a
  real in-core MCP tool → the run's audit history contains a real per-step
  `AuditWorkflowEmitter` row (not an echo row), and the step result is the tool's
  output, NOT `MockRunner::echo`.
- A second test: when the provider/dispatch errors, the deterministic `MockRunner`
  fallback engages and the response is shape-compatible (graceful degrade, no 500).
- A tenant-isolation assertion: the dispatcher resolves the MCP pool by the request
  tenant (re-uses `McpClientPool` keyed by tenant), so tenant A's run cannot reach
  tenant B's connector.
- `cargo test -p argo-cli` green for the new tests. **Windows-build caveat:** if the
  MCP transport pulls a unix-only dep, gate the integration test `#[cfg(unix)]` and
  note it in the test header (mirror the cu(macOS) skip-path convention).

## On ship
Update the relevant `docs/build-plan/FEATURE_MATRIX.md` workflow-execution row
(MockRunner-fallback → real-adapter, with the commit SHA + route) in the SAME commit,
then `python3 scripts/render-feature-matrix.py`.

## Effort
S (Rust). ~150–250 LOC incl. test. Additive; no schema change.
