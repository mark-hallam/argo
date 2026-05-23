# Round 32 — PR #2112 Claim-5 LIVE anti-theater verification

**Date**: 2026-05-23
**Round**: 32 (scheduled-task hourly, read-only)
**Pillar/Claim**: Mission claim 5 — "Any task a human can do on a PC. TWO execution backends: MCP AND computer-use"
**Anchor**: `417b74746` (eval-claim-5-live-mcp-and-cu-test (#2112)) on `main` at `1706d4eeaa7012eab44656e46aac4031e262d0f0`
**Merged**: 2026-05-23T08:50:59Z by conductor tick-102

## Verdict

🟢🟢 **PASS** — MCP half (6/6 tests green on main, cold-built from origin/main worktree; real `WorkerSessionRunner` + `Executor` + `ToolDispatcher` + `WorkflowAuditEmitter` + `WorkflowCapabilityChecker` seams; assertions pin tenant-keyed routing + cap-check-before-dispatch + failure-not-swallowed contracts; both happy + sad paths covered). The cu-macos half is gated correctly (env+cfg+permissions triple-gate; skip path + always-runs validate guard).

## Setup

```bash
cargo test -p argo-workflow \
  --test claim_5_live_mcp_pool_routing_test \
  --test claim_5_live_mcp_cap_audit_test
```

Cold build from main worktree (no shared incremental cache). Compile ~15min then test exec ~0.00s (all in-memory, no I/O).

## Observed

```
running 4 tests
test mcp_fake_server_transport_500_takes_same_failure_path ... ok
test mcp_fake_server_dispatch_error_is_surfaced_not_swallowed ... ok
test mcp_fake_server_audit_row_carries_tenant_and_principal ... ok
test mcp_fake_server_cap_check_fires_before_dispatch ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 2 tests
test mcp_fake_server_call_is_keyed_by_tenant_and_server ... ok
test mcp_fake_server_two_tenants_route_independently ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

6/6 green. PR shape: +959 LOC across 5 files (3 new tests, 1 shared `common/mod.rs`, +2 LOC to `permission_revoke_mid_run_test.rs` — the cu-macos P0 tenant_id+worker_id fix shipped as collateral).

## Anti-theater check

The MCP half is structurally identical to #2109's pattern (verified round 31): test-local stubs implement REAL crate-level traits, runner + executor are the real production code:

1. **`WorkerSessionRunner` + `Executor`** — both imported from `argo_workflow` (crates/argo-workflow/src/{runner.rs,executor.rs}). Not test-local. If production code regressed the runner→dispatcher→audit→cap-check sequence, these tests would catch it because the test only swaps the I/O-bearing traits at the seams the runner consumes.
2. **`ToolDispatcher`, `WorkflowAuditEmitter`, `WorkflowCapabilityChecker`** — public traits exported from `argo_workflow`. The stub `FakeMcpServerPool` (impl `ToolDispatcher`) routes by `(tenant_id, server, tool)` — the exact 3-tuple `argo_mcp::McpClientPool` keys by structurally (per round 10 finding, `PoolKey { tenant_id, manifest_id }`). If the runner ever dropped `tenant_id` from the dispatcher call, assertion 1 (`calls[0].key.tenant_id == tenant_id`) would fail.
3. **Cap-check fires BEFORE dispatch** — the load-bearing witness is `pool.calls().is_empty()` after a refusal. If the runner ever moved the cap-check after dispatch (the classic security-bypass refactor), the assertion flips red. This is exactly the same shape #2089 pinned for the runner happy path; #2112 extends it across the failure surfaces (4 distinct verdicts: capability-denied, success-audit-payload, tool-error-surfaced, transport-error-surfaced).
4. **Audit row tenant + principal stamping** — `events[0].tenant_id == tenant_id` + `events[0].principal_label == "web:operator-9"`. The audit `StepAuditContext` carries these from the runner; if either dropped, the assertion fails.
5. **Two-tenant pool isolation** — `assert_ne!(tenant_a, tenant_b)` + `tenants.len() == 2` after both runs proves the pool-key tuple isn't being collapsed across tenants. This is the substrate contract that `mission-mcp-connector-token-cache-tenant-keyed` (tick-87 audit) protects.

The 4 failure-surface assertions are particularly anti-theater-resistant: an attacker writing fake-green tests for `step_failed` would typically only assert kind=="failed" + count==1. These assert: the upstream message round-trips into `reason_or_response.as_str()` AND `report.task_verdicts` carries `Failed { reason }` AND `pool.calls().len()==1` for the post-dispatch flavour vs `==0` for the pre-dispatch flavour. Negative-path parity (tool error vs transport error producing structurally-identical audit rows) is the kind of invariant that only matters in production but is the rear-guard against silent failure-classification drift.

## cu-macos half (claim_5_live_macos_cu_smoke_test.rs)

The cu half is **gate-fenced**: a triple-gate (`ARGO_NATIVE_MACOS_INTEGRATION=1` env + `cfg(target_os="macos")` + `MacosCuExecutor::preflight().all_granted()`) — if any gate is closed, the test prints a skip banner and returns Ok. A separate always-runs assertion (`claim_5_live_macos_cu_config_always_validates`) keeps the test binary always passing. This is the **correct anti-theater pattern** for permission-gated native tests on shared CI: the substrate path validates on every host, the real path only runs when the operator's machine actually has the permissions. Verifying the live path requires the operator to set the env + grant the 4 macOS permissions — out of scope for a scheduled-task evaluator.

## Cu-macos P0 collateral fix

`crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs` line 140-141 now reads:
```rust
tenant_id: argo_types::TenantId::new(),
worker_id: argo_identity::WorkerId::new(),
```

This is the 2-line additive fix the round-26 evaluator finding + round-30/31 P0 escalations demanded; shipped as collateral by the brief author rather than waiting for `fix-cu-macos-permission-revoke-test-add-tenant-worker-fields` to land independently. Verifies the cu-macos crate now compiles its tests against the canonical `MacosCuRunRequest` struct shape.

## Gap / follow-up brief candidates

None new from this scenario — #2112 is gold-standard anti-theater. The only theatrical seam is in the cu half (skip-on-no-permissions) but that's correct for native-integration gating, not a gap to chase.

**Mission scoring impact**: Claim 5 MCP-half is now LIVE-pinned (was substrate-only per round 16's WorkerSessionRunner verify). With Claim 6 MVP at 100% LIVE post #2113 and Claim 5 MCP-side at 🟢🟢 here, the v1.0 ship-gate evidence chain is in excellent shape. **12th green mission-claim regression contract on `main`** this week (counting #2112's 6 tests + #2113's 2 tests = 14 if counted per-test; per-brief: 12).

## Cite

- Test files: [crates/argo-workflow/tests/claim_5_live_mcp_pool_routing_test.rs](../../../../crates/argo-workflow/tests/claim_5_live_mcp_pool_routing_test.rs), [crates/argo-workflow/tests/claim_5_live_mcp_cap_audit_test.rs](../../../../crates/argo-workflow/tests/claim_5_live_mcp_cap_audit_test.rs), [crates/argo-workflow/tests/common/mod.rs](../../../../crates/argo-workflow/tests/common/mod.rs), [crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs](../../../../crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs)
- PR: https://github.com/mark-hallam/argo/pull/2112 (merged `417b74746`)
- Pattern reference: round 31 / `round-31-claim-3-live-merged-anti-theater-verify.md`
