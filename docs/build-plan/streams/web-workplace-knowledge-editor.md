# web-workplace-knowledge-editor — Next.js page `/settings/knowledge` with markdown editor + version history + diff viewer

**Owner**: claude.
**Depends on**: `mission-workplace-knowledge-gateway` MERGED (provides the 6 HTTP endpoints this page consumes).
**Authored against**: origin/main `c613d922` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workplace-knowledge-editor`.
**Priority**: P1 — Wave A brief 4 of 6 from `docs/skycot/design/workplace-onboarding-architecture.md`. The operator-facing UI for the wiki MVP.
**Sizing**: ~1000 LOC across page + components + API client + hooks + tests. Largest brief of Wave A.

## Why this brief exists

Read design doc §2 (Pillar 1) and the demo scenario §9 step 1 ("Operator opens `/settings/knowledge/brand-voice` and edits"). The operator needs a Next.js page where they can write, version, and view the diff for the 5 categories of workplace docs.

The substrate (brief 1) + audit kinds (brief 2) + gateway (brief 3) all exist for THIS page to consume. Without this brief, the only way to edit wiki content is via `curl` to the gateway endpoints.

## Page surface

Route: `/settings/knowledge` (and sub-routes for slug detail + diff).

### `/settings/knowledge` (index)

- **Hero**: Audiowide title "Workplace Knowledge", subtitle "What every worker reads, every turn."
- **Left rail (category nav)**: 5 entries (Brand / Business Rules / Compliance / Internal / FAQ). Clicking filters the main grid. Counts per category (e.g. "Brand · 3").
- **Top right**: `+ New doc` button (gated on `useViewerPolicy().canWrite`).
- **Main grid**: card per doc — title, category pill, version chip, "Updated <relative> by <name>". Click → `/settings/knowledge/<slug>`.

### `/settings/knowledge/<slug>` (detail + editor)

- **Header**: title (editable inline if `canWrite`), category pill, current-version chip.
- **Two-column body**:
  - **Left (70%)**: markdown editor. Tabbed: `Edit` (textarea-style editor with monospace + line numbers) | `Preview` (rendered markdown). Save button at the bottom (`canWrite` gates; otherwise read-only).
  - **Right (30%)**: version-history sidebar. Each entry: HashCell-style truncated `revision_id` + relative time + `edited_by` + edit_reason (if any). Click a revision → `/settings/knowledge/<slug>/diff?from=<prev>&to=<this>`.
- **Bottom**: red `Delete doc` button (gated on `canDelete` — owner only).

### `/settings/knowledge/<slug>/diff?from=<v>&to=<v>` (lightweight viewer)

This brief ships a minimal viewer (server-side `<pre>` rendering the `diff_from_prior` unified-diff string already stored on the revision row). The full semantic diff is Wave-E brief 22 (`web-knowledge-diff-viewer`). Keep this lean — just enough so version-history clicks don't 404.

## Approach (concrete)

### Phase 1 — Markdown library decision

Pre-flight grep returned NO existing markdown editor on disk (`react-markdown`, `@uiw/react-md-editor`, `@tinymce`, `@lexical` all absent from `web/package.json`). Recommended vendoring:

- **`react-markdown`** for rendering (small, well-maintained).
- **`@uiw/react-md-editor`** for editing (compact, includes preview-tab and textarea — exactly the shape this brief needs).

Both are MIT-licensed, ~30KB gzipped combined. Add to `web/package.json` and run `pnpm install` in the worker's local tree.

Alternative: ship with a plain `<textarea>` for editing + `react-markdown` for preview (no editor lib). That's smaller and brand-consistent with SKYCOT's current "no chrome" pattern. **Recommend this alternative** — see Gotchas.

### Phase 2 — API client + hooks

1. **NEW** `web/lib/api/workplace-knowledge.ts` (~120 LOC):
   ```ts
   import { gatewayFetch } from './client'  // existing helper

   export type Category = 'brand' | 'business-rules' | 'compliance' | 'internal' | 'faq'

   export interface KnowledgeDoc {
     id: string
     slug: string
     title: string
     category: Category
     body_markdown: string
     version: number
     updated_at: string  // ISO from epoch_micros
     updated_by: string
   }

   export interface KnowledgeRevision {
     id: string
     doc_id: string
     version: number
     body_markdown: string
     diff_from_prior: string | null
     edited_at: string
     edited_by: string
     edit_reason: string | null
   }

   export async function listDocs(category?: Category): Promise<KnowledgeDoc[]>
   export async function getDoc(slug: string): Promise<KnowledgeDoc>
   export async function createDoc(input: {
     slug: string; title: string; category: Category; body_markdown: string
   }): Promise<KnowledgeDoc>
   export async function updateDoc(slug: string, input: {
     body_markdown: string; edit_reason?: string
   }): Promise<KnowledgeDoc>
   export async function deleteDoc(slug: string): Promise<void>
   export async function listRevisions(slug: string): Promise<KnowledgeRevision[]>
   ```

   **DEMO-PATH PRECONDITION**: must NOT have a sample-data fallback in `isStrictProduction()` mode (design doc §7 Pillar 1 acceptance #1). In dev mode the API CAN fall back to local sample if the gateway endpoint 404s; in prod mode the 404 propagates as an error toast. Implement via the existing `client.ts` strict-mode flag (grep for `isStrictProduction\|STRICT_MODE` in `web/lib/`).

2. **NEW** `web/lib/hooks/use-workplace-knowledge.ts` (~100 LOC) — wrapper hooks following the existing pattern in `web/lib/hooks/use-*.ts`:
   - `useKnowledgeDocs(category?)` — list with auto-revalidate.
   - `useKnowledgeDoc(slug)` — head doc + version.
   - `useKnowledgeRevisions(slug)` — version history.
   - `useUpdateKnowledgeDoc(slug)` — `mutateAsync` callback + `isSaving`.

### Phase 3 — UI components

3. **NEW** `web/app/(app)/settings/knowledge/page.tsx` (~140 LOC) — the index page. Renders category nav + grid; `'use client'`; uses the shared `WizardEditorialShell` or `BrandLoudHero` whatever the locked /settings register prescribes (check `app/(app)/settings/page.tsx` for the canonical pattern from prior /settings briefs).

4. **NEW** `web/app/(app)/settings/knowledge/[slug]/page.tsx` (~250 LOC) — the doc detail page. Edit/Preview tabs, save button, delete button, version sidebar wire-up. Uses `useViewerPolicy()` for the canWrite/canDelete gates.

5. **NEW** `web/app/(app)/settings/knowledge/[slug]/diff/page.tsx` (~80 LOC) — the lightweight diff viewer. Reads `?from=<v>&to=<v>` query params, fetches the two revisions, renders the `diff_from_prior` string in a `<pre>` with light syntax highlight.

6. **NEW** `web/components/skycot/knowledge/KnowledgeCard.tsx` (~80 LOC) — the doc card on the index grid.
7. **NEW** `web/components/skycot/knowledge/CategoryNav.tsx` (~80 LOC) — the left-rail category nav.
8. **NEW** `web/components/skycot/knowledge/VersionHistorySidebar.tsx` (~120 LOC) — the version sidebar. **Re-use HashCell** from `web/components/skycot/ActivityRow.tsx` (the truncated-hash pattern from #1902 audit-feed-per-row-hash).
9. **NEW** `web/components/skycot/knowledge/KnowledgeEditor.tsx` (~150 LOC) — the Edit/Preview tabbed editor component. If using the plain-textarea approach: textarea with monospace font + a `<MarkdownRenderer>` preview that lazy-imports `react-markdown`.
10. **NEW** `web/components/skycot/knowledge/CreateDocModal.tsx` (~120 LOC) — the `+ New doc` modal. Form fields: slug (autocomplete to kebab-case from title), title, category, initial markdown.

### Phase 4 — Tests

11. **NEW** `web/components/skycot/knowledge/KnowledgeCard.test.tsx` (~50 LOC) — renders title, category pill, version chip.
12. **NEW** `web/components/skycot/knowledge/CategoryNav.test.tsx` (~40 LOC) — clicks filter the visible cards.
13. **NEW** `web/components/skycot/knowledge/VersionHistorySidebar.test.tsx` (~60 LOC) — clicks navigate to diff page with correct params.
14. **NEW** `web/components/skycot/knowledge/KnowledgeEditor.test.tsx` (~80 LOC) — typing in Edit tab updates Preview tab content; save button disabled while in-flight.
15. **NEW** `web/app/(app)/settings/knowledge/__tests__/edit-flow.test.tsx` (~120 LOC) — integration: load doc → edit → save → optimistic update + Saved toast → revisions list grows by one.
16. **NEW** `web/app/(app)/settings/knowledge/__tests__/rbac.test.tsx` (~60 LOC) — guest role → editor is read-only + delete button hidden; owner role → both visible.

## Files you may touch (verified to exist on `c613d922`)

- **NEW** 13 files under `web/app/(app)/settings/knowledge/...` + `web/components/skycot/knowledge/...` + `web/lib/api/workplace-knowledge.ts` + `web/lib/hooks/use-workplace-knowledge.ts`.
- **MODIFY** `web/package.json` — add `react-markdown` (and `@uiw/react-md-editor` ONLY if you opt for the library route; otherwise skip).
- **MODIFY** `web/components/skycot/AppShell.tsx` (or wherever the /settings sub-nav lives) — add a "Knowledge" tab to the settings sub-tabs row. Grep first: `rg 'settings.*tab\|/settings/' web/components/skycot/`.

## Files you must NOT touch

- `crates/argo-workplace-knowledge/*` — substrate is brief 1's domain.
- `crates/argo-cli/src/handlers/gateway/*` — gateway is brief 3's domain.
- `web/lib/api/client.ts` core — only extend via the existing strict-mode flag, do not refactor the client.
- Any other `/settings/*` page — adding the Knowledge tab is a 1-line addition to the sub-tabs list.
- `web/__tests__/a11y-headings.test.ts` — the new pages must comply with the a11y heading rules from #1917; the smoke test will catch violations automatically.

## Pre-flight verification

```bash
# Confirm gateway dep landed.
gh pr view <number-of-gateway-brief> --json mergedAt --jq '.mergedAt'
# Or: git log --oneline origin/main | grep -i 'mission-workplace-knowledge-gateway merged' | head -1

# Confirm HashCell exists (revision-id truncation pattern).
rg 'HashCell' web/components/skycot/ActivityRow.tsx | head -1

# Confirm the settings sub-tabs pattern.
rg 'settings.*tabs|/settings/' web/components/skycot/AppShell.tsx | head -3

# Confirm useViewerPolicy hook + canWrite/canDelete fields.
rg 'useViewerPolicy|canWrite|canDelete' web/lib/hooks/ | head -5

# Confirm strict-mode flag location.
rg 'isStrictProduction|STRICT_MODE' web/lib/ | head -3
```

## Acceptance criteria

- [ ] All 3 pages route correctly (`/settings/knowledge`, `/settings/knowledge/<slug>`, `/settings/knowledge/<slug>/diff`).
- [ ] Category nav filters the grid; counts visible per category.
- [ ] `+ New doc` modal creates a doc → gateway POST → grid updates optimistically.
- [ ] Save updates the doc → gateway PUT → version chip increments → "Saved" toast.
- [ ] Delete (owner only) removes the doc from the grid + toast confirms.
- [ ] Version-history sidebar uses HashCell for truncated revision IDs (clickable to diff page).
- [ ] Diff page renders the stored `diff_from_prior` text correctly.
- [ ] **RBAC enforced UI-side**: guest role → editor read-only, delete hidden, +New hidden. Test with `useViewerPolicy()` mock.
- [ ] **No mock fallback in strict-production mode** — design doc §7 Pillar 1 acceptance #1. Test verifies a 404 from the gateway surfaces as an error toast, not as sample data.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm lint` clean (or pre-existing flakes documented).
- [ ] `pnpm test --run` — all new tests pass; existing suite holds (or pre-existing flakes documented per HARD RULE 2).
- [ ] `pnpm build` clean (no new server/client component-boundary errors).
- [ ] Every modified file < 500 LOC.
- [ ] a11y heading sweep auto-passes (the page hierarchy is h1 → h2 → h3 well-ordered; `__tests__/a11y-headings.test.ts` from #1917 will catch violations).
- [ ] BUILD_STATUS.md row flipped to IN_REVIEW with PR number on push.

## Branch + PR

- Branch: `claude-web-workplace-knowledge-editor`
- PR title: `web — settings/knowledge — workplace wiki editor (web-workplace-knowledge-editor)`

## Gotchas

- **Library choice**: vendoring `react-markdown` (~25KB) is a net-positive — used heavily in markdown SKYCOT surfaces already (cohort detail pages, brief previews). Adding `@uiw/react-md-editor` (~80KB) for a tabbed editor is borderline — recommend the plain-textarea route to keep the bundle lean (Wave-E brief 22's semantic-diff viewer will need react-markdown anyway).
- **Optimistic updates**: the gateway POST/PUT returns the new doc. After a successful Save, replace local state with the server response (gets the canonical `updated_at_micros` + bumped `version` + new `id` for create). For Delete, optimistically hide; on error, restore.
- **SWR or react-query**: do whichever the existing settings pages use (`rg 'useSWR\|@tanstack/react-query' web/lib/hooks/` to confirm). Don't introduce a new data-fetching lib for this brief.
- **Slug autocomplete**: title → slug via `web/lib/string/slug.ts` if it exists; otherwise inline a `title.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '')`. Allow the user to override.
- **HashCell prop shape**: confirm via `rg 'export.*function HashCell|HashCell\(' web/components/skycot/ActivityRow.tsx` so you pass the right props. Likely `<HashCell hash={revision.id} />`.
- **Edit-reason field**: the brief's gateway PUT accepts an optional `edit_reason`. Surface it as a small textarea below the body editor with placeholder "Why this edit? (optional)". Keep it under 200 chars.
- **/settings sub-tab**: per `project_settings_page_design_locked` (top-tabs design with sub-tabs), Knowledge is a NEW sub-tab. Add it to the sub-tabs row.

## References

- Design doc: `docs/skycot/design/workplace-onboarding-architecture.md` §2, §5 brief 4, §7 Pillar 1, §9 step 1 (demo scenario).
- Reference patterns:
  - `web/app/(app)/settings/api-keys/page.tsx` (from #1894) — settings sub-page structure.
  - `web/components/skycot/ActivityRow.tsx` — HashCell pattern.
  - `web/lib/hooks/use-api-keys.ts` (from #1894) — hook + API client split.
  - `web/components/skycot/approvals/ApprovalRateRing.tsx` (from #1905) — recent component-test pattern.
- Sibling briefs in Wave A:
  - **brief 3** (`mission-workplace-knowledge-gateway`) — provides the 6 endpoints this page consumes.
  - **brief 5** (`mission-knowledge-context-injection`) — reads the same docs from the agent runner; this brief edits them.
- Design constraints: `project_settings_page_design_locked` memory.
