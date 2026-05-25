# fix-workflow-builder-subworkflow-picker-uses-static-fixture

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-builder-subworkflow-picker-uses-static-fixture`.
**Source**: e2e-ux-audit Agent 3 finding F9 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🟡 **P1 — workflow builder's sub-workflow node picker reads from a local TypeScript fixture, never the gateway.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx:5` imports `SEED_WORKFLOWS` from `./seed-workflows` and uses it as the sub-workflow picker source:

```ts
import { SEED_WORKFLOWS } from "./seed-workflows";
…
return Object.entries(SEED_WORKFLOWS) …
```

So the operator opening the builder during Scene 5 and dragging a sub-workflow node into the canvas gets a picker populated entirely from a local TypeScript fixture, never the gateway. Other builder surfaces (the step-config inspector, the AI proposal stream) do route through real endpoints — this is a single-component leak.

Combined with `fix-workflows-page-mock-pill-eyebrow-on-camera`: if the operator opens the builder during Scene 5 and the camera hovers a sub-workflow node, they get a static-fixture list that doesn't reflect real workflows.

## Findings this brief closes

- Agent 3 F9 [P1] — Builder uses `SEED_WORKFLOWS` as a hard-coded fallback path for sub-workflow picker

## Fix recipe

1. Wire `SubWorkflowPicker.tsx` to `useWorkflows()` (already exported from `web/lib/api/workflows.ts`).
2. Treat `SEED_WORKFLOWS` as the empty-state-only fallback (gate behind `data?.workflows.length === 0`).
3. Update the picker's render to walk `data.workflows` not `Object.entries(SEED_WORKFLOWS)`.
4. Verify the loading state is graceful (skeleton row, not a frozen empty list).

```diff
- import { SEED_WORKFLOWS } from "./seed-workflows";
+ import { useWorkflows } from "@/lib/api/workflows";
…
- return Object.entries(SEED_WORKFLOWS).map(([id, w]) => ...);
+ const { data } = useWorkflows({ verticalId });
+ const items = data?.workflows.length ? data.workflows : SEED_WORKFLOWS_FALLBACK;
+ return items.map((w) => ...);
```

## Acceptance criteria

- [ ] `SubWorkflowPicker.tsx` no longer references `SEED_WORKFLOWS` as the primary data source — only as an empty-state fallback.
- [ ] Opening the builder against the live gateway with workflows seeded shows real workflow names in the sub-workflow picker.
- [ ] Component test for the picker pinning the gateway-first behavior.
- [ ] No regression on the empty state (renders gracefully when zero workflows exist).

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx` (the swap)
- `web/app/(app)/workflows/[verticalId]/[name]/build/seed-workflows.ts` (only if it needs an empty-state rename)
- Component test alongside `SubWorkflowPicker.tsx`.

## Out of scope

- `/workflows/build` top-level entry-point (separate brief: `fix-workflows-build-top-level-entry-point`)
- MOCK pill on `/workflows` (separate brief: `fix-workflows-page-mock-pill-eyebrow-on-camera`)
- Other builder fixtures (the step-config inspector + AI proposal stream are already wired correctly)

## Effort

~30-60 LOC + 1 test. ~30-45 min.
