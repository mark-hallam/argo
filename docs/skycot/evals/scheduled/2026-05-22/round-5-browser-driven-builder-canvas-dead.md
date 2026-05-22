# Eval: Round 5 — browser-driven discovery of broken workflow builder canvas

**Date**: 2026-05-23 (round 5)
**Pillar**: Workflows + cross-pillar dev hygiene
**Mission claim**: 3 — Platform builds workers
**Verdict**: 🔴 **TWO NEW P0s** — both discovered via live Chrome (could not have surfaced from curl/cargo).

Round 5 finally exercised the operator's actual surface: a real Chromium browser driven via Playwright + a real signed-in Chrome via the Claude-in-Chrome MCP. Round 1's marketing-route 200 sweep was correct in HTTP terms but missed that Turbopack's dev-mode error overlay sits in front of pages, AND missed that pages can return HTTP 200 with a blank/error body.

## Setup

- Restarted Next.js dev server at `:3000` with the Playwright-expected env (`NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:1`).
- Live gateway at `:7400` still dead; doesn't matter for this lane — Playwright's local lane is mock-fallback by design.
- Ran the full Playwright suite first (49 specs), then Chrome MCP for visual confirmation.

## P0-E — Playwright suite: 47 of 49 tests FAIL on cold dev start

```
$ cd web && SKIP_MARKETING_SERVER=1 pnpm exec playwright test --reporter=line
  47 failed
  2 passed (2.9m)
```

Failure clusters:

| Category | Specs |
|---|---|
| Approvals inbox | approvals-filter, approvals-inbox, approvals-live |
| Audit | audit-cross-system-links, audit-filter-popover |
| Auth | auth-logout (×2) |
| **Workflow builder** | builder-add-step (×2), builder-connect, builder-controlled-state, builder-delete (×3), builder-duplicate (×3), builder-shortcuts |
| Dark-mode token sweep | 4 marketing pages |
| Hire wizard | hire-celebration-banner, hire-flow-shell (×2), hire-flow-step2/3/4 |
| Onboarding | the full marketing → signup → 5-step wizard |
| Workflow CRUD | concurrent-edit, publish-real (×3), rollback (×2), save-real (×2), skill-canvas-editor |
| Team / orchestration / trust | team-filter-search, orchestration-queue, trust-promotion |

47-of-49 is not flaky — it's systemic. Each failure mode pointed at one of two root causes below.

## Root cause #1 — `layout.ts` collides with Next.js App Router convention

Driving Chrome to `http://localhost:3000/workflows/sales/qualify_lead/build` showed a **completely blank page**. Console message:

```
[EXCEPTION] (...)
Error: The default export is not a React Component in
       "/workflows/[verticalId]/[name]/build/layout"
    at errorMissingDefaultExport (next-server/app-page-turbo.runtime.dev.js)
    ...
```

The file at `web/app/(app)/workflows/[verticalId]/[name]/build/layout.ts` is a **utility module** for React Flow positioning — exports include `formatWaitSummary`, `stepsToFlow`, `COL_BRANCH_X`, etc. None of them is a React component. Next.js App Router treats any file at `app/.../layout.ts|tsx|js|jsx` as a route layout and demands a default-exported React component. The router finds none → blank page + console exception.

Two production-side consumers reference this file:

```bash
$ rg "from .*build/layout|from .\./layout\b" app/
app/(app)/workflows/[verticalId]/[name]/build/BuilderCanvasNode.tsx:
  import { formatWaitSummary } from "./layout";
app/(app)/workflows/[verticalId]/[name]/build/page.tsx:
  import { stepsToFlow } from "./layout";
```

**Fix recipe** (small, safe sweep — call it `web-workflow-build-layout-ts-rename`):

1. Rename `app/(app)/workflows/[verticalId]/[name]/build/layout.ts` → `flow-layout.ts` (or `react-flow-layout.ts`, or move into a sibling subdir like `_layout/index.ts` since Next.js ignores underscore-prefixed paths).
2. Update the two imports above to `from "./flow-layout"` (or equivalent).
3. Acceptance: `http://localhost:3000/workflows/sales/qualify_lead/build` renders the React Flow canvas (visible nodes, edges, palette, toolbar — NOT a blank page).
4. Bonus regression: a one-line ESLint rule `no-restricted-filenames` banning `layout.ts/.tsx` under any directory that also contains a `page.tsx`/`route.ts` — prevents future collisions of this exact shape.

## Root cause #2 — `@monaco-editor/react` (+ 6 transitive deps) missing from `node_modules`

Same pattern as round 1's `@radix-ui/react-tooltip`. The dep is declared in `web/package.json` (`"@monaco-editor/react": "^4.6.0"`) but absent from `node_modules`. Turbopack throws:

