# fix-workflow-build-dirty-flag-on-initial-load

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `d3929941e` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-build-dirty-flag-on-initial-load`.

🟡 **P2 SPA polish — workflow-build editor fires `beforeunload` on INITIAL load with no edits.** Source: evaluator round-25 Chrome MCP walkthrough 2026-05-23.

> "🟡 P2 workflow build editor fires beforeunload on INITIAL load with no edits (dirty-flag seeded wrong)"

The build editor at `/workflows/[verticalId]/[name]/build` registers a `beforeunload` listener that triggers the browser's "Discard changes?" prompt even when the user has not edited anything. Root cause: the dirty-flag initial value is `true` instead of `false`, OR the state is being initialised by writing into the same store that the dirty-flag listens on, marking it dirty pre-interaction.

## Scope

1. Reproduce locally: `pnpm dev`, navigate to `/workflows/<v>/<w>/build`, refresh or hit back without editing. Confirm browser prompt.
2. Inspect the relevant component (likely under `web/app/(app)/workflows/[verticalId]/[name]/build/`). Find the dirty-flag seed.
3. Fix: initial dirty-flag must be `false`; only flip to `true` on a user-initiated change event.
4. Playwright regression: load the build page, immediately navigate away, assert no `beforeunload` event fires. Place at `web/tests/e2e/workflow-build-no-beforeunload-on-initial-load.spec.ts`.

## Acceptance

- Bug fix ≤30 LOC.
- Playwright spec green.
- Crate-scoped (web/) eslint + typecheck clean.

## Parallelism
None.

## Done when
PR opens, spec green locally, conductor admin-merges.
