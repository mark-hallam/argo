# Stream E4-c — Worker profile: Activity tab (worker-scoped audit timeline)

**Owner**: claude.
**Depends on**: E0 MERGED; E4-a MERGED.
**Authored against**: origin/main `5e145fd3` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E4-c-worker-profile-activity-tab`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E4 brief 3 of 9.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Audit row source | `GET /api/v1/workers/<id>/activity` with mock fallback | Mandatory server endpoint; tenant-scoped; rbac-filtered fields; redaction-aware | `mission-enterprise-worker-activity-mandatory` |
| Field redaction | PII redaction via the E1-e layer (defense-in-depth) | Server-side first fence (mandatory); client redaction is belt-and-braces | `mission-enterprise-audit-pii-server-fence` |
| Pagination | Server-paginated 50 per page; client renders pages | Same | n/a |
| Filter dimensions | Time-range + action_type + status (3 dims) | Audit-timeline-full filter parity: + channel + approver + cost band + data classification | `mission-enterprise-activity-filter-parity` |
| Audit-row click | Routes to `/audit/action/<id>` (E5-c surface) | Same | n/a |

v1 is acceptable: Activity tab is a sub-scoped view of
the full audit timeline (E5). The 3 filter dimensions
cover 80% of operator drilldowns; the remaining 20%
goes through the full audit timeline.

## Why

E4-a's placeholder for `/workers/<id>/activity` reads
"Activity — coming in E4-c". This brief makes it real:
a chronological feed of the worker's actions over the
last 7 days (paginatable to last 30, then "Open in
Audit timeline" for older), with light filters.

The Activity tab is the **operator's primary
drilldown surface** when investigating one worker —
"what has Mariah done today?" / "why did Priya refuse
that candidate's screening?" The full audit timeline
(E5) is the cross-worker view; this is the per-worker
view.

## Scope

- Route `/workers/<id>/activity` renders the tab.
- Worker-filtered activity feed:
  - Default: last 7 days, all action types, all
    statuses.
  - 50 rows per page; server-paginated.
- Light filter bar above the feed:
  - Time range dropdown: Last hour / Today /
    Yesterday / Last 7 days (default) / Last 30 days
  - Action type dropdown: All (default), Message
    sent, Draft queued, Approval asked, Refusal
    issued, Escalation, Skill learned, Memory
    written
  - Status dropdown: All / Succeeded / Failed /
    Refused / Pending
- ActivityRow per audit record: timestamp + verb +
  status pill + cost (if surfaced).
- Click row → `/audit/action/<id>` (E5-c).
- Empty state when zero activity in scope.
- "Open in Audit timeline →" link at bottom routes
  to `/audit?worker=<id>` (E5).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `web/app/(app)/workers/[workerId]/activity/page.tsx`
    (NEW).

## Files you may touch

- `web/app/(app)/workers/[workerId]/activity/page.tsx`
  — NEW. Renders the worker-scoped feed. ≤ 300 LOC.
- `web/components/skycot/ActivityRow.tsx` — NEW.
  Shared ActivityRow primitive (will also be used in
  E5-a — this brief ships it first; E5-a re-uses).
  ≤ 200 LOC.
- `web/components/skycot/ActivityFilterBar.tsx` —
  NEW. Light filter bar with 3 dropdowns. ≤ 180 LOC.
- `web/lib/api/worker-activity.ts` — NEW.
  `fetchWorkerActivity(workerId, filters, cursor)`
  with mock fallback. ≤ 250 LOC.
- `web/lib/hooks/use-worker-activity.ts` — NEW.
  TanStack Query infinite-query hook for
  pagination. ≤ 200 LOC.
- `web/lib/sample-worker-activity.ts` — NEW.
  Enterprise-flavored sample activity per the 6
  sample workers from E4-a. ≤ 300 LOC.
- `web/components/skycot/worker/Activity.test.tsx`
  — NEW. ≥ 8 tests: feed renders, filter changes
  refetch, row click routes, see-all-in-audit
  routes, empty state, loading state, pagination
  load-more, error state. ≤ 350 LOC.

## Files you must NOT touch

- All E1/E2/E3 / earlier E4 files (E4-a, E4-b) —
  read-only.
- `crates/**` — sibling backend brief for the
  endpoint.

## Acceptance criteria

- [ ] `/workers/<id>/activity` renders the activity
      tab.
- [ ] Default view: last 7 days of audit rows for
      this worker, 50 rows.
- [ ] ActivityRow renders:
      - Timestamp (relative "5m ago" inline, full
        ISO on hover)
      - Worker avatar (xs)
      - Action verb (plain English summary from
        gateway)
      - Status pill (succeeded / refused / pending /
        failed)
      - Optional cost chip (when surfaced)
      - Action ID (monospace, copy-on-click)
      - Arrow → indicating clickable
- [ ] Click ActivityRow → routes to
      `/audit/action/<actionId>`.
- [ ] FilterBar with 3 dropdowns:
      - Time range (5 options).
      - Action type (8 options including "All").
      - Status (5 options).
      - All filter changes update URL `?range=...&
        type=...&status=...` and trigger refetch.
- [ ] Empty state per filter scenario:
      - All filters at default + no rows: `"No
        activity yet."` + body `"${workerName}
        hasn't taken any actions yet. Once she's
        working, you'll see her activity here."`
      - Filters applied + no match: `"No matching
        activity"` + `"Try widening the time range
        or removing filters."` + `[ Clear filters ]`
- [ ] Load-more pagination at bottom of the feed:
      - Button `[ Load more — page ${current+1} of
        ${total} ]`
      - Disabled state during fetch.
- [ ] "Open in Audit timeline →" link at the bottom
      routes to `/audit?worker=<id>` preserving
      time-range filter.
- [ ] PII redaction layer from E1-e applies to the
      ActivityRow body (verb text). Sentinel form:
      `[EMAIL]`, `[PHONE]`, etc.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e: visit activity tab, filter to
      "Last hour", assert reduced rows; click first
      row, assert /audit/action/<id> reached.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E4-c-worker-profile-activity-tab`
- PR title: `web(workers): Activity tab — worker-scoped audit feed (E4-c)`

## Notes for the implementer

- **ActivityRow lives in `components/skycot/` as a
  shared primitive.** Both this brief and E5-a use
  it. This brief ships the primitive; E5-a consumes
  it without modification.

- **Sample activity per worker.** Enterprise-shaped:
  - Priya (Recruiter): "screened candidate #4821
    against role's must-haves" / "booked interview
    with Diego Alvarez for Thu 10:30" / "drafted
    rejection email to Sara Lin"
  - Sam (AP Specialist): "extracted line items from
    invoice INV-2024-08741" / "flagged duplicate
    invoice INV-2024-08739" / "queued invoice for
    Controller approval ($14,250)"
  - Mariah (Vendor Manager): "drafted RFP response
    summary for vendor Tomás Industries" /
    "scheduled renewal review with Acme Tech"
  - Jamal (Support): "resolved tier-1 ticket
    #SUP-29382 with KB article SK-413" / "escalated
    refund request to support lead"
  - Nori (RevOps): "validated quote QT-2024-7283
    against pricing rules" / "flagged discount
    request for Deal Desk"

- **Cost chip rendering.** Only show when
  `audit.cost_usd > 0`. Format: `$0.012`. Tooltip
  shows breakdown (tokens in / out / model used).

- **Filter URL params.** Use short names:
  - `?range=7d` / `?range=24h` / `?range=1h` /
    `?range=today` / `?range=30d`
  - `?type=message_sent` / `?type=all`
  - `?status=succeeded` / `?status=all`
  Round-trip via existing filter-lib pattern.

- **Pagination cursor.** Use the server-issued
  cursor token, NOT page numbers. TanStack
  `useInfiniteQuery` handles this — `next_cursor`
  from the response feeds into `getNextPageParam`.

- **Redaction.** Use the `<RedactedText>` wrapper
  from E1-e on the verb text. Pre-redacted server-
  side data passes through unchanged; client-side
  acts as second fence.

- **Don't reimplement the audit drill-in here.**
  That's E5-c. Just link to it.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-worker-activity-mandatory` —
  server endpoint always present in prod.
- `mission-enterprise-audit-pii-server-fence` —
  server-side first fence on PII.
- `mission-enterprise-activity-filter-parity` —
  full filter parity with the audit timeline.

Sibling backend brief:

- `mission-gateway-worker-activity-endpoint` —
  `/api/v1/workers/<id>/activity` GET with cursor
  pagination.
