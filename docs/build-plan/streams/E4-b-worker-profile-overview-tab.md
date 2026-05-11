# Stream E4-b — Worker profile: Overview tab (charter + duties + KPIs + recent activity)

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a MERGED (route + Worker type contract).
**Authored against**: origin/main `217fd2d3` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-b-worker-profile-overview-tab`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 2 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Charter edit | Inline click-to-edit, single editor (delegator); audit-logged | Multi-stakeholder edit workflow — Department lead reviews charter changes for workers reporting through their org; auto-route via the org-hierarchy chain | `mission-enterprise-charter-multi-approval` |
| Duty editing | Toggle enabled / disabled; reorder; schedule edit | Server-side duty conflict-detection across the worker's team (don't fire two AI-Recruiter rejection-batch duties at the same hour); routing rules per duty kind | `mission-enterprise-duty-conflict-detection` (also named in E2-f) |
| KPI surfacing | Vertical-aware (E3-c headline generator); 4 fixed slots | Operator-customizable KPI selection per worker; team-roll-up KPIs visible on department dashboards | `mission-enterprise-kpi-customization` |
| Recent activity | Last 5 actions, links to audit drill-in | Same; v1 ships the audit-row links and the Activity tab (E4-c) handles the deeper view | n/a |
| Sparkline data source | Pre-computed by gateway; rendered client-side | Per-tenant retention policy on the underlying metrics; sparklines stay within data-retention window | `mission-enterprise-kpi-retention` |

v1 is acceptable: Overview is read-heavy plus the
charter / duty mutations which use the standard
approval-action audit infrastructure.

## Why

E4-a renders Overview as a placeholder. This brief
fills it in: the charter card (with inline edit + Adjust
link to E4-g), the duties list (with toggle + edit + add
via E4-h), the KPI block (4 sparklines per vertical), and
the recent activity feed (last 5 actions linking to the
Activity tab + audit drill-in).

For enterprise: each item reflects org-aware shape —
charter shows reporting-line affordance, duties show
team-coordination shape, KPIs include team-roll-up KPIs.

## Scope

- Implement Overview tab content at
  `/workers/<id>/overview` (replacing the E4-a
  placeholder).
- Four sections, top to bottom:
  - **WhatSheDoes card** — charter (3-sentence
    summary) + inline edit (delegator-permitted) +
    "✎ Adjust" button (opens RetrainDialog from
    E4-g).
  - **Duties card** — list of duty rows; each row
    toggle + label + schedule chip + click-to-edit;
    "+ Add responsibility" button (opens
    AddResponsibilityDialog from E4-h).
  - **KPIs card** — 4 KPI rows: label + current
    value + 7-day sparkline + trend arrow + tone
    (success/warning/muted).
  - **RecentActivity card** — last 5 actions, one
    line each, link to audit drill-in. "See all in
    Activity →" link at the bottom routes to the
    Activity tab (E4-c).
- Loading skeleton for each card.
- Error states per card.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(app)/workers/[workerId]/overview/page.tsx`
    (NEW)
  - The Worker type from E4-a (read-only).

## Files you may touch

- `web/app/(app)/workers/[workerId]/overview/page.tsx`
  — NEW. Renders the 4 cards. ≤ 250 LOC.
- `web/components/skycot/worker/WhatSheDoesCard.tsx`
  — NEW. Charter card with inline edit. ≤ 250 LOC.
- `web/components/skycot/worker/DutiesCard.tsx` —
  NEW. Duty list + toggle + add button. ≤ 280 LOC.
- `web/components/skycot/worker/KpiCard.tsx` — NEW.
  4-KPI block with sparklines. ≤ 220 LOC.
- `web/components/skycot/worker/RecentActivityCard.tsx`
  — NEW. Activity list. ≤ 180 LOC.
- `web/components/skycot/worker/KpiSparkline.tsx` —
  NEW. Reusable sparkline primitive (SVG-based,
  ~40px tall). ≤ 150 LOC.
- `web/lib/api/worker-overview.ts` — NEW.
  `fetchWorkerOverview(workerId)` returning charter,
  duties, KPIs, recent activity. Mock fallback. ≤
  200 LOC.
- `web/lib/hooks/use-worker-overview.ts` — NEW.
  TanStack Query hook. ≤ 100 LOC.
- `web/lib/sample-worker-overview.ts` — NEW.
  Enterprise-flavored mocks per the 6 sample workers
  from E4-a. ≤ 300 LOC.
- `web/components/skycot/worker/Overview.test.tsx`
  — NEW. ≥ 8 tests: renders all 4 cards, loading
  states, error states, inline-edit charter,
  duty-toggle, KPI sparkline renders, activity-row
  link routes, see-all routes. ≤ 350 LOC.

## Files you must NOT touch

- All E1/E2/E3 files except read-only consumption.
- `web/components/skycot/WorkerHero.tsx`,
  `WorkerTabs.tsx` — locked from E4-a.
