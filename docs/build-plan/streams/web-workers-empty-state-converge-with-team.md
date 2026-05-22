# web-workers-empty-state-converge-with-team

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `7ce67c46d` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workers-empty-state-converge-with-team`.

🟢 **P3 UX CONSISTENCY — /workers and /team show divergent empty-state CTAs.** Source: evaluator round-6 SHARED.md handoff 2026-05-23T05:30:00Z (Chrome MCP comprehensive E2E).

> "web-workers-empty-state-converge-with-team — /workers shows sparse CTA but /team shows 5 starter prompts"

Context: in a fresh workspace, the user sees `/workers` with a single sparse "Hire your first worker" CTA, but `/team` shows 5 starter prompts ("Hire a SDR", "Hire a CSM", etc.). These are the SAME concept — the team IS the workers — and the divergent empty-states confuse the buyer. The 5-prompt set is the better UX (per memory `project_workers_page_design_locked.md` lock); converge `/workers` onto it.

## Scope

Audit + reconcile the two empty states:

1. **Read both pages** — `web/app/(app)/workers/page.tsx` and `web/app/(app)/team/page.tsx`.
2. **Identify the divergence** — likely `/workers` uses a generic `<EmptyState>` while `/team` uses a richer custom component (or has the 5 prompts inline).
3. **Lift the richer pattern** into a shared component (e.g. `web/components/empty-state/HireStarterPrompts.tsx`) and consume from both pages.
4. **Verify with both routes** — both should now render the 5 starter prompts when empty.

The 5 starter-prompt set should match `/team`'s existing list verbatim — DON'T invent new prompts unless `/team` has stale ones. If both pages have stale prompts, that's a separate brief; this brief just converges them.

### Regression test

- `web/__tests__/workers/empty-state-matches-team.test.tsx` — render both pages with empty data, assert both show the same 5 starter prompts.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/app/(app)/workers/page.tsx` — narrowly scoped: empty-state branch.
- `web/app/(app)/team/page.tsx` — narrowly scoped: refactor inline prompts → shared component (if needed).
- `NEW: web/components/empty-state/HireStarterPrompts.tsx` (~60 LOC).
- `NEW: web/__tests__/workers/empty-state-matches-team.test.tsx`.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The hire wizard flow (`/hire/step/*`) — out of scope.
- The non-empty-state rendering branch of either page.
- The 5 starter prompts content itself — preserve `/team`'s existing copy.

## Acceptance criteria

- [ ] Both `/workers` and `/team` render the same 5 starter prompts on empty.
- [ ] Each prompt is a clickable CTA that routes to the hire flow with the role pre-filled.
- [ ] No duplicate JSX between the two pages — both consume the shared component.
- [ ] Regression test asserts the convergence.
- [ ] `pnpm typecheck` + `pnpm lint` + vitest clean.
- [ ] All page.tsx files preserve `"use client"` if they use hooks / styled-jsx.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `web-workers-empty-state-converge-with-team`
- PR title: `fix(web): converge /workers empty state with /team's 5 starter prompts (web-workers-empty-state-converge-with-team)`

## Notes for the implementer

- ~120-200 LOC expected. Mostly a refactor + a small new component.
- **Don't reinvent — lift /team's existing pattern.** If `/team`'s prompts live inline, the smallest fix is extract to a component + consume from both. If they already live in a shared file, just import.
- **The locked design** is `/workers` = C2 holographic showcase (workers-page-design-locked). The empty-state shouldn't break that lock — show the 5 starter prompts AS holographic-styled cards, not as a sparse text list.
- Cite the evaluator round-6 report verbatim in the PR body.
