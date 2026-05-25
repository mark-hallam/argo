# mission-claim-2-kpi-rollup-rebase-and-resubmit

**Owner**: claude.
**Authored against**: origin/main `2bb33b002` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-2-kpi-rollup-rebase-and-resubmit`.
**Supersedes**: PR #2279 (BLOCKED on charter-clobber bug now fixed via #2287 MERGED).

🔴 **P0 DEMO-GATE — mission claim 2 KPI rollup, take 2.**

PR #2279 originally shipped `KpiRollupScheduler` + migration v82 + `WorkerKpiRolled` audit kind + 5 regression tests — but evaluator round-55 caught a P0 bug: the rollup tick was NULLing `worker.charter_md` every 5 min via `update_charter(t, w, None, Some(kpis))`. The fix landed in #2287 (`IdentityStore::set_kpis` sibling). PR #2279 is BLOCKED waiting for its author to rebase + adopt the fix + add the 6th regression test.

This brief lets a fresh worker re-ship the rollup feature against current main with the fix already merged. Faster than waiting for the #2279 author.

## Scope

1. **Open a new branch** off current `origin/main` (which now has #2287's `IdentityStore::set_kpis`).
2. **Lift the rollup scope from PR #2279**: `gh pr diff 2279 > /tmp/pr-2279.diff` (read-only). Cherry-pick the additions: `crates/argo-orchestrator/src/kpi_rollup.rs`, migration v82 (`crates/argo-store/src/migrations/v82_*.rs`), `WorkerKpiRolled` AuditKind variant, gateway boot wire in `boot_spawns.rs`. Skip the broken `update_charter(t, w, None, Some(...))` call.
3. **Wire to the new fix**: instead of `update_charter(..., None, Some(&kpis_value))`, call the new `identity.set_kpis(tenant, worker, &kpis_value).await?;` (lives at `crates/argo-identity/src/store.rs:374-ish` post-#2287).
4. **Port the original 5 tests** from #2279's `crates/argo-orchestrator/tests/kpi_rollup_test.rs`.
5. **Add the 6th test `rollup_does_not_clobber_existing_charter`** mirroring the evaluator's probe (port from `docs/skycot/evals/scheduled/2026-05-25/round-55-pr-2279-kpi-rollup-charter-clobber-finding.md`). Setup: seed a worker → `update_charter(t, w, Some("..."), None)` → run one tick → assert charter survives.
6. **Migration slot check**: confirm migration v82 doesn't collide with anything that landed in the 15-PR merge train (the registry should be fine but `bash scripts/check-migration-slots.sh` will confirm).
7. **Close PR #2279 as superseded** in the PR body of the new PR. Reference both the original PR + this brief.
8. **Ledger row**: flip `mission-kpi-rollup-from-audit-trail` BLOCKED → MERGED-as-superseded once this PR lands; add a follow-up note pointing at this brief's PR.

## Acceptance criteria

- [ ] New PR opens with the rollup feature reproduced against current main + the #2287 `set_kpis` wire.
- [ ] 6 tests pass cold (`cargo test -p argo-orchestrator kpi_rollup`).
- [ ] The 6th test `rollup_does_not_clobber_existing_charter` is present + green.
- [ ] `bash scripts/check-migration-slots.sh` passes (v82 unique).
- [ ] `cargo clippy -p argo-orchestrator -p argo-identity --all-targets -- -D warnings` clean.
- [ ] PR body cites: (a) the #2279 supersede, (b) #2287 as the dependency, (c) the evaluator round-55 report.
- [ ] Migration slot 82 unique; no collision.
- [ ] BUILD_STATUS.md updated: new row IN_REVIEW with new PR number; #2279 row flipped BLOCKED → MERGED-as-superseded.

## May-touch

- `crates/argo-orchestrator/src/kpi_rollup.rs` (NEW; ~200-300 LOC)
- `crates/argo-orchestrator/tests/kpi_rollup_test.rs` (NEW; 6 tests; ~300 LOC)
- `crates/argo-store/src/migrations/v82_*.rs` (NEW; the watermark column for `worker.kpis_rolled_at`)
- `crates/argo-store/src/migrations/registry.rs` (narrow: register v82 if not already on main)
- `crates/argo-audit/src/event_kind.rs` (narrow: add `WorkerKpiRolled` variant if not already)
- `crates/argo-cli/src/handlers/gateway/boot_spawns.rs` (narrow: wire `spawn_gateway_kpi_rollup`)
- This brief file (status updates)

## Out of scope

- Touching the #2287 `set_kpis` API (consume only)
- Touching the production hire paths `contract.rs:71` / `worker_charter.rs:34` (they pass `Some(charter)` correctly)

## Effort

~250-400 LOC. ~60-120 min including the cherry-pick + rebase. The diff is well-understood (the original PR #2279 + the eval's probe + the #2287 fix recipe combined).
