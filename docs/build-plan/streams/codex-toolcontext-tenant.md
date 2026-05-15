# codex-toolcontext-tenant — Plumb tenant_id through ToolContext; remove nil-tenant sentinel

**Owner**: codex. **Depends on**: nothing (precursor for F-005 audit-tenant-plumb).
**Authored against**: origin/main `ea8f41d0` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-toolcontext-tenant`.
**Source**: F-004 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`.

## Scope

P1 multi-tenancy. The tool/MCP dispatch path uses a nil tenant sentinel:

```
crates/argo-agent-runner/src/dispatch.rs:21
   fn dispatch_tenant() -> TenantId { TenantId(Uuid::nil()) }
   // "Tenant identity is not threaded through dispatch_one..."
```

The nil sentinel keys MCP pool acquisition at `dispatch.rs:57` (same `(tenant, manifest)` pool key for every tenant in process), is repeated in `delegate_task` at `crates/argo-tool/src/tools/delegate_task.rs:22`, and is inserted as the handoff tenant at `:184`. This violates `CLAUDE.md:46` multi-tenant-by-construction.

Scope:
- Add `tenant_id: TenantId` field to `ToolContext` in `argo-tool`.
- Update every `ToolContext` constructor site to plumb `session.tenant_id` from the agent runner.
- Replace every `dispatch_tenant()` call with `ctx.tenant_id` in the 6 known callers:
  - `crates/argo-agent-runner/src/dispatch.rs`
  - `crates/argo-tool/src/tools/delegate_task.rs`
  - `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs`
  - `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs`
  - `crates/argo-cli/src/handlers/mcp_tools.rs`
  - `crates/argo-cli/src/handlers/skill_ir_backfill.rs`
- Delete `dispatch_tenant()` (force compile failure for any new caller).
- Tests prove MCP pool keying isolates per tenant; tests prove delegate_task handoff persists the real tenant.

LOC: ~600 (struct field add + ~10 production constructor updates + ~12 test constructor updates + 6 nil-tenant callsite updates + new isolation tests).

## Files you may touch

**Struct definition + re-export:**
- `crates/argo-tool/src/tool.rs` — add `tenant_id: TenantId` field to `ToolContext` struct (defined at `tool.rs:53`); update `ToolContext::new` signature to require `TenantId`; update internal struct-literal callers in this file. ≤50 modified lines.
- `crates/argo-tool/src/lib.rs` — narrowly: re-export adjustment if needed (current re-export at line 42). ≤10 modified lines.

**Agent runner integration (real tenant source):**
- `crates/argo-agent-runner/src/dispatch.rs` — remove `dispatch_tenant()`; replace 2 callsites with `ctx.tenant_id`; update `McpClientPool` keying. ≤80 modified lines.
- `crates/argo-agent-runner/src/turn.rs` — narrowly: pass `session.tenant_id` when building `ToolContext`. ≤25 modified lines.

**Production constructor callsites (need real tenant plumbed in):**
- `crates/argo-cli/src/handlers/vertical_workflow_step.rs` — pass real tenant (likely from workflow step's session). ≤20 modified lines.
- `crates/argo-tool/src/registry.rs` — update internal `ToolContext::new` callers. ≤30 modified lines.
- `crates/argo-tool/src/tools/process.rs` — update constructor + dispatch_tenant call if present. ≤25 modified lines.
- `crates/argo-tool/src/tools/file_read.rs` — update constructor. ≤15 modified lines.
- `crates/argo-tool/src/tools/file_write.rs` — update constructor. ≤15 modified lines.
- `crates/argo-tool/src/tools/code_execute.rs` — update constructor. ≤15 modified lines.
- `crates/argo-tool/src/tools/file_patch.rs` — update constructor. ≤15 modified lines.
- `crates/argo-tool/src/tools/web/mod.rs` — update struct-literal callers. ≤15 modified lines.

**Nil-tenant dispatch_tenant() callers (the 6 known sites):**
- `crates/argo-tool/src/tools/delegate_task.rs` — replace `dispatch_tenant()` calls with `ctx.tenant_id`; remove the nil-tenant comment. ≤50 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs` — replace nil-tenant. ≤30 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` — replace nil-tenant. ≤40 modified lines.
- `crates/argo-cli/src/handlers/mcp_tools.rs` — replace nil-tenant. ≤30 modified lines.
- `crates/argo-cli/src/handlers/skill_ir_backfill.rs` — replace nil-tenant. ≤25 modified lines.

**Test files (pass explicit test-fixture tenant; nil is acceptable in `#[cfg(test)]`):**
- `crates/argo-tool/src/tools/delegate_task_tests.rs` — pass test tenant. ≤30 modified lines.
- `crates/argo-tool/src/tools/terminal_tests.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/src/tools/memory_tests.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/src/tools/clarify_tests.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/src/tools/send_message_tests.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/src/tools/todo_tests.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/src/tools/session_search_tests.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/tests/todo_clarify_integration.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/tests/process_registry_integration.rs` — pass test tenant. ≤15 modified lines.
- `crates/argo-tool/tests/code_execute_integration.rs` — pass test tenant. ≤15 modified lines.
- Existing `dispatch_tests.rs` + new cross-tenant isolation test sibling. ≤120 LOC.

Use a single test-fixture helper like `fn test_tenant() -> TenantId { TenantId(Uuid::nil()) }` inside a shared test helper to keep test code DRY. Test-side nil tenants are acceptable; production-side nil tenants must be deleted.

## Files you must NOT touch

- `crates/argo-tool/src/tools/` other than `delegate_task.rs` — tools that don't currently call dispatch_tenant() don't need updating in this brief (their `ToolContext` arg gains the field automatically via the struct change). Only update tools that ACTIVELY read tenant.
- `argo-store` / `argo-mcp` internals — those already accept `TenantId`; this brief just changes what callers pass.
- F-005 (`AuditSink::new` deprecation) — separate brief; this is the precursor.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `ToolContext` has `tenant_id: TenantId`.
- [ ] `dispatch_tenant()` is deleted from `argo-agent-runner/src/dispatch.rs`.
- [ ] All 6 callers above use `ctx.tenant_id` (verify with `rg 'TenantId\(Uuid::nil\(\)\)|dispatch_tenant\(\)' crates --type rust` — should return 0 production hits, only test fixtures).
- [ ] `McpClientPool` acquisition is keyed by the real tenant; tests prove cross-tenant isolation.
- [ ] `delegate_task` handoff persists the real tenant; test proves it.
- [ ] `cargo test -p argo-tool -p argo-agent-runner -p argo-cli` passes.
- [ ] `cargo clippy -p argo-tool -p argo-agent-runner -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-toolcontext-tenant`
- PR title: `refactor(tool): plumb real tenant_id through ToolContext; remove nil sentinel (codex-toolcontext-tenant)`

## Notes for the implementer

- This is a load-bearing security/correctness fix. The nil sentinel currently means cross-tenant MCP pool reuse is structurally possible — any future call site that runs multi-tenant in-process would leak credentials.
- The struct field add is a breaking change for ToolContext constructors. Take that hit cleanly — no `#[deprecated(...)]` wrapper, no `Default` impl that fills nil.
- F-005 (`codex-audit-tenant-plumb-and-deprecate`) builds on this: after this brief lands, AuditSink can be built with the real `session.tenant_id` everywhere.
- If a callsite genuinely doesn't have a tenant (eg. system boot, cron without a session), that's a bug worth surfacing — escalate rather than silently using nil.
