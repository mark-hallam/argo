# wire-worker-session-runner

**Owner**: claude.
**Depends on**: `eval-workflow-execution-runs-real-step` (#2086 MERGED with FINDINGS-ONLY ignores).
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh wire-worker-session-runner`.

🔴 **P0 MISSION-CLAIM-3 BROKEN WIRE — workflow execution layer doesn't exist.** Source: PR #2086 review pre-flight grep.

> "`argo-workflow` has zero refs to `argo-mcp`, `tenant_id`, or `audit_record`; only `MockRunner` exists workspace-wide. `WorkerSessionRunner` is referenced in `argo-cli/src/handlers/workflow.rs` header comment but never landed."

Mission claim 3 ("platform builds workers") needs the build-canvas → save → publish → **execute** loop. CRUD half shipped (round 7). Execution half — nothing. The header comment in `routes_workflow.rs` mentions `WorkerSessionRunner` as the canonical executor, but the struct + impl don't exist. Only `MockRunner` (test-only).

## Scope

Land the production `WorkerSessionRunner` (or rename to `TaskRunner` per evaluator's recommended name) that actually invokes argo-mcp tools + writes audit + threads tenant_id.

### Implementation

1. **NEW struct + impl** in `crates/argo-workflow/src/runner.rs` (or canonical location):

```rust
pub struct WorkerSessionRunner {
    mcp_pool: Arc<dyn McpClientPool>,
    audit_sink: Arc<dyn AuditSink>,
    capability_engine: Arc<dyn CapabilityEngine>,
}

impl WorkflowRunner for WorkerSessionRunner {
    async fn execute_step(&self, ctx: &ExecutionRequest, step: &Step) -> Result<StepOutcome, RunnerError> {
        // 1. Capability check: principal authorized for step.tool?
        self.capability_engine.check(&ctx.principal, &Capability::ToolDispatch { tool: step.tool.clone() })?;
        // 2. MCP dispatch: invoke step.tool via pool keyed by (tenant_id, server_name)
        let response = self.mcp_pool.client(&ctx.tenant_id, &step.server, step.transport)
            .invoke(&step.tool, &step.args).await?;
        // 3. Audit: write step-outcome row inside a tx
        self.audit_sink.write(AuditKind::WorkflowStepExecuted { ... }).await?;
        Ok(StepOutcome::Success { response })
    }
}
```

2. **Add `Principal` field to `ExecutionRequest`** — currently the request likely has tenant_id but not principal. RBAC needs it.

3. **Add `AuditKind::WorkflowStepExecuted` + `WorkflowStepFailed`** variants in `crates/argo-audit/src/event_kind.rs`.

4. **Wire `routes_workflows.rs` to use `WorkerSessionRunner`** for any execute endpoint (find or add `POST /api/v1/verticals/<v>/workflows/<n>/run` if it doesn't exist — separate brief if no exec route exists yet).

5. **Drop `#[ignore]`** on #2086's 3 contract tests + verify they pass.

6. **Cross-tenant negative**: tenant_a's workflow can't execute under tenant_b's principal (capability check refuses).

7. **Failure-injection**: mock MCP returns 500 → assert WorkflowStepFailed audit row + step marked FAILED in workflow_run checkpoint.

## Parallelism

- Manager-worker allowed: no (coordinated multi-file)
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-workflow/src/runner.rs`]

## Files you may touch

- `NEW: crates/argo-workflow/src/runner.rs` (~150-250 LOC).
- `crates/argo-workflow/src/lib.rs` — export the new runner.
- `crates/argo-workflow/Cargo.toml` — add `argo-mcp`, `argo-audit`, `argo-capability` deps.
- `crates/argo-workflow/src/execution_request.rs` (or where ExecutionRequest lives) — add `principal: Principal` field.
- `crates/argo-audit/src/event_kind.rs` — 2 new variants.
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — wire the runner; add execute endpoint if missing.
- `crates/argo-workflow/tests/execution_runs_real_step_test.rs` — un-ignore #2086's 3 tests.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `MockRunner` — leave intact for tests.
- `argo-mcp` core — use existing pool, don't refactor.
- `argo-capability` engine — use existing, don't extend.

## Acceptance criteria

- [ ] `WorkerSessionRunner` exists + implements `WorkflowRunner` trait.
- [ ] `ExecutionRequest` carries `principal`.
- [ ] Capability check fires BEFORE MCP dispatch.
- [ ] Audit row INSIDE the step's transaction (atomic).
- [ ] #2086's 3 `#[ignore]`'d tests pass.
- [ ] Cross-tenant negative test pinned.
- [ ] Failure-injection test pinned.
- [ ] `cargo clippy -p argo-workflow -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Workspace clippy clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `wire-worker-session-runner`
- PR title: `feat(workflow): WorkerSessionRunner wires MCP + audit + capability (wire-worker-session-runner)`

## Notes for the implementer

- ~400-600 LOC expected. Runner ~150, Principal addition ~30, AuditKind ~30, routes wire ~50, tests un-ignored ~30, cross-tenant ~80, failure-injection ~80, dep additions ~30.
- **The capability check BEFORE MCP dispatch is load-bearing** — without it, a worker with capability X can invoke tool Y (RBAC bypass).
- **If `POST /api/v1/.../run` route doesn't exist**, this brief surfaces that gap as a finding + names follow-up brief `mission-workflow-execute-route`. Don't try to land both in one brief.
- Cite #2086 review pre-flight + manifest WIRE BROKEN P0 verbatim.
