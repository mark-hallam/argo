# Stream E4-a — Worker profile: route + hero + tabs shell

**Owner**: claude.
**Depends on**: E0 MERGED.
**Authored against**: origin/main `3dbcc5b4` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-a-worker-profile-shell`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 (worker profile) brief 1 of 9. **CONTRACT PR for E4** — downstream E4-b..i briefs implement tabs against the route + state shape locked here.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Worker data shape | Flat `Worker` (tenant + id + persona); team / department fields nullable in v1 | Full org hierarchy `Worker → Team → Department → Tenant`; approval routing chains derive from the chain | `mission-enterprise-org-hierarchy` (named in Track I) |
| Tab routing | Sub-routes `/workers/<id>/<tab>` with URL state | Same. Tabs gate behavior is RBAC-aware in target (e.g. only operators see Settings tab) | `mission-enterprise-worker-profile-rbac-tabs` |
| Operator view vs delegator view | Same tabs for all viewers in v1 | Different tab sets per role: delegator sees Overview/Activity/Memory/Skills; operator additionally sees Settings + Audit-actions; auditor sees Audit-only read-only | `mission-enterprise-worker-profile-role-views` |
| Worker fetch | `GET /api/v1/workers/<id>` with mock fallback | Mandatory server endpoint; tenant-scoped; RBAC-filtered fields | `mission-enterprise-worker-detail-mandatory` |
| Cross-tenant access | Single-tenant assumed | Operator working across tenants must explicitly switch tenant context; cross-tenant peek requires audit row | `mission-enterprise-cross-tenant-access` |

v1 is acceptable: the page is read-heavy + 2 mutations
(rename, charter-edit). Both go through standard audit;
neither moves money or data outside the tenant.

## Why

The worker profile ([wireframe-worker-profile.md](../../skycot/ux-research/wireframes/wireframe-worker-profile.md))
is the deep view of one AI teammate. 5 tabs: Overview /
Activity / Memory / Skills / Settings. This brief lands
the skeleton — route, hero card, tab navigation, breadcrumb,
worker fetch with mock fallback. Each tab is its own
follow-on brief (E4-b through E4-f); tab content is empty
in this brief.

For the **enterprise lens**: the hero card surfaces the
worker's **department** + **team** alongside name + role,
so an operator viewing the page can immediately see where
this worker sits in the org. v1 makes those fields
optional + falls back to "—" when nullable.

## Scope

- NEW route `/workers/<workerId>` under `(app)/` with
  default tab `Overview`.
- Tab sub-routes: `/workers/<workerId>/<tab>` for
  `overview` / `activity` / `memory` / `skills` /
  `settings`. URL state preserved across tab clicks.
- Breadcrumb: `Team › <worker.name>`.
- Hero card:
  - Avatar (lg, with status ring)
  - Worker name + role tagline
  - **Department + Team** (when present)
  - Status pill (Working / Waiting / Idle / Blocked /
    Paused)
  - Today's KPI summary (one-line)
  - Overflow menu (⋮): Pause, Disable duties, Archive,
    Export data, View as Operator
- Tabs strip below hero, 5 tabs labeled Overview /
  Activity / Memory / Skills / Settings.
- Each tab renders a placeholder `"<tab> — coming in
  E4-<x>"` block.
- `useWorker(workerId)` hook with mock fallback.
- Worker-not-found state (404 inside AppShell).
- Permission-denied state (when cross-tenant access
  refused).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Shared hotspots: `web/app/(app)/workers/[workerId]/**`
  (NEW directory tree).

## Files you may touch

- `web/app/(app)/workers/[workerId]/layout.tsx` — NEW.
  Worker-scoped layout: fetches worker, renders hero +
  tab strip. Children render as the tab content. ≤ 200
  LOC.
- `web/app/(app)/workers/[workerId]/page.tsx` — NEW.
  Default route → redirects to `/overview` tab. ≤ 30
  LOC.
- `web/app/(app)/workers/[workerId]/[tab]/page.tsx` —
  NEW dynamic. Validates tab is one of the 5; renders
  per-tab placeholder. Returns 404 for invalid tabs.
  ≤ 200 LOC.
