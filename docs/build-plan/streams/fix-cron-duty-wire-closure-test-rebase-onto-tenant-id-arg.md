# fix-cron-duty-wire-closure-test-rebase-onto-tenant-id-arg

- **Priority**: 🟡 P1 (workspace-clippy gate-5 blocker; pre-existing rebase drift on a test that pins claim-2 wiring)
- **Owner**: claude
- **Type**: fix (test rebase, ~10-30 LOC mechanical)
- **Fast-mode candidate**: YES (pure mechanical rebase of 3 callsites; signature change is already on main, test wasn't updated)
**Authored against**: origin/main `ccdff2958` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cron-duty-wire-closure-test-rebase-onto-tenant-id-arg`.

## Premise

PR #2091 (`eval-orchestrator-cross-tenant-handoff-negative-test`, merged earlier this loop) grew the signature of `DutyScheduler::sessions_for_duty` to accept a `TenantId` argument. The canonical signature at `crates/argo-orchestrator/src/duty_scheduler_store.rs:205-209` is:

```rust
pub async fn sessions_for_duty(
    &self,
    tenant_id: TenantId,
    duty_id: &str,
) -> ArgoResult<Vec<(SessionId, DateTime<Utc>, String)>>
```

But `crates/argo-agent-runner/tests/cron_duty_wire_closure_test.rs` was not rebased before #2091 merged. Three callsites still pass only `duty_id`:

- Line 120: `duties.sessions_for_duty(&duty.id).await.unwrap();`
- Line 177: `duties.sessions_for_duty(&duty.id).await.unwrap();`
- Line 275: `duties.sessions_for_duty(&duty.id).await.unwrap();`

Result: `cargo test -p argo-agent-runner -p argo-skill` fails compile with `E0061: this method takes 2 arguments but 1 argument was supplied`, which blocks workspace clippy gate-5 from going clean. PR #2143's boundary note already documented the regression; same-shape as ongoing #2074 cu-macos `MacosCuRunRequest` drift that was caught earlier this week.

This test pins the **claim-2 closure-loop wire** (DutyScheduler → AgentRunner closure invocation produces the expected `WorkerDutyFired` audit + duty-fired-session row). Letting it stay broken means we have no compile-protected pin on claim 2's load-bearing wiring.

## What to do

Update the 3 callsites in `crates/argo-agent-runner/tests/cron_duty_wire_closure_test.rs` to thread the test's tenant_id through `sessions_for_duty(...)`. Each test already has a `tenant_id` in scope (it's part of the per-test test-setup that constructs the duty via `DutyStore::insert`). The change is mechanical:

```rust
// before
let attribution = duties.sessions_for_duty(&duty.id).await.unwrap();
// after
let attribution = duties.sessions_for_duty(tenant_id.clone(), &duty.id).await.unwrap();
```

The exact local variable name may differ per test — read lines 80-120 / 140-180 / 240-275 for the surrounding context and use the local-scope tenant_id (it's already constructed for the duty insert).

If a test does NOT already construct a `tenant_id` in local scope, prefer adding one (one-line `let tenant_id = TenantId::from("ws-default");`) over weakening the contract. The whole point of the new signature is that the test exercises tenant-scoped reads.

## May-touch

- `crates/argo-agent-runner/tests/cron_duty_wire_closure_test.rs` — ONLY this file.

## Hands-off

- `crates/argo-orchestrator/src/duty_scheduler_store.rs` — the production signature is correct; do not weaken it.
- Any other test file — out of scope.

## Verification

- `cargo test -p argo-agent-runner --test cron_duty_wire_closure_test` — all 3 tests green.
- `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` — clean.
- `bash scripts/workspace-clippy-fast-fail.sh` — exits 0 (this is the gate this brief unblocks).

## Acceptance

- All 3 callsites updated, all 3 tests green.
- No production code touched.
- Workspace clippy gate-5 passes again.

## Provenance

Round-36 evaluator finding (2026-05-23T13:27Z) — pre-existing rebase drift documented in PR #2143's boundary note. Pattern is the same shape as the ongoing #2074 cu-macos `MacosCuRunRequest` regression (closed 2026-05-23 collaterally via #2112).
