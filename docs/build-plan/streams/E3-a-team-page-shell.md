# Stream E3-a — Team page: route + sticky header + responsive grid shell

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED.
**Authored against**: origin/main `fe59d7da` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E3-a-team-page-shell`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E3 brief 1 of 5.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Team data source | GET `/api/v1/team` with mock fallback (returns SAMPLE_TEAM list) when gateway 404s in dev | Mandatory server endpoint; tenant-scoped; permission-filtered; no client fallback in production | `mission-enterprise-team-list-mandatory` |
| RBAC | All authenticated users see all workers in their workspace | Per-role visibility: delegators see only the workers they own; operators see all in tenant; cross-tenant view requires explicit cross-tenant grant | `mission-enterprise-team-rbac-scoping` |
| Pagination | Returns all workers; client renders all | Server-paginated at 50 per page when workspace has > 50 workers | `mission-enterprise-team-pagination` |
| Audit | None | Every team-list query audit-logged (who looked at whose team, when) | `mission-enterprise-team-audit` |

v1 is acceptable because team-list visibility within a
single tenant is low-risk in early deployments. The
architecture (RBAC-aware filter at the gateway) is there;
v1 implementation just doesn't enforce it client-side.

## Why

The Team page ([wireframe-team-page.md](../../skycot/ux-research/wireframes/wireframe-team-page.md))
is the "meet your AI teammates as faces" surface. It's
the 2nd nav item after Approvals; clicking it shows a
responsive grid of TeammateCards.

E3-a lands the shell — route, sticky header (title +
filter + hire CTA), responsive grid layout, empty state
when zero teammates. The TeammateCard composite itself
ships in E3-b; this brief renders just placeholder cards
in the grid.

## Scope

- NEW route `/team` under `(app)/`.
- Page renders:
  - Sticky header: title `"Team"` + subtitle `"{n}
    teammates working"` + status filter dropdown +
    Hire CTA.
  - Search input below header (filters by name).
  - Responsive grid (1/2/3 columns) of placeholder
    cards.
  - Empty state when zero teammates.