- `web/components/skycot/WorkerHero.tsx` — NEW. The
  hero card composite. ≤ 280 LOC.
- `web/components/skycot/WorkerTabs.tsx` — NEW. Tab
  strip with active-tab highlight from `usePathname()`.
  Uses shadcn Tabs primitive. ≤ 150 LOC.
- `web/components/skycot/WorkerNotFound.tsx` — NEW.
  Inside-AppShell 404 with link back to Team. ≤ 100
  LOC.
- `web/lib/api/workers.ts` — narrowly scoped: extend
  with `fetchWorker(workerId)`. Existing pause/resume
  + hireWorker from E2-e/E3-b untouched. ≤ 300 LOC
  after.
- `web/lib/hooks/use-worker.ts` — NEW. `useWorker(id)`
  TanStack Query hook. ≤ 100 LOC.
- `web/lib/types/worker.ts` — narrowly scoped: extend
  the `Worker` type with optional `department`,
  `team`, `kpiToday`, `permissions` fields. Keep all
  E3-a fields intact. ≤ 200 LOC after.
- `web/lib/sample-workers-detail.ts` — NEW. Detailed
  mock workers (matching E3-a's `sample-team` ids but
  with richer fields for the profile page). Enterprise-
  flavored personas:
  - Priya — Senior Recruiter (People Ops team, People
    & Culture dept)
  - Jamal — Tier 1 Support (Customer Experience team,
    Customer Success dept)
  - Nori — RevOps Analyst (Deal Desk team, Revenue
    Operations dept)
  - Sam — AP Specialist (Accounts Payable team,
    Finance dept)
  - Mariah — Vendor Manager (Procurement team,
    Operations dept)
  - + 1 inert example (no department; recently hired,
    no connectors).
  ≤ 250 LOC.
- `web/components/skycot/WorkerHero.test.tsx` — NEW.
  ≥ 6 tests: renders all fields, department fallback,
  status pills, overflow menu opens, breadcrumb route,
  permission-denied state. ≤ 250 LOC.

## Files you must NOT touch

- All E1/E2/E3 files except the narrow E3-a-types
  extension (Worker type) and E3-b's
  pause/resume mutations (re-used here).
- `crates/**` — sibling backend brief for
  `/api/v1/workers/<id>` and the org-hierarchy schema
  not in this PR.

## Acceptance criteria

- [ ] `/workers/<workerId>` route loads.
- [ ] Default route redirects to `/overview` tab.
- [ ] Sub-routes `/overview` / `/activity` / `/memory`
      / `/skills` / `/settings` all load and show the
      placeholder block for that tab.
- [ ] Invalid tab (e.g. `/workers/abc/banana`) returns
      404 inside AppShell.
- [ ] Worker not found → `<WorkerNotFound>` with
      copy: Title `"Worker not found"` · Body `"The
      worker '${id}' doesn't exist or you don't have
      access."` · CTA `"Back to team"`.
- [ ] Hero card renders:
      - Avatar (lg) with status ring per worker.status
      - Name `font-semibold text-base` + role tagline
        muted
      - Department + Team (only when present): inline,
        muted, format `Department · Team`
      - Status pill (matches TeammateCard pill)
      - One-line KPI summary (e.g. `$340 today`,
        `12 tickets resolved today`)
      - ⋮ overflow menu with menu items: Pause / Disable
        duties / Archive / Export data / View as
        Operator
- [ ] WorkerTabs strip:
      - 5 labeled tabs.
      - Active tab styled via accent border-bottom.
      - Inactive tabs muted.
      - Clicking a tab routes to its sub-route.
      - Hover tab → cursor pointer + subtle background
        change.
- [ ] Breadcrumb above hero: `Team › <worker.name>`
      where `Team` is a link to `/team`.
- [ ] `useWorker(id)` returns mock fallback when
      gateway 404s; production hard-fails on 404.
- [ ] Mock dataset has 6 enterprise-flavored personas
      with department + team fields.
- [ ] Tab content placeholder copy per tab:
      - `"Overview — coming in E4-b."` etc. Wired to
        the brief id so future readers can find the
        implementation.
- [ ] Pause/resume from the overflow menu uses the
      existing E3-b mutations.
