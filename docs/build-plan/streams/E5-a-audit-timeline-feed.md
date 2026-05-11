# Stream E5-a — Audit timeline: route + activity feed + search

**Owner**: claude.
**Depends on**: E0 MERGED; E4-c MERGED (ActivityRow primitive).
**Authored against**: origin/main `c468104e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-a-audit-timeline-feed`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 1 of 8. **CONTRACT PR for E5** — locks the audit-timeline route + state shape that E5-b..h reference.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Audit-row source | `GET /api/v1/audit?<filters>` with mock fallback | Mandatory server endpoint; tenant-scoped; viewer's RBAC governs visible fields; redaction-aware | `mission-enterprise-audit-feed-mandatory` |
| Pagination | Cursor-based, 50 rows per page | Same; v1 ships the contract | n/a |
| Field redaction | Defense-in-depth via E1-e on ActivityRow | Server-side first fence (mandatory); per-viewer RBAC determines unredacted vs sentinel | `mission-enterprise-audit-pii-server-fence` (already named) |
| Cross-tenant view | Single-tenant for v1 | Operator tenant-switch context + explicit cross-tenant audit-grant | `mission-enterprise-cross-tenant-audit` |
| Search scope | Free-text on worker name + verb + action id | Full-text search across audit-row meta (per-tenant retention policy on indexing) | `mission-enterprise-audit-full-text-search` |
| Export gating | Single-action export from drill-in; bulk export from `/audit` (E5-e) | Operator-gated exports for sensitive scopes (PII-heavy, large bundles); compliance-officer role required | `mission-enterprise-audit-export-rbac` |

v1 is acceptable: feed reads from existing audit-row
infrastructure; the visible-field RBAC already exists
in mission-compliance-soc2-audit-trail MERGED.

## Why

The audit timeline ([wireframe-audit-timeline.md](../../skycot/ux-research/wireframes/wireframe-audit-timeline.md))
is the forensic + compliance surface — operator's
primary investigation tool, auditor's quarterly
evidence path. This brief lands the route + activity
feed + search; E5-b adds the filter popover, E5-c the
drill-in, E5-d the replay sandbox, E5-e the export
flow, etc.

## Scope

- NEW route `/audit` under `(app)/`.
- Page renders:
  - Page header: title `"Audit"` + subtitle `"Forensic
    + compliance view"`.
  - Action row: Filter button (icon-only;
    implementation in E5-b), time-range dropdown
    (5 options), `[ ⤓ Export ]` button (opens E5-e
    dialog when ready; placeholder toast for now).
  - Search input (full-width below header).
  - Server-paginated timeline of ActivityRow
    instances (reuses the component from E4-c).
- 50 rows per page; "Load more" at bottom; cursor-
  based pagination.
- Click row → routes to `/audit/action/<id>` (E5-c
  surface).
- Active sidebar nav highlights "Audit".
- Time-range dropdown updates URL `?range=` and
  refetches.
- Search input debounced 200ms; URL `?q=`.
- URL state for `?worker=`, `?action_type=`, `?status=`
  (set by deep links from worker profile / approvals
  card), preserved across navigation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Shared hotspots: `web/app/(app)/audit/page.tsx`
  (NEW)

## Files you may touch

- `web/app/(app)/audit/page.tsx` — NEW. Main audit
  feed page. ≤ 380 LOC.
- `web/lib/api/audit.ts` — NEW. `fetchAuditFeed(filters,
  cursor)` with mock fallback. ≤ 250 LOC.
- `web/lib/hooks/use-audit-feed.ts` — NEW. TanStack
  Query infinite-query hook for cursor pagination.
  ≤ 200 LOC.
- `web/lib/types/audit.ts` — NEW. `AuditRow`
  + `AuditFilters` + `AuditCursor` types. ≤ 150 LOC.
- `web/lib/sample-audit.ts` — NEW. Enterprise-flavored
  sample audit rows (cross-worker) spanning the 6
  E4-a workers. ≤ 350 LOC.
- `web/components/skycot/audit/AuditPageHeader.tsx` —
  NEW. Title + action row + search. ≤ 250 LOC.
- `web/components/skycot/audit/AuditDayGroup.tsx` —
  NEW. Day-header divider + rows. ≤ 100 LOC.
- `web/app/(app)/audit/page.test.tsx` — NEW. ≥ 8
  tests: header renders, time-range filter, search,
  pagination load-more, row click routes, empty
  state, error state, URL state round-trip. ≤ 350
  LOC.

## Files you must NOT touch

- `web/components/skycot/ActivityRow.tsx` — locked
  from E4-c. Reused here.
- All E1/E2/E3/E4 files except read-only consumption.
- `web/components/skycot/AppShell.tsx`,
  `NavDrawer.tsx` — locked (sidebar nav item for
  "Audit" already exists from E0).
- Other E5 sibling briefs — they'll author their
  own files.
- `crates/**` — sibling backend brief for `/api/v1/audit`.

## Acceptance criteria

