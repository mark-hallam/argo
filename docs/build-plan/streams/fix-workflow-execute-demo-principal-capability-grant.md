# fix-workflow-execute-demo-principal-capability-grant

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-execute-demo-principal-capability-grant`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F6 [P1]).

🟠 **P1 — `POST /api/v1/verticals/{vertical}/workflows/{name}/execute` returns 403 `forbidden — principal lacks the required workflow capability` for the demo principal `human:demo@skycot.com` that the SPA uses. Clicking the "Run now" button during recording surfaces a permission-error toast rather than a successful execution + audit-row landing. Scene 5's narrated payoff ("watch the run land in the audit log") fails on the demo recording profile.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:163` wires the Run-now button to `executeWorkflow({verticalId, name})` — a real call, no mock, no theater. That's the positive part. The negative part:

```
$ curl -s -X POST -H 'X-Argo-Principal: human:demo@skycot.com' \
       -H 'Content-Type: application/json' \
       "http://127.0.0.1:7400/api/v1/verticals/support/workflows/triage_ticket/execute" \
       -d '{}'
{"error":{"code":"forbidden","message":"principal lacks the required workflow capability"}}
```

The principal-to-capability resolution path that maps `human:demo@skycot.com` → role → capabilities does NOT include the `workflow:execute` capability on the demo tenant. So every Run-now click in the recording configuration fails with a 403 → toast.

The function-side wiring is correct; the auth-grant is missing.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-4-and-5-claim-4-and-5.md` F6 [P1] — `/execute` returns 403 forbidden on the demo principal — clicking "Run now" in the recording will show an error toast

## Fix recipe

Two options. Recommend (a) for the v1.0 demo timeline.

**Option A — seed-step: grant the demo principal the workflow:execute capability under the demo tenant (preferred)**:

1. Locate the demo-tenant bootstrap in `crates/argo-demo-seed/src/` (likely `tenant.rs` or `principal.rs`). Verify with `rg -n 'human:demo@skycot.com' crates/argo-demo-seed/`.
2. Add (or extend) a seed step that grants role `workflow_operator` (or whatever role-name owns `workflow:execute` — verify in `crates/argo-security/src/capabilities.rs`) to the demo principal under the demo tenant.
3. Run `argo demo seed --tenant <demo>` against the running gateway as part of `scripts/restart-live-gateway.sh` so the grant is in place before any take.
4. Verify with curl:
   ```
   curl -s -X POST -H 'X-Argo-Principal: human:demo@skycot.com' \
        -H 'Content-Type: application/json' \
        "http://127.0.0.1:7400/api/v1/verticals/support/workflows/triage_ticket/execute" \
        -d '{}'
   ```
   Should return 200 with a real run envelope, not 403.

**Option B — SPA service-account principal for execute calls in demo mode**:

1. When `NEXT_PUBLIC_ARGO_DEMO_MODE=1` AND the workflow is a known demo workflow, the SPA includes `X-Argo-Principal: service:demo-runner@skycot.system` (or similar) on the execute POST.
2. Gateway grants the service principal the full capability set under the demo tenant.
3. Audit row records the service principal; for camera purposes that's a worse story than option A (operator wants the audit row to show `human:demo@skycot.com` clicked Run-now).

Pick (a). It's the right shape long-term — the demo principal is what every other SPA call uses, so the demo tenant's principal-capability graph should be coherent.

## Acceptance criteria

- [ ] Fresh `argo demo seed` run + curl against `/execute` returns 200 for `human:demo@skycot.com`.
- [ ] SPA "Run now" button on `/workflows/support/triage_ticket` (or whichever workflow Scene 5 lands on) succeeds and surfaces a success toast.
- [ ] Audit row appears within 5s with `principal == "human:demo@skycot.com"`.
- [ ] `cargo test -p argo-demo-seed` passes with a new test that asserts the demo principal has `workflow:execute` after seed.
- [ ] Cold rerun of `eval-claim-2-live-duty-cron-fires-runner-test` (or the workflow-execute analogue) passes.
- [ ] Document the grant in `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-flight checklist: "After `argo demo seed`, verify `curl … /execute -d '{}' → 200`."

## May-touch

- `crates/argo-demo-seed/src/` — one of `tenant.rs` / `principal.rs` / `capability.rs` (~30 LOC for the grant)
- `crates/argo-security/src/capabilities.rs` (read-only — verify role name)
- `scripts/restart-live-gateway.sh` (~3 LOC — ensure seed runs)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (~3 LOC checklist update)
- One Rust test (~40 LOC)

## Out of scope

- The canonical Scene-5 `customer_research_followup` workflow itself (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Builder palette MCP/CU step kinds (same brief above)
- The Run-now button label / alignment (covered by `fix-workflow-run-button-label-alignment` + `fix-workflow-detail-run-now-buried-among-five-ctas`)

## Effort

S. ~100 LOC total across seed + test + docs. ~2 hours. P1 because the "Run now" payoff is the load-bearing function-side beat for Scene 5 and currently 403s on the demo recording principal.
