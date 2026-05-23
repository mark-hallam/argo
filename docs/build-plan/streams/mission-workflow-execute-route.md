# mission-workflow-execute-route

**Authored against**: origin/main `b0afe9f99` on 2026-05-23
**Owner**: claude
**Priority**: 🟡 P1 — mission-claim-3 HALF-LIVE → LIVE gate
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workflow-execute-route`

## Why this is P1

Audit 2026-05-23T03-39 §2 named this the gating follow-up that turns
mission-claim-3 from **LIVE-PENDING-HTTP-ROUTE → LIVE**. PR #2089
landed `WorkerSessionRunner` (the real `TaskRunner` impl wiring
argo-mcp + capability + atomic audit). But **no HTTP path invokes
it**. The runner is reachable from `argo-cli` only — the SPA's
workflow detail page cannot drive a workflow execution today
because the gateway has no `POST /api/v1/workflows/:id/execute`
route.

Demo MP4 needs: hire worker → build/edit workflow → click "Run"
button on workflow detail page → server invokes
`WorkerSessionRunner::execute` → audit_record row lands → SPA polls
for completion. Step 3 is the missing link.

## Pre-flight grep

Before authoring scope, worker MUST verify:

```
rg "POST.*workflows.*execute" crates/argo-gateway/
rg "WorkerSessionRunner" crates/argo-gateway/ crates/argo-cli/src/handlers/
rg "fn run_workflow" crates/argo-gateway/
```

The audit named `crates/argo-orchestrator/.../runner.rs:50-57` as
calling out the missing HTTP route in code comments. Worker should
read that comment block first and let it drive the route shape.

## Scope

NEW gateway route + handler:

- `POST /api/v1/workflows/:id/execute` (JSON body: `{ inputs:
  serde_json::Value, idempotency_key?: String }`).
- Resolves `(tenant_id, workflow_id)` via existing
  `WorkflowDefinitionStore`.
- Constructs `WorkerSessionRunner` with the same wired deps the CLI
  handler uses (`mcp_pool` + `audit_sink` + `capability_engine` from
  `AgentDeps`).
- Invokes `runner.execute(ExecutionRequest { principal,
  workflow_id, inputs })` — return `202 Accepted` + `{
  run_id: String }` on submission, OR `200 OK` + run result on
  synchronous completion (worker picks one — synchronous is fine
  for v1 demo since `MockRunner::echo` returns immediately and
  `WorkerSessionRunner` waits on `McpClient::call_tool` which is
  bounded by capability timeout).
- Capability + tenant pre-flight BEFORE invoking the runner.
- Audit row on dispatch (NEW `AuditKind::WorkflowExecuteDispatched`
  if not already there) AND on completion (existing audit-trail in
  `WorkerSessionRunner`).
- Error contract: 403 on RBAC deny, 404 on tenant-scoped lookup
  miss, 409 on idempotency-key collision (if implemented), 500
  otherwise. NEVER 200 on a denied request.

## SPA wire-up (light)

Workflow detail page (`web/app/workflows/[id]/page.tsx`) gains a
"Run workflow" button. On click → `fetch('/api/v1/workflows/:id/
execute', { method: 'POST', body: JSON.stringify({ inputs }) })`.
On 202 → optimistic toast + poll `/api/v1/workflow-runs/:run_id`
every 2s until terminal state. On 200 → show result inline. On
non-2xx → toast error + stay on page.

If the SPA route already has a stub "Run" button (worker checks
`page.tsx` first) — wire it. If not — add it as a 8-LOC affordance.

## Tests

- `crates/argo-gateway/tests/routes_workflow_execute_test.rs` (NEW):
  - happy path: tenant_a creates workflow + executes → 200/202,
    audit row exists.
  - tenant_b cannot execute tenant_a's workflow (404 same-shape as
    not-found, NOT 403 — existence-leak parity per #1992 pattern).
  - capability-deny → 403 + AuditKind::CapabilityDenied row.
  - missing principal → 401.
- E2E smoke: extend `scripts/smoke-skycot.sh` with a
  curl-and-assert against the route (out of scope if it grows the
  script past its budget — leave a TODO note instead).

## Acceptance

1. `gh pr view <N> --json files` shows ZERO new files >500 LOC.
2. New route compiles + clippy clean.
3. 4 new tests pass.
4. Demo path: SPA workflow detail → Run button → audit row visible
   in audit page. (Worker captures a screenshot for the PR body.)
5. Mission claim 3 LIVE — gap-manifest argo-workflow row flips from
   🟡 P1 → 🟢.

## May touch

- `crates/argo-gateway/src/routes_workflow_execute.rs` (NEW)
- `crates/argo-gateway/src/routes.rs` (route registration)
- `crates/argo-gateway/tests/routes_workflow_execute_test.rs` (NEW)
- `crates/argo-orchestrator/src/.../runner.rs` (DELETE the
  "missing HTTP route" comment block at lines 50-57 once wired)
- `web/app/workflows/[id]/page.tsx` (Run button + fetch handler)
- `web/lib/api.ts` (if a typed client wrapper exists, add the new
  endpoint)
- `crates/argo-audit/src/kinds.rs` ONLY if
  `WorkflowExecuteDispatched` doesn't already exist.

## Must NOT touch

- `WorkerSessionRunner` itself — it's already correct per #2089.
- Migration files.
- Any other gateway route.

## LOC budget

~400-500 LOC (gateway route + tests + SPA wire-up).

## Manager-worker eligibility

NO — single-PR scope. If worker discovers a needed precursor (e.g.
new AuditKind variant adds 2 callsites), split inline; don't fork.