- `useTeam()` hook with mock fallback (same pattern as
  E1-a's `useApprovals`). Returns `Worker[]`.
- `Worker` type definition (lives in
  `lib/types/worker.ts`).
- Active nav item highlighted in the sidebar.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/team/page.tsx`
  (NEW; locks until merge).

## Files you may touch

- `web/app/(app)/team/page.tsx` — NEW. Main team page.
  ≤ 350 LOC.
- `web/lib/types/worker.ts` — NEW. `Worker` type +
  `WorkerStatus` + related enums. ≤ 100 LOC.
- `web/lib/api/team.ts` — NEW. `fetchTeam()` with mock
  fallback. ≤ 150 LOC.
- `web/lib/hooks/use-team.ts` — NEW. `useTeam()` hook
  wrapping `fetchTeam` via TanStack Query. ≤ 100 LOC.
- `web/lib/sample-team.ts` — NEW. 4-6 sample workers
  for mock fallback. ≤ 200 LOC.
- `web/components/skycot/TeammateGrid.tsx` — NEW.
  Responsive grid wrapper (1/2/3 cols). ≤ 100 LOC.
- `web/components/skycot/TeammateCardSkeleton.tsx` —
  NEW. Skeleton render of a card while loading.
  Real card composite ships in E3-b. ≤ 100 LOC.
- `web/lib/api/team.test.ts` — NEW. ≥ 4 tests:
  happy path, 404 → mock, 5xx → throw, auth-bypass
  → mock. ≤ 150 LOC.
- `web/app/(app)/team/page.test.tsx` — NEW. ≥ 4 tests:
  renders header, renders grid with sample workers,
  empty state when zero, search filters. ≤ 200 LOC.

## Files you must NOT touch

- All E1/E2 files — locked.
- `web/components/skycot/ApprovalCard.tsx` — locked.
- `web/components/skycot/AppShell.tsx` — locked from
  E0 (nav item already exists; this brief just makes
  the route real).
- `crates/**` — sibling backend brief for `/api/v1/team`
  endpoint, not in this PR.

## Acceptance criteria

- [ ] `/team` route loads and renders the Team page.
- [ ] Sidebar nav item "Team" highlights as active when
      on `/team`.
- [ ] Sticky header: title `"Team"`, subtitle `"{n}
      teammates working"` (hidden when n=0), status
      filter dropdown (Strings: `"Status v"`,
      Options: All / Working / Waiting on you / Idle /
      Blocked / Paused), `[ + Hire ]` primary button
      (routes to `/hire`).
- [ ] Search input below header. Strings: placeholder
      `"Filter by name..."`.
- [ ] Responsive grid:
      - `< 640px`: 1 column
      - `640-1024px`: 2 columns
      - `> 1024px`: 3 columns
- [ ] Grid renders 4-6 skeleton placeholder cards
      from the mock dataset.
- [ ] Loading state: 6 skeleton cards.
- [ ] Empty state (zero teammates):
      - Title: `"No teammates yet."`
      - Body: `"What do you need help with day-to-day?"`
      - CTA: `[ + Hire your first teammate ]`
      - Tagline: `"It takes about 5 minutes."`
- [ ] Search filter updates URL `?q=` and filters the
      grid client-side (matches against worker.name +
      worker.role.tagline).
- [ ] Status filter updates URL `?status=` and filters
      the grid client-side.
- [ ] Empty-state variants per failed filter:
      - `"No teammates in this state right now."`
        (status filter, no match)
      - `"No matching teammates"` + `"Nothing matches
        \"${query}\". Try a different search or clear
        the filter."` (search no match)
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: navigate to /team, assert grid
      renders, search filters, status filter filters.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E3-a-team-page-shell`
- PR title: `web(team): page shell — route + header + grid (E3-a)`

## Notes for the implementer

- **Worker type** (locked):
  ```ts
  export type WorkerStatus =
    | "working"
    | "waiting"   // waiting on the user's approval
    | "idle"
    | "blocked"
    | "paused";

  export interface Worker {
    id: string;
    name: string;
    avatarUrl?: string;
    role: { tagline: string; vertical: string };
    status: WorkerStatus;
    headline?: {
      bullets: string[];            // "8 replies sent", "$340 made"
      tomorrow?: string;            // "morning routine at 06:30"
    };
    lastActiveAt?: string;          // ISO
    pendingApprovalCount?: number;  // surfaces in nav badge
  }
  ```
  Worker.headline is rendered by the TeammateCard composite
  (E3-b); E3-a just stores the data.

- **Sample team** (`lib/sample-team.ts`) — 4-6 workers
  matching the 5 archetypes from the personas doc + an
  inert one. Same fixed-timestamp pattern as
  `sample-approvals.ts` to avoid SSR/CSR hydration
  mismatch.

- **Sort order.** Workers in the grid sort by:
  1. `waiting` first (urgent at top)
  2. `working` next
  3. `idle` and `paused` after
  4. `blocked` at bottom
  Within each group: by `lastActiveAt` descending.

- **Pending approval count.** If `pendingApprovalCount
  > 0`, the worker's row shows it on the status pill.
  E3-b implements the visual; E3-a just provides the
  data.

- **Don't implement the TeammateCard composite here.**
  Render a `TeammateCardSkeleton` placeholder in each
  grid cell. The composite ships in E3-b and slots into
  this grid without changing the grid layout.

- **Mock fallback gating** mirrors E1-a:
  - 404 from gateway → `dataSource = "mock-offline"`,
    returns SAMPLE_TEAM.
  - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → mock
    immediately.
  - Production builds (`NODE_ENV === 'production'`
    AND `NEXT_PUBLIC_ENV === 'production'`): no mock
    fallback; 404 surfaces as error.

- **DataSourceChip from E1-a.** Reuse it here too —
  position next to the page subtitle. Same visibility
  rules.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-team-list-mandatory` — server
  endpoint always present in prod.
- `mission-enterprise-team-rbac-scoping` — per-role
  visibility filtering.
- `mission-enterprise-team-pagination` — server-side
  pagination.
- `mission-enterprise-team-audit` — audit log on
  team-list queries.

Sibling backend brief:

- `mission-gateway-team-list-endpoint` — `/api/v1/team`
  GET endpoint.
