# web-workflows-live-data-binding — wire /workflows catalog + visual viewer to gateway

**Owner**: claude. **Sizing**: ~200-300 LOC (one useWorkflows hook + 2 page swaps).
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflows-live-data-binding`.
**Depends on**: gateway PR #1860 (already MERGED — workflow CRUD routes are live).
**Priority**: P1 — the /workflows catalog and /workflows/[v]/[name] visual viewer currently render static seed data. The gateway has the CRUD endpoints (POST/GET/PUT/DELETE `/api/v1/verticals/<v>/workflows[...]`) shipped in #1860. This brief swaps seed → live data.

## Scope

### 1. New hook: `useWorkflows`
Path: `web/lib/hooks/use-workflows.ts`
- `useWorkflows()` — fetch ALL workflows across ALL verticals (parallel calls)
- `useWorkflow(verticalId, name)` — fetch one workflow including versions + step spec
- TanStack Query keys: `["workflows"]` and `["workflow", verticalId, name]`
- Mock fallback: if gateway returns empty / 502, fall through to existing seed data so the demo surface still works (mark as `dataSource: "mock"`)

### 2. New API client: `web/lib/api/workflows.ts`
- `fetchAllWorkflows()` — composite call: GET /verticals → for each, GET /workflows
- `fetchWorkflow(verticalId, name)` — GET /verticals/<v>/workflows/<n>
- Types match the gateway response shape

### 3. Update `/workflows/page.tsx`
Replace `SEED_WORKFLOWS` with `useWorkflows()`. Loading skeleton + empty state + error state.

### 4. Update `/workflows/[verticalId]/[name]/page.tsx`
Replace `SEED_WORKFLOWS[key]` with `useWorkflow(verticalId, name)`. Decode the step TOML/spec into the `WorkflowStep[]` shape the visual timeline expects.

## Acceptance criteria

- [ ] `useWorkflows()` hook lands and is unit-tested.
- [ ] Catalog page shows real workflows from the gateway (or fall-through to seed if no live data).
- [ ] Visual viewer renders steps from the live workflow definition.
- [ ] Data-source pill ("LIVE" vs "MOCK") visible on the catalog hero.
- [ ] Cards link correctly to viewer; viewer renders without crashing on workflows with 0 steps.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Editing UX (covered by separate `web-workflow-propose-modal-wiring` brief).