- [ ] `/audit` route loads.
- [ ] Sidebar "Audit" nav item highlights as active.
- [ ] Page header renders: title `"Audit"`, subtitle
      `"Forensic + compliance view"`, action row with
      [Filter] (placeholder until E5-b), time-range
      dropdown, `[ ⤓ Export ]` (placeholder until
      E5-e).
- [ ] Time-range dropdown options: Last hour / Today
      (default) / Yesterday / Last 7 days / Last 30
      days / This quarter / Last quarter / Custom.
- [ ] Custom range opens a date-range picker
      (placeholder for v1; render `"Custom range
      picker ships in E5-b"` toast).
- [ ] Search input below header; placeholder
      `"Search action, worker, or action ID..."`;
      debounced 200ms.
- [ ] Timeline renders with day-group headers
      ("TODAY", "YESTERDAY", "MAY 9", etc.) +
      ActivityRow instances under each.
- [ ] ActivityRow click → routes to
      `/audit/action/<actionId>` (E5-c surface;
      placeholder route returns 404 until E5-c
      ships).
- [ ] Cursor pagination:
      - Initial load: 50 most-recent rows.
      - `[ Load more — page ${n+1} ]` at bottom of
        the list.
      - Disabled state during fetch.
- [ ] URL state via `?range=`, `?q=`, `?worker=`,
      `?action_type=`, `?status=`. Filters that come
      in via deep link (e.g. `/audit?worker=mariah`)
      are applied immediately and surface as filter
      chips in the chip rail (E5-b implements; v1
      renders a simple chip).
- [ ] Empty state: title `"Nothing's happened yet."`
      / `"No actions match"` per state. Body per
      wireframe.
- [ ] Error state: dashed-border card with retry CTA.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: visit /audit, see feed, time-
      range change refetches, search filters, click
      row routes (asserts 404 placeholder for now).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-a-audit-timeline-feed`
- PR title: `web(audit): timeline feed + search + time-range (E5-a CONTRACT)`

## Notes for the implementer

- **AuditRow type** (locked):
  ```ts
  export interface AuditRow {
    id: string;                  // act_01H...
    timestamp: string;           // ISO
    worker: { id: string; name: string; avatarUrl?: string };
    actionType: string;          // 'message_sent', 'invoice_processed', etc.
    verb: string;                // plain English summary
    status: "succeeded" | "refused" | "pending" | "failed" | "autonomous";
    approver?: { id: string; name: string };
    channel?: ChannelKind;
    cost?: { usd: number };
    dataClassification?: "public" | "internal" | "confidential" | "restricted";
  }
  ```

- **AuditFilters shape:**
  ```ts
  export interface AuditFilters {
    range?: "1h" | "today" | "yesterday" | "7d" | "30d" | "qtd" | "lq" | { from: string; to: string };
    q?: string;
    worker?: string;
    actionType?: string;
    status?: string;
    cursor?: string;
  }
  ```

- **Day-group headers.** Group rows by calendar day in
  the workspace timezone. Headers: `"TODAY"`,
  `"YESTERDAY"`, then `"MAY 9"` (locale-formatted
  short date with year omitted when current year).

- **Sample data scenarios — enterprise-flavored.**
  Spread across the 6 E4-a workers:
  - Priya (Recruiter): "screened Diego Alvarez against
    JD-2024-08" / "booked Thursday interview" /
    "drafted rejection email to Sara Lin"
  - Sam (AP Specialist): "extracted INV-2024-08741
    line items" / "flagged duplicate invoice
    INV-2024-08739" / "queued $14,250 for CFO
    approval"
  - Mariah (Vendor Manager): "drafted RFP response
    summary" / "scheduled renewal review with Acme
    Tech"
  - Jamal (Support): "resolved SUP-29382 with KB
    SK-413" / "escalated refund to support lead"
  - Nori (RevOps): "validated QT-2024-7283" /
    "flagged discount for Deal Desk"
  - 50+ such rows; mix of statuses, costs, channels.

- **Cursor pagination.** Returns `{ rows: AuditRow[],
  nextCursor?: string }`. `nextCursor` null means no
  more pages. The `useAuditFeed()` hook uses
  TanStack `useInfiniteQuery` with `getNextPageParam`.

- **Search debounce.** 200ms (heavier than the
  approvals search because the corpus is larger).

- **Don't implement the filter popover here.** E5-b
  takes that lock. This brief renders only the
  trigger button + the basic time-range dropdown.

- **Don't implement the export dialog here.** E5-e
  takes that lock. This brief renders only the
  trigger button.

- **Don't implement the drill-in here.** E5-c. Just
  link to it.

- **Reuse `ActivityRow` from E4-c verbatim.** Don't
  fork the component.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-audit-feed-mandatory` —
  server endpoint always present in prod.
- `mission-enterprise-cross-tenant-audit` — explicit
  tenant-switch + audit-grant.
- `mission-enterprise-audit-full-text-search` —
  full-text indexing per tenant.
- `mission-enterprise-audit-export-rbac` — operator-
  gated exports for sensitive scopes.

Sibling backend brief:

- `mission-gateway-audit-feed-endpoint` — `GET
  /api/v1/audit` with cursor pagination.
