# fix-worker-detail-link-href-undefined

**Owner**: claude.
**Authored against**: origin/main `d0ee42aee` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-link-href-undefined`.

## Why (evidence-grounded — deploy-readiness run 2026-06-08, BUG-2)
On `/workers/[workerId]` the page throws a render error: `worker detail error: prop href ... undefined` — a `<Link href={undefined}>` is rendered when some target id/slug is absent. The ErrorBoundary catches it so the heading still shows, but a sub-link is broken and the console error is real. A signed-in customer's worker-detail page must render clean.

## Grep-before-write (find the exact offending Link)
- `rg -n '<Link' web/app/\(app\)/workers/\[workerId\] web/components/skycot/worker*` — locate the `<Link href={...}>` whose `href` can resolve to `undefined` (e.g. a related-worker, team, cohort, or skill link built from a possibly-absent field).
- Reproduce: load `/workers/<id>` for a worker missing that field (e.g. the freshly-hired worker with no team/cohort), watch the console for `prop href ... undefined`.

## Files you may touch
- the worker-detail page/component under `web/app/(app)/workers/[workerId]/` (and/or a child component it renders).

## Fix
Guard the link: render the `<Link>` only when the href target is present (conditional render), OR fall back to a non-link element, OR provide a safe default href. Do NOT render `<Link href={undefined}>`. Keep the layout stable when the link is absent.

## Acceptance (truth-bounded)
1. `/workers/<id>` for a worker MISSING the target field renders with ZERO `prop href ... undefined` console errors (capture the clean console).
2. When the target IS present, the link still works (navigates correctly).
3. Add a small test if the codebase has a harness for this component (assert no undefined-href link is produced for a worker with the field absent).
4. `pnpm -C web exec tsc --noEmit` = 0; touched vitest green.

## On ship (same commit)
Note in the PR it clears deploy-readiness BUG-2.

## Effort
S (one guarded Link + a test).