```
Module not found: Can't resolve '@monaco-editor/react'
File: ./components/builder/MonacoCodeEditor.tsx (2:1)

Import trace:
  ./components/builder/MonacoCodeEditor.tsx [Client Component Browser]
  ./app/(app)/workflows/[verticalId]/[name]/build/step-config-code.tsx
  ./app/(app)/workflows/[verticalId]/[name]/build/BuilderInspector.tsx
  ./app/(app)/workflows/[verticalId]/[name]/build/page.tsx
```

The error shows as a Turbopack dev-overlay modal on top of EVERY page in development, masking the underlying UI. Fixed locally with one `pnpm install --prefer-offline` round-trip:

```
$ pnpm install --prefer-offline
Lockfile is up to date, resolution step is skipped
Progress: resolved 7, reused 7, downloaded 0, added 7, done
dependencies:
+ @monaco-editor/react 4.7.0
```

**This is the SECOND occurrence of "package.json declares X, node_modules is missing X on this dev machine".** Round 1's was `@radix-ui/react-tooltip` from PR #1957. This one is `@monaco-editor/react` from a more recent PR I haven't pinned.

Recommend `claude-dev-safe-check-deps-present` — extend `scripts/dev-safe-check.sh` to:
1. After `pnpm install`, walk every top-level dependency in `web/package.json`.
2. Check `node_modules/<dep>/package.json` exists.
3. Fail loud if anything in `dependencies` (not `devDependencies`) is missing.

This would catch the issue cold-start before the operator hits a blank workflow canvas mid-demo.

## Why this matters

**Mission claim 3 — "Platform builds workers" — is the v1.0 demo MP4 segment 4 anchor.** The demo runbook calls out:

> **04:45 — On /workflows/sales/qualify_lead/build**
> "And here's the magic. Two ways to edit. Drag and drop the nodes — pan, zoom, mini-map. Just like n8n or UIpath."

This is the screen Mark spent the most design time on (locked design `project_workflow_goldens_page_design_locked.md`). **It does not render right now.** A cold-start demo recording from `main` would hit this blank page within 4 minutes of starting.

## What Chrome MCP / Playwright caught that prior rounds missed

- Round 2's HTTP-200 sweep showed `/landing`, `/audit`, etc. returning 200. **All true** — the HTTP layer works, but the rendered page is masked by a dev-error overlay because some OTHER page (the build canvas) failed to compile, and Turbopack's overlay is global.
- The cargo workflow-lifecycle smoke test from round 2 (`workflow_edit_api_smoke`) passes — because it tests the GATEWAY API, not the SPA. The blank canvas is purely a Next.js compile + naming-collision issue.
- Browser-driven evaluation is the **only** layer that catches this class of bug. It's exactly the gap I told Mark about in round 1 — "200 with hydration error renders a blank page". Now confirmed.

## Cumulative — what the evaluator → conductor loop now needs

In order of demo-MP4 leverage:

1. **🔴 `web-workflow-build-layout-ts-rename`** (NEW from round 5) — rename the conflicting `layout.ts` file. ~5 LOC change + 2 import updates. **Unblocks the workflow builder canvas, which is the demo MP4 anchor for mission claim 3.** Most urgent.

2. **🟡 `claude-dev-safe-check-deps-present`** (NEW from round 5) — extend `scripts/dev-safe-check.sh` to fail on `package.json`-declared deps missing from `node_modules`. Both round 1 and round 5 have now hit this pattern.

3. **🟡 Existing — `mission-cross-tenant-worker-handler-no-existence-leak`** (NEW from round 4, brief authored by conductor in 624f8ff5a). Still pending claim.

4. **🟡 Existing — `claude-zero-caller-substrate-lint`** (NEW from round 3, brief authored). Still pending claim.

5. **🟡 Existing — `claude-conductor-fix-by-enforcement`** (NEW from round 3, brief authored). Still pending claim.

## Bonus — what visual evaluation also confirmed

While Chrome was driving, the following design-locks render correctly (post-`pnpm install`):

- **`/`** (Approvals) — E-ops-dashboard register: 5-KPI strip + 78% donut + "All clear" empty state. ✅ Matches `project_approvals_page_design_locked.md`.
- **`/workflows`** — Brand-loud marketplace register with Audiowide hero "How your teammates actually think.", hero-search with try-pills, 4 workflow cards (post_listing/screen_candidate/approve_refund/qualify_lead). ✅ Matches `project_skills_page_design_locked.md` hero-search pattern.
- **`/workflows/sales/qualify_lead`** — Detail page with "Open builder" gradient CTA + 5 step types (TRIGGER, SKILL, DECISION, APPROVAL GATE, TOOL CALL). ✅ Matches `project_workflow_goldens_page_design_locked.md`.

The design system is holding up; the bug is naming-convention + missing dep. Three lines from a fix.
