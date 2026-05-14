# L4 — `skill-canvas-viewer` (Track L Phase 1, Tier 2)

**Owner**: claude. **Depends on**: L1 MERGED; E0 MERGED (frontend foundation, shipped); L4-gw can land in parallel.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L4-skill-canvas-viewer`.

## Scope

Read-only Skill Canvas viewer. The first visible deliverable of Track L — operators can open any skill and see exactly what the worker knows. LOC ~1200. Branch: `claude-L4-skill-canvas-viewer`. PR title: `feat(web/canvas): read-only skill canvas viewer (L4)`.

**Specifics (verbatim from L0 spec)**:

- Route loads at `/workers/[id]/skills/[skillId]/canvas`, fetches IR via gateway (L4-gw endpoint).
- Renders the graph with React Flow (`@xyflow/react`) — operator-locked renderer choice per L0 §"Operator decisions".
- Each of the 9 node kinds has a distinct visual (icon + color).
- Side panel shows annotations for the selected node.
- Read-only — no editing affordances (L7 adds edit mode).
- Empty-state: "Worker hasn't learned this skill yet. [ Teach via observation ] [ Generate from prompt ]" — second button routes to Ghostwriter (L6).

## Files you may touch

- NEW `web/app/(app)/workers/[id]/skills/[skillId]/canvas/page.tsx` — ≤350 LOC.
- NEW `web/components/skycot/canvas/SkillCanvas.tsx` — React Flow renderer. ≤400 LOC.
- NEW `web/components/skycot/canvas/NodeRenderer.tsx` — per-kind node UI. ≤350 LOC.
- NEW `web/components/skycot/canvas/EdgeRenderer.tsx` — ≤150 LOC.
- NEW `web/components/skycot/canvas/AnnotationsPanel.tsx` — side panel. ≤250 LOC.
- NEW `web/lib/api/skill-ir.ts` — `fetchSkillIr(skillId, version?)`. ≤150 LOC.
- NEW `web/lib/hooks/use-skill-ir.ts` — ≤80 LOC.
- NEW `web/components/skycot/canvas/SkillCanvas.test.tsx` — ≥6 tests.

## Files you must NOT touch

- L1 schema types in `web/lib/types/skill-ir.ts` — read-only.
- `web/lib/types/skill-ir.ts` — locked (L1 owns).
- E0/E4 worker profile shell — locked (this brief adds a new sub-route only).

## Acceptance criteria

- [ ] Route `/workers/[id]/skills/[skillId]/canvas` loads and renders backfilled IR graphs.
- [ ] All 9 node kinds render with distinct visuals (icon + color).
- [ ] Selecting a node shows annotations in the side panel.
- [ ] Empty-state CTA wired to L6 placeholder (`/workers/[id]/skills/new`).
- [ ] 6 unit tests pass.
- [ ] `pnpm typecheck` clean in `web/`.
- [ ] Bundle delta < 25KB after adding `@xyflow/react`.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L4-skill-canvas-viewer`
- PR title: `feat(web/canvas): read-only skill canvas viewer (L4)`

## Notes for the implementer

- React Flow choice is locked per L0 §"Operator decisions" 1.
- Add `@xyflow/react` to `web/package.json` (additive dep).
- Coordinate with L4-gw sibling for the gateway endpoint shape.
- See `streams/L0-skill-canvas-track-overview.md` for full track context.
