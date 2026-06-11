# uf-b3-finish-mcp-executor-dispatch

**Owner**: claude or codex — Phase B completion (DEC-037); conductor review + adversarial review before merge (real side-effects).
**Depends on**: #2825 MERGED (`e6a52811f`, executor_promotion store + promote route).
**Authored against**: origin/main `e6a52811f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh uf-b3-finish-mcp-executor-dispatch

## Why (operator-directed 2026-06-11: Phase B now)

Phase B status, verified against main 2026-06-11: B1 (hire→role_duty bridge)
MERGED, B2 (`AutoApprovalDutyGate` behind `ARGO_DUTY_AUTOFIRE` default-OFF,
`boot_spawns_duty.rs:104-115`) MERGED, B3 promotion path MERGED + extended by
#2825. **The one remaining gap**: the executor that performs the real work is
still a stub — `worker_actions_executor_promotion.rs:34` imports `McpExecutor`
from `worker_actions_executor_stubs`, and the dispatch shim doc
(`worker_actions_executor.rs:17-20`) says stubs "become real without reshaping
the dispatch shim". A promoted action therefore still cannot produce a real
MCP side-effect. This brief finishes `McpExecutor::dispatch`.

## Scope

1. **Finish `McpExecutor::dispatch`** (replace the stub): resolve the action's
   tagged `ExecutorDeclaration {kind, server, tool}`, call the MCP tool via the
   EXISTING **`McpClientPool`** exactly as the runtime dispatch path does
   (`crates/argo-agent-runner/src/dispatch.rs:5,22` — wrapper around the
   process-wide pool). **Do NOT author a parallel pool** — the prior
   cross-tenant credential-leak P1 (`McpConnectionPool`) came from exactly
   that; per-tenant keying must come from the pool's existing tenant-scoped
   config path.
2. **Preconditions stay intact** (these all shipped; wire through, don't
   re-implement): action must be PROMOTED (#2825 `executor_promotion` store),
   probation precondition, approval still required on `requires_approval`
   verbs, the DEC-051 risk floor untouched, `ARGO_EXECUTION_GRADUATION`
   default-OFF gates any auto path.
3. **Outcome honesty**: map MCP success/failure into `ActionOutcome` with the
   real result payload (truncated/capped, no silent truncation — log what was
   dropped); failure → failed outcome + audit, never a fake success (DEC-029).
4. **Audit**: each real dispatch emits an audit row (worker, action, server,
   tool, outcome) — extend `AuditKind` only if no fit exists.

## Acceptance

- Integration test against a MOCK MCP server (the repo has MCP test
  fixtures — grep `tests/` for mcp mock/cassette): a promoted action performs
  the real call and records the outcome; a NON-promoted action still produces
  the human-handoff bundle (regression); promotion-without-probation rejected
  (pin exists — keep green).
- Tenant isolation: worker A's dispatch can never use worker B's tenant
  connector session (negative test).
- All flags default-OFF preserved: with flags off, today's behavior unchanged.
- 🔴 ADVERSARIAL REVIEW BEFORE MERGE (first real side-effect executor).

## Parallel-surface check

`rg 'McpExecutor|McpClientPool|McpConnectionPool' crates/` — finish the
existing stub in place; the ONLY pool is `argo-mcp`'s `McpClientPool`.
