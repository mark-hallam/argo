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

LOC: ~350 (additive constructor pattern; production callsites switch to new method; tests keep existing constructor).

**Approach revision (after second codex BLOCK):** Use ADDITIVE constructor pattern instead of breaking `ToolContext::new` signature. This avoids touching 18+ test files (one of which — `memory_tests.rs` — is already 585 LOC and cannot grow per the file-size invariant).

- `ToolContext::new(...)` keeps its signature; internally sets `tenant_id` to `TenantId(Uuid::nil())` with a comment marking it as "test fallback — production must use `new_with_tenant`".
- NEW `ToolContext::new_with_tenant(..., tenant_id: TenantId)` is the production constructor.
- Add `#[deprecated(note = "production callers must use new_with_tenant; new() retained for test fixtures only")]` on `ToolContext::new`. The deprecation warning enforces the discipline without breaking compile.
- Production callsites switch to `new_with_tenant`.
- Test files are NOT touched in this brief.

## Files you may touch

**Struct definition:**
- `crates/argo-tool/src/tool.rs` — add `tenant_id: TenantId` field to `ToolContext` struct (defined at `tool.rs:53`); keep `new(...)` signature (sets `tenant_id` to nil + deprecation attribute); add `new_with_tenant(...)`. ≤60 modified lines.

**Agent runner integration (real tenant source — `spend.tenant_id` is the canonical source per audit F-005):**
- `crates/argo-agent-runner/src/dispatch.rs` — remove `dispatch_tenant()`; replace 2 callsites with `ctx.tenant_id`; update `McpClientPool` keying. ≤80 modified lines.
- `crates/argo-agent-runner/src/turn.rs` — narrowly: pass `session.tenant_id` when building `ToolContext` via `new_with_tenant`. ≤30 modified lines.
- `crates/argo-agent-runner/src/turn_helpers.rs` — narrowly: `dispatch_one` callsite where `spend.tenant_id` is in scope; construct `ToolContext` via `new_with_tenant`. ≤30 modified lines.
- `crates/argo-agent-runner/src/helpers.rs` — narrowly: any other production `ToolContext::new` callers in this file. ≤25 modified lines.

**Other production constructor callsites:**
- `crates/argo-cli/src/handlers/vertical_workflow_step.rs` — pass real tenant. ≤20 modified lines.
- `crates/argo-core/src/agent.rs` — narrowly: any production `ToolContext::new` callers. ≤20 modified lines.
- `crates/argo-tool/src/registry.rs` — narrowly: if registry constructs production ToolContext, switch to `new_with_tenant`. ≤25 modified lines.

NOTE: tool-impl files in `crates/argo-tool/src/tools/*.rs` construct `ToolContext::new` ONLY inside `mod tests` / `#[cfg(test)]` blocks (verified by inspection 2026-05-16). Those test-only constructors can keep using `new()` per the additive-constructor approach and DO NOT need touching in this brief.

**Nil-tenant dispatch_tenant() callers (the 6 known sites):**
- `crates/argo-tool/src/tools/delegate_task.rs` — replace `dispatch_tenant()` calls with `ctx.tenant_id`; remove the nil-tenant comment. ≤50 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_executor_browser.rs` — replace nil-tenant. ≤30 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_executor_mcp.rs` — replace nil-tenant. ≤40 modified lines.
- `crates/argo-cli/src/handlers/mcp_tools.rs` — replace nil-tenant. ≤30 modified lines.
- `crates/argo-cli/src/handlers/skill_ir_backfill.rs` — replace nil-tenant. ≤25 modified lines.

**New isolation test (new sibling file):**
- `crates/argo-agent-runner/src/dispatch_tenant_isolation_tests.rs` (NEW) — cross-tenant MCP pool isolation test. ≤120 LOC.

## Files you must NOT touch

- `crates/argo-tool/src/tools/` other than `delegate_task.rs` — tools that don't currently call dispatch_tenant() don't need updating in this brief (their `ToolContext` arg gains the field automatically via the struct change). Only update tools that ACTIVELY read tenant.
- `argo-store` / `argo-mcp` internals — those already accept `TenantId`; this brief just changes what callers pass.
- F-005 (`AuditSink::new` deprecation) — separate brief; this is the precursor.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `ToolContext` has `tenant_id: TenantId`.
- [ ] `ToolContext::new_with_tenant(...)` exists.
- [ ] `ToolContext::new(...)` has `#[deprecated(...)]` attribute pointing at `new_with_tenant`.
- [ ] `dispatch_tenant()` is deleted from `argo-agent-runner/src/dispatch.rs`.
- [ ] All 6 callers above use `ctx.tenant_id` (verify with `rg 'TenantId\(Uuid::nil\(\)\)|dispatch_tenant\(\)' crates/argo-agent-runner crates/argo-cli/src/handlers --type rust` — should return 0 hits in production paths).
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
- **Additive constructor pattern is intentional** (after 2 codex BLOCKs): bare `ToolContext::new` keeps its signature so 18+ test files don't break. The `#[deprecated]` attribute enforces the production discipline via compile warnings; production code uses `new_with_tenant` exclusively.
- F-005 (`codex-audit-tenant-plumb-and-deprecate`) builds on this: after this brief lands, AuditSink can be built with the real `session.tenant_id` everywhere.
- If a callsite genuinely doesn't have a tenant (eg. system boot, cron without a session), that's a bug worth surfacing — escalate rather than silently using nil.
- A follow-up brief can migrate test files off `ToolContext::new` over time. This brief deliberately doesn't touch tests.
