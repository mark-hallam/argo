# rebase-fix-worker-detail-run-now-action-and-endpoint

**Owner**: claude.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-worker-detail-run-now-action-and-endpoint`.
**Supersedes**: PR #2449 (closed for re-author 2026-05-29) + original brief `streams/fix-worker-detail-run-now-action-and-endpoint.md`.

🔴 **P0 — Scene 2 of v1.0 demo MP4 runbook ("click Mariah → Run now → audit drill-in") is not recordable — `/workers/<id>/overview` still has no Run-now button. Re-implement against current main; PR #2449's 750-LOC approach conflicts with multiple merged PRs that have reshaped the gateway dispatch + WorkerOverviewContent + workers.ts + runbook.md surfaces.**

## What changed on main since PR #2449 was authored

Multiple unrelated PRs landed on the touched files:
- `gateway/mod.rs`: 6 commits since PR authoring including `c1cbab396` (approvals seeded surfacing), `18d1b1b99` (inbound channels surface), `6145082fc` (extract gateway store helpers — LOC unblock)
- `WorkerOverviewContent.tsx`: `720a33bba` (pronouns field), `603bf057f` (delete RetrainDialog onAdjust no-op)
- `web/lib/api/workers.ts`: `9c90765f4` (/workers/<unknown-id>/overview 404 fix)
- `routes_dispatch_post_paths.rs`: stable; PR #2449's mount point is still correct
- `docs/skycot/demos/v1-mission-mvp-runbook.md`: `18d1b1b99`, `cec1331c9`, `7c4838a3e`, `80f566df7` — multiple Scene 2 + audit + auth-bypass updates

The feature itself has NOT shipped: `routes_worker_run.rs` does not exist on main, and grep on `WorkerOverviewContent.tsx` returns 0 hits for "Run now" / "runNow" / "run_now" / "/workers/.*/run".

## What this brief MUST ship

Re-implement PR #2449's full posture against current `origin/main` HEAD:

1. **Gateway**: new `POST /api/v1/workers/<id>/run` at `crates/argo-cli/src/handlers/gateway/routes_worker_run.rs`.
   - Capability-gated via `Action::WorkerLifecycle` (admin-floor, same as pause/resume/freeze).
   - Emits a real `worker.run` audit row with `decision_chain[0].step = manual_trigger`, `actor`, `trigger_source`, `note`, `ts`.
   - Returns `{ action_id, audit_id, trigger_id, queued_at }` where `action_id` and `audit_id` both carry the lower-hex 32-char audit row id (so the SPA can deep-link to `/audit/action/<action_id>` without picking a field).
   - 9 unit tests covering 200/400/403/404/410/503 branches + the audit-row roundtrip.
2. **Mount**: register the route in `routes_dispatch_post_paths.rs` next to the existing pause/resume/archive/freeze handlers (NOT `routes_dispatch_post_api.rs`).
3. **Gateway mod.rs wire**: declare the new module in `crates/argo-cli/src/handlers/gateway/mod.rs`. Add the line in alphabetic order; small surgical change.
4. **SPA**: primary "Run now" button on the worker overview tab (`WorkerOverviewContent.tsx`). Sonner success toast with a "View in audit" affordance that deep-links to the just-emitted audit row. Sonner error toast on failure.
5. **Proxy**: new `web/app/api/workers/[id]/run/route.ts` mirroring the existing freeze/unfreeze proxies (bearer-token passthrough + `gateway_unreachable` fallback).
6. **API client**: extend `web/lib/api/workers.ts` with a `runWorkerNow(workerId)` function that hits the proxy.
7. **Playwright spec**: `web/tests/e2e/worker-detail-run-now.spec.ts` pinning click-path + error branch.
8. **Runbook**: reconcile Scene 2 in `docs/skycot/demos/v1-mission-mvp-runbook.md` to the new worker-overview Run-now button. Pull current Scene 2 copy, surgically swap the affordance; do NOT rewrite the whole scene.

## v1 posture (documented sensible deviations from the original brief, preserved here)

- **No `argo-agent-runner::triggers::manual` module.** v1 mirrors pause/resume/archive shape (audit-row-only): emit the audit row, return shape. The actual runner enqueue is a follow-up (`fix-worker-run-runner-trigger-wire`).
- **No 3-second undo affordance** — without runner enqueue, "undo" would be theater. The audit-link is the load-bearing affordance.
- **No new `Action::WorkerRun` capability** — re-using `WorkerLifecycle`.

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-worker-detail-run-now-action-and-endpoint` passes.
- [ ] `POST /api/v1/workers/<id>/run` returns 200 with `action_id` (capability-gated).
- [ ] `/workers/<id>/overview` shows a primary "Run now" button above the fold.
- [ ] Clicking the button emits a real `worker.run` audit row (verified via unit test `run_response_audit_id_resolves_to_real_row`).
- [ ] Toast offers an audit-link that deep-links to `/audit/action/<action_id>`.
- [ ] Playwright spec at `web/tests/e2e/worker-detail-run-now.spec.ts`.
- [ ] Runbook Scene 2 reconciled to the new affordance.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli routes_worker_run` 9/9 pass.
- [ ] `pnpm typecheck` (web) clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.

## May-touch

- `crates/argo-cli/src/handlers/gateway/mod.rs` (one-line module declaration)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_paths.rs` (route registration)
- `crates/argo-cli/src/handlers/gateway/routes_worker_run.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_worker_run_tests.rs` (NEW)
- `web/app/api/workers/[id]/run/route.ts` (NEW proxy)
- `web/components/skycot/worker/WorkerOverviewContent.tsx` (Run-now button + toast wiring)
- `web/lib/api/workers.ts` (additive `runWorkerNow`)
- `web/tests/e2e/worker-detail-run-now.spec.ts` (NEW)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 2 affordance reconcile)

## Out of scope

- Runner enqueue (`fix-worker-run-runner-trigger-wire` follow-up).
- 3-second undo affordance.
- New `Action::WorkerRun` capability.
- Per-duty/per-schedule Run-now path (separate follow-up).

## Effort

~750 LOC across the gateway + SPA + proxy + Playwright surface. ~3-4 hours.

## Source

- Original brief: `streams/fix-worker-detail-run-now-action-and-endpoint.md`
- Closed PR: #2449 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR; 750 LOC straddling Rust + web — too large to manually rebase)
- UX audit: `docs/skycot/ux-audits/2026-05-27-0415/scene-1-and-2-claim-1-and-2.md` F2 [P0]