- [ ] 6 unit tests pass.
- [ ] Playwright e2e: navigate to /workers/<id>,
      assert hero renders, click each tab, assert
      tab placeholder visible.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-a-worker-profile-shell`
- PR title: `web(workers): profile shell — route + hero + tabs (E4-a CONTRACT)`

## Notes for the implementer

- **Worker type extension** (locked):
  ```ts
  export interface Worker {
    id: string;
    name: string;
    avatarUrl?: string;
    role: { tagline: string; vertical: string };
    status: WorkerStatus;
    // E4-a additions (all optional for v1 backwards
    // compat with E3-a's leaner type):
    department?: { id: string; name: string };
    team?: { id: string; name: string };
    kpiToday?: { label: string; value: string };
    permissions?: {
      canEditCharter: boolean;
      canEditDuties: boolean;
      canPauseResume: boolean;
      canArchive: boolean;
      canDelete: boolean;          // operator-only in target
      canViewAudit: boolean;
      canViewSettings: boolean;    // operator-only in target
    };
    // existing E3-a fields below
    headline?: { bullets: string[]; tomorrow?: string };
    lastActiveAt?: string;
    pendingApprovalCount?: number;
  }
  ```

- **Tab routing pattern.** Each tab is its own sub-route.
  Active state read from `usePathname()`:
  ```ts
  const segments = usePathname().split("/");
  const activeTab = segments[3] ?? "overview";
  ```

- **Sample-workers-detail** structure — each worker
  matches an E3-a `sample-team` entry by id, with
  these added fields:
  ```ts
  {
    id: "priya",
    name: "Priya",
    department: { id: "people-and-culture", name: "People & Culture" },
    team: { id: "people-ops", name: "People Ops" },
    role: { tagline: "Senior Recruiter (AI)", vertical: "recruiter" },
    status: "working",
    kpiToday: { label: "interviews booked", value: "3" },
    permissions: { ...all true for delegators },
    ...
  }
  ```
  Names are enterprise-shaped — "Priya" is a recruiter
  ON a team, not "Priya the solo recruiter". The
  difference is in the `department` + `team` fields.

- **Hero rendering hierarchy** — name is primary,
  role/department/team are secondary text. Don't
  bury the worker's identity in the corporate
  hierarchy.

- **Operator vs delegator visibility.** v1 shows all
  tabs to all authenticated viewers. The
  permissions field on Worker lets downstream tabs
  (especially E4-f Settings, E4-c Activity) gate
  visibility/editability per field. Don't enforce
  visibility at the tab-strip level in v1 — render
  the tabs but make their content respect
  permissions in their own briefs.

- **Breadcrumb routing.** "Team" links to `/team`,
  not to "the worker's team within the org". The
  worker's team is shown in the hero, not the
  breadcrumb.

- **Overflow menu items in v1.** Pause + Disable
  duties + Archive (reversible, 30d recovery) are
  available to delegators. Export data is GDPR-
  scoped and lives in the audit-export flow (E5).
  View as Operator is operator-only — surfaces a
  different view of the same page (no impl in v1;
  just a placeholder menu item with "(coming soon)"
  badge).

- **Don't implement Pause/Resume mutations again.**
  Reuse `useWorkerActions()` from E3-b.

## Hardening briefs implied (Track H + Track I)

NOT blocking this PR:

- `mission-enterprise-worker-profile-rbac-tabs` —
  tab visibility per role.
- `mission-enterprise-worker-profile-role-views` —
  delegator / operator / auditor distinct views.
- `mission-enterprise-worker-detail-mandatory` —
  mandatory server endpoint.
- `mission-enterprise-cross-tenant-access` — explicit
  tenant switch + audit on cross-tenant access.

Track I (enterprise product alignment):

- `mission-enterprise-org-hierarchy` — Worker → Team
  → Department → Tenant primitives + schema.

Sibling backend briefs (NOT blocking; v1 works on
mock fallback):

- `mission-gateway-worker-detail-endpoint` — GET
  `/api/v1/workers/<id>` server-side.
- `mission-gateway-org-hierarchy-schema` — schema +
  migration for team + department.
