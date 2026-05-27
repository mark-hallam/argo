# fix-runbook-prewarm-add-audit-action-and-workflow-viewer-routes

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-runbook-prewarm-add-audit-action-and-workflow-viewer-routes`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-1-and-2-claim-1-and-2.md` F9 [P1].

🟠 **P1 — `/audit/action/[actionId]` cold-compile takes 203s in dev; first request takes 784s (~13 minutes from click to paint). Today's runbook pre-warm list only covers `/compare` + `/workflows/build`. The audit-detail route is Scene 2's drill-in target — without pre-warm, the camera catches a 3-13 minute blank state on record.**

## Root cause

Documented in `preview_logs` from the audit:

```
Compiling /audit/action/[actionId] ... ✓ Compiled /audit/action/[actionId] in 203s
GET /audit/action/.../act_018f67d0-... 200 in 784813ms
```

The audit-detail route apparently drags in a heavy dependency tree: PII redactor + decision-chain renderer + provenance block + evidence-exporter modal lazy-load. This is a function of Next.js dev compile (Turbopack 16) — production builds are fast — but the v1.0 demo MP4 is recorded against `pnpm dev` per the existing runbook conventions.

The companion brief `fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct` addresses the runtime latency. This brief is the narrower runbook fix: just adding more pre-warm routes to the pre-record checklist so the operator doesn't catch the cold-compile pain on the recording.

Per memory `project_v1_demo_mp4_ship_gate` (locked 2026-05-21), the existing pre-record checklist already includes "/compare + /workflows/build pre-warm." This brief extends the list.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F9 [P1] — Cold-start dev compile time for `/audit/action/[actionId]` is 203s; first request takes 784s

## Fix recipe

1. **Update `docs/skycot/demos/v1-mission-mvp-runbook.md`** pre-record checklist. Add these routes to the pre-warm sequence, in this order:
   - `/audit` (list view — already fast, but anchors React Query cache)
   - `/audit/action/<known-canonical-id>` (the drill-in target — needs ~3-13 min cold-compile)
   - `/workers/<known-id>/overview` (Scene 1+2 anchor)
   - `/workers/<known-id>/activity` (Scene 2 activity-tab drill-in)
   - `/workflows/sales/qualify_lead` (Scene 5 anchor)
   - `/workflows/sales/qualify_lead/build` (if Scene 5 narrates the builder)
   - `/approvals` (Scene 6 anchor)
2. **Choose canonical IDs**: pick stable known-good action_id + worker_id + workflow names that exist in the demo seed. Document them inline in the runbook so the operator can copy-paste.
3. **Add a pre-warm script** `scripts/demo-prewarm.sh` that curls each route in sequence with a 60s timeout each and reports cold-compile duration. Operator runs this ONCE before pressing record. Estimated total prewarm: 5-15 minutes on first run; <5 sec on subsequent runs.
4. **Cross-reference** the companion brief `fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct` — when that lands, the cold-compile pain reduces, but the pre-warm step is still load-bearing for the demo recording.
5. **Optional**: extend `scripts/check-gateway-freshness.sh` with a "SPA-warm" check that exercises the same routes and gates the operator from recording until they're warm.

## Acceptance criteria

- [ ] `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist includes the 7-route pre-warm sequence with canonical IDs.
- [ ] `scripts/demo-prewarm.sh` (NEW) exists, curls each route, reports timing.
- [ ] Running the prewarm script then immediately clicking through the demo scenes shows no 3-13 minute blank states.
- [ ] Cross-reference to companion brief is added inline.

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record checklist)
- `scripts/demo-prewarm.sh` (NEW)
- `scripts/check-gateway-freshness.sh` (optional extension)

## Out of scope

- Reducing the actual cold-compile time (covered by `fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct`)
- Turbopack panic in dev logs (Next.js bug, not app issue)
- The audit-detail route's dependency tree (out of scope; would require code-splitting + lazy-loading the PII redactor)

## Effort

XS — ~50-100 LOC across runbook + new script. ~30-45 min. P1 because operator-only pre-record action; cheap, blocks no other work.
