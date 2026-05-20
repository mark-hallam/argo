# web-command-palette-cmd-k — ⌘K command palette modal with cross-entity search

**Owner**: claude. **Sizing**: ~250-400 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-command-palette-cmd-k`.
**Priority**: **P0** — the /skills hero already advertises ⌘K (it's the locked design signature pattern). Nothing fires when you press it. This is the broken-promise the operator cannot ship over.

## Scope

A global keyboard-driven command palette accessible via ⌘K (macOS) / Ctrl+K (Windows/Linux) on every authed route.

### 1. Palette modal

- Renders as a centered overlay (Radix Dialog or equivalent — match the modal pattern used elsewhere in the app).
- Backdrop blurs the page behind (`backdrop-blur-sm bg-black/40` or matching tokens).
- Top: large search input (auto-focus on open), placeholder "Search for anything…", 18px text, with the gradient ⌘K icon to the left (signature SKYCOT pattern from the locked skills design).
- Below the input: results list, grouped by entity type.
- Footer: keyboard hints ("↑↓ navigate · ↵ open · esc close").

### 2. Searchable entity types

Each group renders with a short header label + up to 5 results:

- **Pages** — static list of all top-level routes (`/workers`, `/workflows`, `/audit`, `/settings`, `/skills`, etc.). Fuzzy match label.
- **Workers** — fetched live from gateway `/api/v1/workers` (use existing `useWorkers` hook if present; otherwise add a lightweight client-side fetch with a 30s cache).
- **Workflows** — fetched from `/api/v1/verticals/<v>/workflows` (use existing `useWorkflows` hook from `web/lib/hooks/use-workflows.ts`).
- **Skills** — fetched from `/api/v1/skills` (use existing `useSkills` hook from `web/lib/hooks/use-skills.ts`).
- **Members** — fetched from `/api/v1/members` if available; otherwise omit until v1.x with a graceful empty group.
- **Audit entries** — fetched from `/api/v1/audit?q=<query>` (use existing audit list endpoint with a query param; if no q-param support exists, fall back to client-side filter of the latest 100 entries).

### 3. Behaviors

- Open: `⌘K` / `Ctrl+K` (capture on a document-level `keydown` listener registered in a top-level layout client component; avoid double-bind).
- Close: Escape key, clicking backdrop, or selecting a result.
- Arrow Up / Arrow Down: move selection (visual highlight on hovered/keyboard-focused row).
- Enter: activate the highlighted result (navigate via `router.push` to the corresponding URL).
- Mouse hover: tracks selection.
- Empty state: "No results for '<query>'." with hint to clear.
- Loading state: subtle spinner next to each group's count while it's fetching.

### 4. Implementation

- Component: `web/components/command-palette/CommandPalette.tsx`.
- Provider: mount in `web/app/(app)/layout.tsx` so it's always available on authed routes.
- Optional: leave a separate trigger button somewhere (top-right header) but core path is the keyboard shortcut.
- Use TanStack Query to dedupe fetches across opens (results stay cached for 30s).
- Debounce search input by 150ms before refiltering.
- Limit each group to top 5 matches; "Show all N" link in the group header opens the relevant list page with the query pre-filled (if the page supports query params; otherwise just navigate to the page).

## Acceptance criteria

- [ ] ⌘K (or Ctrl+K) opens the palette modal from any authed page.
- [ ] Search input auto-focuses on open.
- [ ] Results group by entity type (Pages / Workers / Workflows / Skills / Members / Audit).
- [ ] Arrow keys navigate; Enter activates the highlighted result.
- [ ] Escape closes; clicking backdrop closes.
- [ ] Selecting a result navigates to the right URL.
- [ ] Empty state ("No results for '<query>'.") renders cleanly.
- [ ] No double-bind: pressing ⌘K twice does not stack multiple modals.
- [ ] Modal preserves focus correctly (focus trap + restore on close).
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Server-side full-text search across audit (use the existing list endpoint with a client-side filter; full-search is a separate gateway brief).
- Command-style actions (e.g. "Create workflow") inside the palette — v1.x.
- Recently-viewed / pinned items — v1.x.
