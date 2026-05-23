# fix-workflow-build-layout-default-export

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-build-layout-default-export`.

🔴 **P1 — production-fail bug in workflow-build layout.** Source: evaluator round-25 SHARED.md handoff (Chrome MCP companion session, 2026-05-23).

> "🔴 P1 NEW (companion-session caught it; my session did not see it in console-error stream): `Error: The default export is not a React Component in "/workflows/[verticalId]/[name]/build/layout"` fires intermittently; would 500 the route in production build."

The workflow-build page's layout file is missing or mis-exporting its default React component. In dev mode this surfaces as an intermittent console error; in a production Next.js build it 500s the route. Mission claim 3 (platform builds workers) execution-side just landed via #2106; if operators try to USE the build canvas immediately after running a workflow, this is the failure they hit.

## Scope

Find the offending file at `web/app/(app)/workflows/[verticalId]/[name]/build/layout.tsx` (or its equivalent path under the route group), inspect the default export, fix it to be a valid React component.

Most likely root causes:
1. The file exports a named export but no `export default`.
2. The default export is a non-component (e.g. an object literal, a function returning a non-JSX value, or a hook).
3. The file accidentally exports something destructured from another module without re-defaulting.

For each candidate root cause, the fix is small (1-30 LOC) and well-scoped.

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/layout.tsx` (or its actual path — confirm via `find web -path '*build/layout*'`).
- The layout file's siblings ONLY if the default export's body legitimately needs to import a re-exported component.
- 1 Playwright spec under `web/tests/` that loads the route + asserts no `Error: The default export` console message + asserts the route returns 200 (not 500) in production build.

## Files you must NOT touch

- Unrelated layout files (siblings under `web/app/(app)/`).
- `next.config.*`.
- Build infrastructure.

## Acceptance criteria

- [ ] Default export resolves to a valid React component (returns JSX or `null`).
- [ ] The dev-mode console error no longer fires when loading `/workflows/<vertical>/<workflow>/build`.
- [ ] `pnpm build` in `web/` succeeds.
- [ ] `pnpm typecheck` + `pnpm lint` clean.
- [ ] New Playwright spec asserts the route renders without the default-export error.
- [ ] PR body confirms via curl-on-prod-build that the route returns 200.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- pnpm typecheck + lint + build clean.
- Playwright spec is the regression contract.
- Manual: load `/workflows/sales/qualify_lead/build` in Chrome, confirm no console error.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Refactoring the broader workflow-build canvas.
- Other Next.js layout files.
- Adding new features to the build page.

~30-100 LOC (the fix is small; the Playwright spec is the bulk).
