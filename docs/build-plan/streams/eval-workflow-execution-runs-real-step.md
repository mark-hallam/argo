# eval-workflow-execution-runs-real-step

**Owner**: claude (evaluator-shape brief — investigation + cargo integration test).
**Depends on**: none.
**Authored against**: origin/main `432aac842` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-workflow-execution-runs-real-step`.

🟡 **P1 MISSION-CLAIM-3 (Platform builds workers).** Source: round-8 manifest Claim 3 row for `argo-workflow`.

> "argo-workflow (runtime) — DAG / executor / checkpoint primitives. The workflow runner. Mission claim 3 needs the SPA build canvas → save → publish → execute loop end-to-end. Round 7 confirmed CRUD layer is solid; execution layer is unaudited."

Round 7 verified the CRUD half. Round 8 calls out the execution half — `argo-workflow::executor` has never been smoke-tested end-to-end. Without verifying it actually runs a step, mission claim 3 demo path is only half-real (save works, but does the run actually fire?).

## Scope

Cargo integration test in `crates/argo-workflow/tests/` that:

1. Construct a minimal workflow with ONE step (a `tool_call` against a mock MCP server).
2. Save it via the canonical store path (slot 130/v78 + workflow store from #2056).
3. Invoke the executor (find entry: `Executor::run` or `WorkflowRunner::execute`).
4. **Assert**:
   - The mock MCP server received the tool call.
   - The executor produced an `audit_record` row with the tool's actual response.
   - The executor wrote a `workflow_run` checkpoint with the step outcome.
5. **Cross-tenant negative**: try to run tenant_a's workflow with tenant_b's principal → assert refusal at executor pre-flight.

6. **Failure-injection**: mock MCP returns 500 → assert workflow_run row marks step `FAILED` (not silently swallowed), audit row still written with the failure.

If pre-flight reveals the executor doesn't actually exist OR has no test seam OR the wire is broken → ship `#[ignore]`'d + FINDINGS-ONLY.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-workflow/tests/execution_runs_real_step_test.rs` (~150-300 LOC).
- `crates/argo-workflow/Cargo.toml` — narrowly scoped: dev-deps for mock MCP.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — update argo-workflow row.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-workflow/src/**` — read-only audit.
- The mock MCP server impl — there's likely one in `argo-mcp-mock` or similar; use it, don't extend it.

## Acceptance criteria

- [ ] Single-step execution happy path passes.
- [ ] Cross-tenant negative refusal pinned.
- [ ] Failure-injection (MCP 500) → step FAILED + audit row written.
- [ ] If executor surface doesn't exist, ship `#[ignore]`'d + FINDINGS-ONLY with surface map.
- [ ] Gap manifest updated.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `eval-workflow-execution-runs-real-step`
- PR title: `test(workflow): executor end-to-end + cross-tenant + failure-injection (eval-workflow-execution-runs-real-step)`

## Notes for the implementer

- ~200-350 LOC expected. Happy path ~120, cross-tenant ~70, failure-injection ~60.
- **The failure-injection contract is the load-bearing one**. Without it, a regression that silently swallows step failures wouldn't be caught.
- **If `argo-mcp-mock` doesn't exist**, use `mockito` for the HTTP layer + a tiny in-test JSON-RPC handler.
- Cite round-8 manifest verbatim.
