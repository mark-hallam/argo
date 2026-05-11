# Stream E5-b — Audit timeline: multi-dimension filter popover

**Owner**: claude.
**Depends on**: E0 MERGED; E5-a MERGED.
**Authored against**: origin/main `c03c4a2e` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-b-audit-filter-popover`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 2 of 8.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Filter dimensions | 7 dimensions (worker / channel / action type / status / approver / cost band / data classification) | Same | n/a |
| Filter persistence | URL params only | Same + named "Saved view" persistence per user (e.g. "All approvals over $10K") | `mission-enterprise-audit-saved-views` |
| Cross-tenant filter | Single-tenant scope | Operator may filter across multiple tenants when granted | `mission-enterprise-cross-tenant-audit` (already named E5-a) |
| Data classification filter | Visible to all viewers | Hidden from viewers without `unredacted-view` permission (the classification itself is sensitive) | `mission-enterprise-audit-classification-rbac` |
| Filter performance | Client-side aggregation (acceptable for ≤ 5000 rows in scope) | Server-side filter pushed into the query when scope > 5000 rows | `mission-enterprise-audit-server-filter` |

v1 is acceptable: client-side filter on the loaded
window. Enterprise scope > 5000 rows is a known follow-
up handled by server-side filter.

## Why

E5-a's `[Filter]` button is a placeholder. This brief
lights it up: a popover with 7 multi-select dimensions,
URL state, active-filter chip rail below the search
input. The chip rail is the same shape as E1-b's
approval-inbox chip rail.

## Scope

- `AuditFilterPopover` opens from the `[Filter]`
  button on the audit page header.
- 7 sections inside, each multi-select:
  - **Worker** — from the loaded audit set
  - **Channel** — 7 channel kinds
  - **Action type** — common action types (extracted
    from sample data)
  - **Status** — succeeded / refused / pending /
    failed / autonomous
  - **Approver** — humans who have approved actions
    in the audit set
  - **Cost band** — `Free` / `< $0.01` / `$0.01-$0.10`
    / `> $0.10`
  - **Data classification** — public / internal /
    confidential / restricted (gated by viewer's RBAC)
- "Clear all" + "Apply" buttons in the popover footer
  (Apply is optional — popover updates URL on each
  change with debounce).
- URL state via `?worker=...&channel=...` etc.
- Inline filter chip rail below the search input
  showing active filters with × removers.
- Filter button shows a dot indicator when any
  filter active.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/audit/page.tsx`
  (locks until E5-b merges; subsequent E5 briefs
  wait turn).

## Files you may touch

- `web/components/skycot/audit/AuditFilterPopover.tsx`
  — NEW. ≤ 400 LOC.
- `web/components/skycot/audit/AuditFilterChipRail.tsx`
  — NEW. ≤ 180 LOC.
- `web/lib/filters/audit.ts` — NEW. Pure functions:
  parse from URL, serialize to URL, apply to
  AuditRow[]. ≤ 220 LOC.
- `web/lib/filters/audit.test.ts` — NEW. ≥ 12 tests
  covering each dimension, multi-dim AND, URL
  round-trip, RBAC-gated classification dimension.
  ≤ 300 LOC.
- `web/app/(app)/audit/page.tsx` — narrowly scoped:
  wire `[Filter]` button to open popover; mount
  `AuditFilterChipRail` between search + timeline.
  ≤ 420 LOC after.

## Files you must NOT touch

- All non-audit files.
- `crates/**` — server-side filter is Track-H.

## Acceptance criteria

- [ ] `[Filter]` button on audit page opens
      `AuditFilterPopover` anchored below it.
- [ ] Popover renders 7 sections (or 6 if viewer
      lacks `unredacted-view` permission — data
      classification dimension hidden).
- [ ] Each section is multi-select with checkboxes.
- [ ] Selecting any option updates URL immediately
      (debounced 250ms to coalesce multi-click).
- [ ] Filter dot indicator on the Filter button
      when any filter active.
- [ ] AuditFilterChipRail renders below the search
      input:
      - One chip per active dimension
        (e.g. `Worker: Priya, Sam ×`, `Channel:
        Slack ×`).
      - "Clear all" link at right end.
      - Hidden when no filters active.
- [ ] Click chip × removes that dimension; "Clear
      all" removes everything.
- [ ] Cost-band filter:
      - 4 options: `Free` (cost = 0), `< $0.01`,
        `$0.01-$0.10`, `> $0.10`.
      - Multi-select.
- [ ] Data-classification filter (only visible to
      operators):
      - 4 options: public / internal /
        confidential / restricted.
      - Multi-select.
- [ ] Filters compose with search query (?q=) and
      time range from E5-a.
- [ ] Empty state when filters return zero rows:
      `"No actions match"` + dynamic body per
      filter combo + `[ Clear filters ]`.
- [ ] 12 unit tests pass.
- [ ] Playwright e2e: open popover, select 2
      filters, see chip rail update, click × on a
      chip, see filter removed.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 7KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-b-audit-filter-popover`
- PR title: `web(audit): multi-dimension filter popover (E5-b)`

## Notes for the implementer

- **Popover composition.** Use shadcn's Popover
  primitive (already used in E1-b for approvals
  filter). Single PopoverContent with 7 stacked
  sections + footer actions.

- **Section UI.** Each section header (clickable
  to collapse/expand) + checkbox list. Default:
  all collapsed except "Worker" + "Status" (the
  two most-used).

- **Worker dimension option list.** Same as E1-b
  approvals filter — derive from the loaded
  audit set. Order alphabetically.

- **Action type extraction.** From sample data,
  build a fixed enumeration:
  - message_sent
  - message_drafted
  - approval_asked
  - approval_decision
  - refund_issued
  - invoice_processed
  - candidate_screened
  - interview_booked
  - rfp_drafted
  - skill_promoted
  - memory_written
  - duty_fired

  Render as human-readable labels: "Message sent",
  "Approval decision", etc.

- **Cost band logic:**
  ```ts
  function inCostBand(row: AuditRow, band: string): boolean {
    const c = row.cost?.usd ?? 0;
    switch (band) {
      case "free":        return c === 0;
      case "lt_0_01":     return c > 0 && c < 0.01;
      case "0_01_to_10":  return c >= 0.01 && c <= 0.10;
      case "gt_0_10":     return c > 0.10;
    }
  }
  ```

- **URL state convention.**
  - `?worker=priya,sam` (comma-separated)
  - `?channel=slack,push`
  - `?action_type=message_sent,refund_issued`
  - `?status=succeeded,refused`
  - `?approver=user_abc,user_def`
  - `?cost_band=lt_0_01,gt_0_10`
  - `?classification=confidential,restricted`

- **RBAC gating for classification dimension.**
  ```tsx
  {viewer.permissions.canViewClassification && (
    <FilterSection title="Data classification" .../>
  )}
  ```
  v1 default: all true. v1.5 swaps in real RBAC.

- **Chip rail copy.** Comma-join values; truncate
  long lists:
  - 1-3 values: show all
  - 4+: show "N selected"
  - `Worker: Priya, Sam ×`
  - `Action type: 4 selected ×`

- **Don't reimplement TanStack Query usage.**
  The page's `useAuditFeed()` reads `filters` and
  triggers refetch. Just update filters; the hook
  handles the refetch.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-audit-saved-views` — per-user
  named filter combinations.
- `mission-enterprise-audit-classification-rbac` —
  per-viewer RBAC gating on the classification
  dimension.
- `mission-enterprise-audit-server-filter` —
  server-side filter push for large scopes.
