# Stream P2-9 — Skill Canvas workflow diff view

**Owner**: claude.
**Depends on**: mission-pm-workflow-persistence-schema MERGED, mission-pm-workflow-edit-api MERGED, mission-pm-workflow-rollback-cli MERGED.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-skill-canvas-diff`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §7 + §11 P2 #9.

## Scope

Skill Canvas viewer at `web/app/(app)/workers/[workerId]/skills/[skillId]/canvas/page.tsx` is read-only today. Track L Phase 2 plans editor + diff + AB-promote; the report flags this as accelerable. This brief lands the **diff view** half — operators can compare any two versions of a workflow side-by-side.

Surface: at `/app/(app)/verticals/[verticalId]/workflows/[name]/diff?from=<version-id>&to=<version-id>`. Side-by-side TOML view with unified-diff annotation in a single column on narrow screens. Read-only — editor is a separate brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `web/app/(app)/verticals/[verticalId]/workflows/[name]/diff/page.tsx` (NEW) — the route + server-side fetch of both versions via the P1 #6 gateway API.
- `web/components/skycot/WorkflowDiff.tsx` (NEW) — pure-render component. Props: `fromToml: string`, `toToml: string`, `fromMeta: VersionMeta`, `toMeta: VersionMeta`.
- `web/components/skycot/WorkflowDiff.test.tsx` (NEW) — coverage.
- `web/lib/diff/unified.ts` (NEW or extend if a diff helper exists) — wrap a workspace-pinned diff library (`diff` or `jsdiff`) into a typed `compute_unified(from, to): DiffHunk[]`.
- `web/app/(app)/verticals/[verticalId]/workflows/[name]/page.tsx` — narrowly scoped: add a "Compare versions" button that links to the diff route with picked from/to. If the parent page doesn't exist yet, create a minimal viewer page that lists versions and routes into the diff.

## Files you must NOT touch

- The existing `web/app/(app)/workers/[workerId]/skills/[skillId]/canvas/page.tsx` — that's the skill viewer, not workflow viewer. Don't reuse the route.
- Any backend (gateway) code — the API is locked from P1 #6.
- `tailwind.config.*` — use existing tokens for diff highlight colors (`bg-emerald-50` / `bg-rose-50` style). No new design tokens.

## Acceptance criteria

- [ ] `/verticals/<v>/workflows/<n>/diff?from=A&to=B` renders the two versions side-by-side on ≥ md viewport, stacked on < md.
- [ ] Unified diff highlights: added lines green, removed lines red, unchanged neutral. Line numbers visible.
- [ ] When `from === to`, render "(no changes)" gracefully.
- [ ] Server-side fetch: the route handler calls the gateway API with the user's bearer token; 401/403/404 from the API surface as the appropriate Next.js error page.
- [ ] WorkflowDiff component is pure-render (no fetch); host page passes in already-fetched TOML strings.
- [ ] A11y: each diff hunk is a `<div role="region" aria-label="...">`; line numbers in `<span aria-hidden>`.
- [ ] Tests: 4 component cases — (a) no-diff renders empty-state; (b) added lines rendered green; (c) removed lines red; (d) mixed hunks rendered with correct line numbers.
- [ ] `pnpm lint && pnpm test && pnpm build` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-skill-canvas-diff`
- PR title: `web: workflow version diff view (mission-pm-skill-canvas-diff)`

## Notes for the implementer

- Diff library: prefer `diff` (the npm package) — small, well-maintained, no runtime native deps. If a different lib is already in `package.json`, use that.
- For long TOMLs, virtualize the diff view (react-window or similar) only if the test fixture exceeds 500 lines. Default to no virtualization to keep the brief tight.
- The route is server-rendered (RSC). The diff computation runs at request time, not on the client. Keep the lib-side function pure so it can run on edge.
- Out of scope: AB-promote button, edit-in-place, conflict resolution against concurrent edits.

## Out of scope

- Workflow editor (`/edit` route) — separate Track L Phase 2 brief.
- AB-test promotion between versions — separate brief.
- Inline-comment threading on diff hunks — separate brief.