- `web/lib/api/workers.ts`, `lib/hooks/use-worker.ts`,
  `lib/types/worker.ts` — locked from E4-a.
- Other E4 tab files (E4-c..f won't exist yet).
- `crates/**` — sibling backend brief for the
  overview endpoint.

## Acceptance criteria

- [ ] Route `/workers/<id>/overview` renders the four
      cards in order: WhatSheDoes → Duties → KPIs →
      RecentActivity.
- [ ] WhatSheDoesCard:
      - Renders worker.charter (3 sentences).
      - Inline-edit on click (only when
        `worker.permissions.canEditCharter`).
      - Save on blur/Enter; Esc cancels.
      - "✎ Adjust" button at top-right opens
        RetrainDialog (placeholder until E4-g; render
        a stub toast `"Adjust dialog ships in E4-g"`).
- [ ] DutiesCard:
      - Renders all duties as DutyRow components
        (label + schedule chip + toggle).
      - Toggle enables/disables the duty (uses
        existing toggle mutation).
      - Click duty body opens an inline editor (out
        of v1 scope; toast `"Edit duty in E4-h"`).
      - "+ Add responsibility" opens
        AddResponsibilityDialog (placeholder until
        E4-h).
- [ ] KpiCard:
      - 4 KPI rows: label, value, sparkline (7-day),
        trend arrow + delta.
      - Trend arrow color: up=success, down=warning,
        flat=muted.
      - Vertical-aware KPI selection (uses E3-c
        backend output via `worker.kpiSet[]`).
- [ ] RecentActivityCard:
      - 5 most recent actions, one line each.
      - Format: `{time} • {verb} • {status}`.
      - Click row → routes to `/audit/action/<id>`
        (route exists via E5).
      - "See all in Activity →" link at bottom
        routes to `/workers/<id>/activity`.
- [ ] Loading: each card shows its own skeleton.
- [ ] Error: per-card error card with retry CTA.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: visit overview, all 4 cards
      render with mock data; toggle a duty.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-b-worker-profile-overview-tab`
- PR title: `web(workers): Overview tab — charter + duties + KPIs + activity (E4-b)`

## Notes for the implementer

- **KpiSparkline primitive.** Pure SVG, 80×24
  px. Plots 7-day data points. No external chart
  library — keep bundle small. ≤ 30 LOC of actual
  SVG generation; the rest is layout.

- **Sample data: enterprise-flavored KPIs.** From
  E4-a's sample workers:
  - Priya (Senior Recruiter): `interviews booked
    this week`, `candidates screened today`,
    `pending rejection drafts`, `time-to-first-
    response avg`.
  - Sam (AP Specialist): `invoices processed
    today`, `payment exceptions flagged`, `pending
    approvals over $10K`, `cycle-time avg`.
  - Mariah (Vendor Manager): `vendor inquiries
    responded`, `contracts in review`, `renewals
    flagged this week`, `escalations to Controller`.
  - Jamal (Support): `tickets resolved today`,
    `avg time-to-resolution`, `tier-2 escalations`,
    `CSAT score this week`.
  - Nori (RevOps): `deals reviewed today`, `quote
    accuracy this week`, `pending Deal Desk
    approvals`, `time-to-quote avg`.

  Each value pairs with a 7-day sparkline +
  trend arrow.

- **Charter copy: enterprise-shaped sample.** For
  Priya:
  > "Priya helps the People Ops team manage
  > inbound applications. She screens candidates
  > against the role's must-haves, schedules
  > first-round interviews, and asks the hiring
  > manager before agreeing to anything outside
  > the JD."

  Note the team-orientation ("the People Ops team
  manage", "asks the hiring manager") — that's the
  enterprise-shaped framing.

- **Reuse the AppShell + WorkerHero from E4-a.**
  This brief renders only the tab body content; the
  hero + tabs strip are inherited via the layout.

- **Don't implement RetrainDialog or
  AddResponsibilityDialog here.** Stub them as
  toasts pointing to E4-g and E4-h respectively.
  Those briefs swap in the real dialogs.

- **Activity-row format**:
  ```
  14:32 · sent reply to <internal>buyer #4821</internal>
        · approved by Sam
  ```
  Use the existing `ActivityRow` primitive from the
  audit timeline (E5) once it lands. Until then,
  inline the row component in `RecentActivityCard`
  and refactor when E5-a's `ActivityRow` is
  available.

- **Permissions gating.** Inline-edit only when
  `permissions.canEditCharter`. Duty toggle only
  when `permissions.canEditDuties`. Default (E4-a
  mock data): all true for delegators.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-charter-multi-approval` —
  department-lead review for charter changes.
- `mission-enterprise-kpi-customization` — operator-
  selected KPI set per worker.
- `mission-enterprise-kpi-retention` — per-tenant
  retention policy on metrics.

Sibling backend briefs (NOT blocking; mock fallback
covers v1):

- `mission-gateway-worker-overview-endpoint` —
  `/api/v1/workers/<id>/overview`.
