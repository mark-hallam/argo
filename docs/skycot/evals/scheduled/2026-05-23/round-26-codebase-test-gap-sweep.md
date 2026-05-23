# Round 26 — Code-base test gap sweep + Claim 2 LIVE substrate test

**Date**: 2026-05-23
**Operator directive 2026-05-23 19:00Z**: "e2e site testing AND e2e code base testing."

## Summary

Enumerated the workspace's test surface, found two material gaps, and authored the higher-impact one as a falsifiable regression test.

## Gap-sweep stats

- **95 `argo-*` crates** in workspace
- **387 integration test files** total across them
- **34 crates with `tests/` directories** (containing at least one .rs file)
- **🟡 61 of 95 crates (64%) have ZERO integration tests** — only unit tests under `src/` if any
- `argo-cli` carries the bulk: 281 of the 387 integration tests live there

### Crates with ZERO integration tests (61 total — top mission-relevant)

| Crate | Mission impact | Severity |
|---|---|---|
| `argo-cron` | **Claim 2 LIVE substrate** — duty cron fires | 🔴 P0 — addressed by this round |
| `argo-judge` | **Claim 3 substrate** — genesis safety judge | 🟡 P1 next-round target |
| `argo-billing` | spend-ceiling enforcement | 🟡 P2 |
| `argo-compliance` | GDPR/SOC2 evidence | 🟡 P2 |
| `argo-delegate` | delegate workflows | 🟡 P2 |
| `argo-cron` ← now has 3 integration tests after this round | — | — |
| `argo-acp` | agent communication protocol | 🟡 P2 |
| 27 × `argo-mcp-connector-*` connectors | mission-text-listed connectors | 🟢 P3 (in-MCP connectors are deferred-v1.x per operator) |
| 3 × `argo-executor-computer-use-{linux,windows}` + supporting | macOS path is the v1 demo; others DEFERRED-v1.x per operator-locked decision 2026-05-14 | 🟢 P3 |
| 27 other crates | mixed substrate-only or unit-test-covered | 🟢 P3 |

## 🚨 P0 finding — workspace test build broken on origin/main

Running `cargo test --workspace --no-run` fails:

```
error[E0063]: missing fields `tenant_id` and `worker_id` in initializer of `MacosCuRunRequest`
   --> crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:139:5
```

**Root cause**: PR #2076 ("feat(cu-macos): tenant-mismatch pre-flight enforcement + TenantWorkerLookup wiring") added two fields to `MacosCuRunRequest` (`tenant_id`, `worker_id`) but the existing test file `tests/permission_revoke_mid_run_test.rs:139` was not updated to populate them.

**Impact**: any `cargo test --workspace` invocation fails before any tests run. Affects every worker / conductor / CI pipeline that uses workspace-wide test invocation.

**Severity**: P0 — blocks CI, blocks every worker's "tests green" verification step.

**Recommended brief (for conductor)**: `fix-cu-macos-permission-revoke-test-tenant-fields` — 2-line edit in `crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:139-145` adding `tenant_id: TenantId::new(),` and `worker_id: WorkerId::new(),` (or appropriate fixture values). Tiny — should be a 5-minute claim.

## Test authored this round

**File**: `crates/argo-cron/tests/claim_2_strengthener_cron_fires_per_tenant.rs`
**Tests**: 3 (all green on `main` as of `0acbe636f`)

| # | Test | What it pins |
|---|---|---|
| 1 | `multi_tenant_cron_fans_out_in_one_tick` | 5-phase test: 3 tenants × mixed enabled/disabled/schedule shapes; 1 tick at minute 0 fires exactly 3 jobs (2 from tenant_a, 1 from tenant_b's enabled job); skips tenant_b's disabled job + tenant_c's minute-33 job. Same-minute re-tick fires zero. Next-minute tick fires 3 again. Minute-33 tick finally fires tenant_c's job. |
| 2 | `runner_error_does_not_advance_cursor_across_tenants` | Defensive: one tenant's job fails, sibling tenants' succeed. `last_fired_at` advances ONLY for success; failing job retries next tick. Report distinguishes `fired` from `errors`. |
| 3 | `handles_many_tenants_in_one_tick` | Stress: 25 tenants × 1 enabled job each → single tick fires all 25; runner sees exactly 25 unique invocations. |

## Why this is the Claim 2 LIVE substrate pin

The operator-locked LIVE criteria (round 15):

> Claim 2 LIVE = a hired worker's duty cron fires AND invokes its agent runner AND writes a WorkerDutyFired audit row within 60s.

This test pins the **scheduler half** — that the `(JobStore, Scheduler, JobRunner)` triple correctly fans out across tenants and dedupes within a minute. The agent-runner half is BLOCKED on the in-flight broken-wire fix `fix-duty-scheduler-invokes-agent-runner` (CLAIMED, fix-by 2026-05-30). Once that lands, the END-TO-END Claim 2 LIVE test can replace the `RecordingRunner` mock with a real `Agent::turn` invocation and assert the audit row's shape.

## Cross-reference

- **Claim 2 MVP score**: ⏳ 0% pre-round-26 → **🟡 substrate ~50%** post-round-26. Half of the LIVE criteria is now backed by a green regression contract.
- **Total mission-claim regression tests on main this week**: 7 (rounds 17, 21, 22, 23, 24, 25 [SPA walkthrough — findings only, no new tests], 26).
