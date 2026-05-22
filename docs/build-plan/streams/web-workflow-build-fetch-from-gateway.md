# web-workflow-build-fetch-from-gateway

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6e5ab8d5b` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-build-fetch-from-gateway`.

🟡 **P1 MISSION-CLAIM-3 — workflow edit-loop incomplete.** Source: evaluator round-7 SHARED.md handoff 2026-05-23T06:30:00Z (workflow code-health audit across 3 layers).

> "🟡 web-workflow-build-fetch-from-gateway — page.tsx loads from STATIC SEED_WORKFLOWS fixture not gateway, save round-trips but reload shows seed → mission-claim-3 edit-loop incomplete (P1)"

The builder canvas (`/workflows/[verticalId]/[name]/build`) saves correctly to the gateway, but on **page reload** it re-reads from the static `SEED_WORKFLOWS` fixture in the page module instead of `GET /api/v1/verticals/<v>/workflows/<n>`. Result: edits silently disappear from the operator's view after a refresh. Mission claim 3 ("platform builds workers") is the demo MP4 anchor at segment 4 — this breaks the edit-loop story.

## Scope

Replace the seed-fixture load with a gateway fetch on mount:

1. Read the current load path in `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (and `useTestRun.ts` if it shares the load). Identify where `SEED_WORKFLOWS` is consumed.
2. Add a `useWorkflow(verticalId, name)` hook (`web/lib/api/workflows.ts` or extend the existing one) that does `GET /api/v1/verticals/<v>/workflows/<n>` and returns `{ data, isLoading, isError }`.
3. Replace the seed read with the hook. On `isLoading` show a skeleton; on `isError` show an EmptyState with retry; on `data` populate the canvas via `stepsToFlow`.
4. Preserve the existing save path (POST/PUT) — that already works per the evaluator.
5. Demo-mode fallback: if `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` and the gateway returns 404, fall back to the seed fixture (matches the pattern shipped in #2054 for other API surfaces). This preserves the v1.0 recorded demo replay.

### Regression test

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.test.tsx` (or sibling): mount with mock that returns gateway data → assert canvas shows gateway nodes, not seed nodes. A second test with demo-mode flag set + gateway 404 → assert seed fallback fires.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — narrowly scoped: swap seed read for hook.
- `web/lib/api/workflows.ts` — narrowly scoped: add/extend `useWorkflow` hook.
- `web/lib/api/workflows.test.ts` — extend tests.
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.test.tsx` — new vitest.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The save path (POST/PUT `/api/v1/.../workflows/<n>`) — already working.
- The gateway handlers — already correct (evaluator round-7 verified).
- `SEED_WORKFLOWS` itself — leave as the demo-mode fallback fixture.
- `stepsToFlow` / `flow-layout.ts` (renamed from layout.ts) — these are utility modules, leave alone.

## Acceptance criteria

- [ ] Edit-loop works end-to-end: save → reload → see saved edits (NOT seed).
- [ ] Demo-mode fallback restores seed on gateway 404.
- [ ] Vitest covers happy path + demo-mode fallback + isError state.
- [ ] `"use client"` preserved on page.tsx (per memory `feedback_styled_jsx_requires_use_client.md`).
- [ ] `pnpm typecheck` + `pnpm lint` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `web-workflow-build-fetch-from-gateway`
- PR title: `feat(web): builder loads workflow from gateway not seed on mount (web-workflow-build-fetch-from-gateway)`

## Notes for the implementer

- ~150-250 LOC expected. Hook ~50, page.tsx swap ~15, tests ~80.
- **Match the #2054 demo-mode-gate pattern**: same `NEXT_PUBLIC_SKYCOT_DEMO_MODE` env flag, same fallback structure as `lib/api/{audit,team,cohorts,approvals}.ts`.
- **The cross-tenant invariant is upstream**: the gateway already enforces tenant scope on `GET /workflows/<n>`. The page just needs to send the request with the auth cookie (NextAuth already plumbs this).
- Cite evaluator round-7 report verbatim in PR body.
