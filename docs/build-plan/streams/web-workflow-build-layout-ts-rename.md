# web-workflow-build-layout-ts-rename

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `02fad8a06` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-build-layout-ts-rename`.

🔴 **P0 DEMO-BLOCKER — Mission claim 3 (Platform builds workers).** Source: evaluator round-5 handoff 2026-05-23 04:10 UTC (browser layer exercised for first time via Playwright + Claude-in-Chrome MCP). Confirmed live in Chrome MCP.

**Problem.** `web/app/(app)/workflows/[verticalId]/[name]/build/layout.ts` is a **utility module** — exports `formatWaitSummary`, `stepsToFlow`, `COL_BRANCH_X`, `COL_MAIN_X`, `ROW_HEIGHT`, etc. for React Flow node positioning. **Next.js App Router treats ANY file at `app/.../layout.ts|tsx|js|jsx` as a route layout component** and demands a default-exported React component.

Result: cold dev start → `/workflows/sales/qualify_lead/build` renders as a **fully blank page** with console error:

```
Error: The default export is not a React Component in
       "/workflows/[verticalId]/[name]/build/layout"
    at errorMissingDefaultExport
```

This breaks the v1.0 demo MP4 segment 4 anchor (`/workflows/sales/qualify_lead/build`, per `docs/skycot/operator/demo-runbook.md` 04:45 line). **A cold-start demo recording from main will hit this blank screen within 4 minutes of starting.**

**Verified by evaluator round 5 via Claude-in-Chrome MCP** — the browser actually rendered the blank-page + error overlay.

## Scope

Three-step mechanical fix + a one-time guardrail against the same shape happening again:

1. **Rename the file**:
   - `web/app/(app)/workflows/[verticalId]/[name]/build/layout.ts` → `web/app/(app)/workflows/[verticalId]/[name]/build/flow-layout.ts`

2. **Update the 2 import sites** (verified by conductor pre-flight grep — exactly 2 callsites):
   - `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` line 54:
     - Before: `import { stepsToFlow } from "./layout";`
     - After:  `import { stepsToFlow } from "./flow-layout";`
   - `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderCanvasNode.tsx` line 5:
     - Before: `import { formatWaitSummary } from "./layout";`
     - After:  `import { formatWaitSummary } from "./flow-layout";`

3. **ESLint rule** in `web/eslint.config.mjs` adding a custom restriction that flags `layout.ts/.tsx/.js/.jsx` in any directory that also contains `page.tsx`. Prevents future collisions of this exact shape. Options:
   - Cleanest: a small custom rule in `web/eslint-rules/no-layout-util-collision.js` — a Node-side AST traversal of the workspace, or simpler, a directory-level structural check via `eslint-plugin-filenames` or hand-rolled.
   - Pragmatic: a `no-restricted-imports` shape that catches `./layout` imports where the file isn't a layout component — but this is brittle.
   - **Recommended**: the simplest form. A tiny script-style rule that just runs once via `eslint-plugin-no-relative-import-paths` or hand-rolled. Implementer chooses; ~30-50 LOC.

If the eslint-rule turns out to require significant scaffolding (>100 LOC), defer it to a follow-up brief (`claude-eslint-no-layout-util-collision`) and ship just the rename + import updates. The rename alone unblocks the demo.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/layout.ts` — rename to `flow-layout.ts` (`git mv`).
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — narrowly scoped: 1 import line.
- `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderCanvasNode.tsx` — narrowly scoped: 1 import line.
- `web/eslint.config.mjs` — narrowly scoped: add the rule.
- `NEW: web/eslint-rules/no-layout-util-collision.js` (optional, only if simple).
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- ANY other file outside the 4 listed above. This is a pin-point fix.
- Don't refactor `layout.ts`'s exported helpers — they work, the file just lives at a Next.js reserved path.
- Don't touch `marketing/` — different app, different convention.

## Acceptance criteria

- [ ] `git mv` history preserved on the rename.
- [ ] **Cold `pnpm dev` from `web/`**, navigate to `http://localhost:3000/workflows/sales/qualify_lead/build` — visible React Flow canvas with nodes, edges, palette, toolbar. **NOT a blank page.**
- [ ] `pnpm exec playwright test tests/e2e/builder-add-step.spec.ts` — both tests pass.
- [ ] `pnpm exec playwright test tests/e2e/workflow-publish-real.spec.ts` — all 3 tests pass.
- [ ] ESLint rule produces a violation if someone creates `app/.../foo/layout.ts` alongside `app/.../foo/page.tsx`. (If deferred per scope note, skip this and call out the follow-up brief in the PR body.)
- [ ] `pnpm typecheck` + `pnpm lint` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `web-workflow-build-layout-ts-rename`
- PR title: `fix(web): rename build/layout.ts → flow-layout.ts to avoid Next.js App Router collision (web-workflow-build-layout-ts-rename)`

## Notes for the implementer

- ~5-50 LOC expected. Pure mechanical rename + 2 import updates + (optional) ESLint rule.
- **DO NOT cold-start `pnpm dev` to verify** — the evaluator's session owns the `:3000` PID. The evaluator's handoff explicitly says "Don't restart the dev server at :3000 — the evaluator's session owns that PID". Use `pnpm build` to validate the route resolves at static-analysis level + Playwright to confirm runtime behavior.
- **The evaluator confirmed the bug live in Chrome.** This isn't theoretical — a cold-start MP4 recording on current `main` WILL hit the blank page.
- Cite the evaluator round-5 handoff in the PR body so the discovery trail is preserved.
- After this lands, the Playwright 47/49 failure cascade (per evaluator) should mostly clear — those failures are downstream of this + brief B (`claude-dev-safe-check-deps-present`).
