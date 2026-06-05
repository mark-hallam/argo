# uf-execution-graduation-and-promotion (UF Phase-B · B3) — OPERATOR-GATED · HIGHEST RISK

**Owner**: claude.
**Depends on**: [[DEC-037]] (graduated real execution); B1, B2 landed first.
**Authored against**: origin/main `58a35a9f9` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-execution-graduation-and-promotion`.

## 🔒 HOLD — operator-gated (do NOT flip to TODO until released)
Do not build until ALL of: (1) every Phase A brief MERGED, (2) the live signup→hire→work→approve→pay gate passes, (3) Mark explicitly says "go-live Phase B". Per DEC-036/037, default-off + reversible. Flips LAST (after B1, B2). **Highest blast radius** — an execution-graduation adversarial 'try-to-break-it' review runs before merge; per-action opt-in keeps the radius small; approval STAYS on sensitive verbs.

## Why
Generated actions are all `executor="human"`; the real `McpExecutor` in the worker-actions path is a not-implemented stub. Finish it + add the per-action graduation/promotion path (DEC-037).

## Grep-before-write (verified 2026-06-05)
- **Reuse `McpClientPool`** from the runtime dispatch path (`crates/argo-agent-runner/src/dispatch.rs:5`) — do NOT author a parallel pool (the prior cross-tenant-leak P1 came from exactly that).
- Locate the stub: `rg 'McpExecutor' crates/` (the `worker_actions_executor*` files).
- The dangerous-autonomous-capability fence STAYS: `crates/argo-genesis/src/hire_gate.rs:320` (`dangerous_autonomous_fence_reason`).

## Files you may touch
- finish `McpExecutor::dispatch` (`worker_actions_executor*`) using the reused `McpClientPool`.
- new `POST /api/v1/workers/{id}/actions/{action}/executor` — rewrites that action's `ExecutorDeclaration` human→tagged `{kind,server,tool}`; RBAC + a probation precondition; promotion audit rows.
- approval STAYS on sensitive verbs; the `hire_gate.rs:320` fence STAYS.

## Acceptance
A promoted MCP action performs a real side-effect against a MOCK MCP server in tests; a non-promoted action still produces a human-handoff bundle; promotion without probation is rejected; tenant + RBAC enforced. Adversarial blast-radius review before merge. NEVER reported 'live' without a current-turn real executed-action audit row.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (execution-graduation row) + regen HTML.

## Effort
~2–3 briefs (L) — highest risk; per-action opt-in keeps blast radius small.
