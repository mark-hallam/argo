# fix-worker-detail-run-now-action-and-endpoint

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-run-now-action-and-endpoint`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-1-and-2-claim-1-and-2.md` F2 [P0].

🔴 **P0 DEMO-BLOCKER — Scene 2 of the runbook requires "click Mariah → Run now → audit row lands in /audit within 5s". No Run-now affordance exists anywhere on `/workers/<id>/*`. The closest is the `/workflows/<vert>/<name>` Run-now button — which means Scene 2 as scripted is not recordable from the worker surface.**

## Root cause

Grep evidence:

```
$ rg -l 'Run\s*now|run-now|"Run"' web/app/\(app\)/workers/ web/components/skycot/
web/components/skycot/workflow/WorkflowDetail.test.tsx
web/components/skycot/workflow/add-as-schedule-drawer.tsx
```

The only `Run now` references are on the **workflow** detail page, not the worker. `WorkerOverflowMenu.tsx` has `"disable-duties"` but no `"run-now"` action. The runbook's Scene 2 click-path ("click Mariah → Run now → audit drill-in") has no button to bind to.

The supporting gateway endpoint `POST /api/v1/workers/<id>/run` also does not exist. There is no current mechanism to manually trigger a worker run from the SPA.

Two acceptable solutions, operator-decisional:

- **(A) Implement the missing affordance**: add Run-now button + new gateway POST endpoint that enqueues a one-off worker run.
- **(B) Rewrite Scene 2 to drive from `/workflows/<vert>/<name>` Run-now** (which DOES exist and is tested per `WorkflowDetail.test.tsx`).

Option B is the no-code-change demo unblock. Option A is the real product fix. Both should be considered; the brief covers Option A.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F2 [P0] — No "Run now" affordance on the worker surface

## Fix recipe (Option A — full implementation)

1. **Gateway endpoint** at `crates/argo-cli/src/handlers/gateway/routes_worker_run.rs` (NEW). `POST /api/v1/workers/<id>/run` with body `{ note?: string, trigger_source: "manual_run_now" }`. Response `{ action_id: "act_...", audit_id: "..." }`. Wires through `argo-agent-runner` to enqueue a one-off turn against the worker's default duty.
2. **Capability check**: the existing `permissions.can_pause_resume` flag is a reasonable proxy. Promote a `can_trigger_run` boolean if the operator wants finer control later.
3. **SPA button** in `web/components/skycot/worker/WorkerOverviewContent.tsx` next to the existing 3 inert action buttons. Primary "Run now" CTA that POSTs to the new endpoint, toasts the resulting `action_id` with `<Link href={"/audit?id=" + action_id}>View in audit</Link>`.
4. **Sonner toast** with a 5s auto-dismiss + an "Undo" affordance for the first 3s (cancel the queued turn).
5. **Audit emission**: the run must emit a real `worker.run` audit row with `decision_chain` including the manual trigger, so the runbook's "audit drill-in" beat lands on a real row.
6. **Web smoke test** at `web/playwright/workers/run-now.spec.ts`: click Run-now on Mariah's overview, assert toast, click "View in audit", assert audit-row landing.

## Acceptance criteria

- [ ] `POST /api/v1/workers/<id>/run` returns 200 with `action_id` for the demo tenant's seeded workers.
- [ ] `/workers/<id>/overview` shows a primary "Run now" button (or overflow item) above the fold.
- [ ] Clicking the button emits a real `worker.run` audit row.
- [ ] Toast offers an audit-link that deep-links to `/audit?id=<action_id>`.
- [ ] Playwright spec passes cold.
- [ ] Demo runbook Scene 2 click-path matches what the SPA delivers.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_run.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_api.rs` (register route)
- `crates/argo-agent-runner/src/triggers/manual.rs` (NEW or extend the duty-scheduler hook)
- `web/components/skycot/worker/WorkerOverviewContent.tsx` (add CTA, wire onClick)
- `web/components/skycot/WorkerOverflowMenu.tsx` (optional: add overflow item too)
- `web/lib/api/workers.ts` (add `triggerWorkerRun(id, note)` helper)
- `web/playwright/workers/run-now.spec.ts` (NEW)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (verify Scene 2 click-path matches)

## Out of scope

- Schedules tab end-to-end (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end`)
- Other 3 inert buttons (Add responsibility / Edit schedule / Save charter — covered by `fix-worker-detail-edit-toggle-stub-toasts`)
- Audit feed action-kind variety (covered by `fix-audit-feed-action-kind-variety`)
- Demo-seed audit row emission (covered by `fix-demo-seed-emit-worker-hired-audit-rows`)

## Effort

~300-500 LOC + 1 gateway route + 1 Playwright spec. ~8-12 hours. P0 because Scene 2 is unrecordable from the worker surface without it.
