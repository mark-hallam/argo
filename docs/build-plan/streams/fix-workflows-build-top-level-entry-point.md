# fix-workflows-build-top-level-entry-point

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-build-top-level-entry-point`.
**Source**: e2e-ux-audit Agent 3 finding F8 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🟡 **P1 — `/workflows/build` 404s. No way to compose a workflow from a blank canvas.**

## Root cause

The Scene 5 scope brief asks: "**Workflows builder** at `/workflows/build` — does it let an operator compose a multi-step workflow?" Answer: that URL does not resolve.

```
$ curl -I http://localhost:3000/workflows/build → 404
$ curl -I http://localhost:3000/workflows/sales/qualify_lead/build → 200
```

The builder lives at `/workflows/[verticalId]/[name]/build` — you can only EDIT an existing workflow, never compose a new one from blank. The `Propose new` button on `/workflows` opens a modal (`WorkflowProposeModal`) that proposes a workflow in plain English; it doesn't lead to a blank builder canvas.

Net effect: a user clicking the runbook's documented entry-point gets 404, then has to find their way to either the modal or an existing workflow's build subroute. Friction-side bug for any "build workflow from scratch" demo beat.

## Findings this brief closes

- Agent 3 F8 [P1] — `/workflows/build` returns 404; no top-level "build a workflow from scratch" entry-point

## Fix recipe

Three options; pick (a) for shortest path:

**(a) Thin shim page at `/workflows/build`**:
1. Author `web/app/(app)/workflows/build/page.tsx` — a small "Which vertical?" picker (Sales / Support / GTM / People-Ops / Finance), with a "Start from scratch" button per vertical.
2. Clicking a vertical routes to `/workflows/<verticalId>/__new__/build` (a new sentinel `__new__` name segment).
3. The existing builder canvas (`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`) detects the `__new__` sentinel and opens with an empty canvas + a name-the-workflow input in the inspector.

**(b) Promote the existing `WorkflowProposeModal` to the builder entry-point**: the modal already exists. Add a "Build from scratch instead" button at its bottom that routes to (a)'s new-workflow URL. Faster but creates two entry-points.

**(c) Drop `/workflows/build` from any documentation and link directly to per-vertical builder URLs**: doesn't fix the user-experience defect; rejected.

Recommendation: ship (a). Single new shim page, clear entry-point, blank canvas reachable.

## Acceptance criteria

- [ ] `/workflows/build` returns 200, not 404.
- [ ] The page presents a vertical picker (or equivalent direct-to-canvas affordance).
- [ ] Picking a vertical lands on a blank builder canvas with the inspector ready for naming.
- [ ] Saving a brand-new workflow from this entry-point persists it via `POST /api/v1/verticals/{v}/workflows` and the new workflow appears on `/workflows`.
- [ ] Curl smoke: `curl -I http://localhost:3000/workflows/build` → 200.

## May-touch

- `web/app/(app)/workflows/build/page.tsx` (NEW)
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (detect `__new__` sentinel)
- `web/components/skycot/workflows/WorkflowProposeModal.tsx` (only if Option (b) is chosen)

## Out of scope

- Sub-workflow picker using static fixture (separate brief: `fix-workflow-builder-subworkflow-picker-uses-static-fixture`)
- New step kinds (separate brief: `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- MOCK pill (separate brief: `fix-workflows-page-mock-pill-eyebrow-on-camera`)

## Effort

~80-150 LOC + 1 smoke test. ~1-2 hours.
