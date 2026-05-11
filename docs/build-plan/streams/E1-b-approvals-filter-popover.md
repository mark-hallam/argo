# Stream E1-b — Approvals inbox: filter popover (channel / status / worker)

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E1-a-approvals-gateway-wireup MERGED (uses the `dataSource` hook).
**Authored against**: origin/main `fec1fc7d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E1-b-approvals-filter-popover`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E1.

## Why

The approvals-inbox spec
([wireframe-approvals-inbox.md §"(4) Action buttons"](../../skycot/ux-research/wireframes/wireframe-approvals-inbox.md))
shows a `Filter` button next to the bell + Hire-teammate
buttons. Today it's a non-functional ghost icon button. This
brief lights it up: clicking opens a popover with worker /
channel / status filters that compose, persist to URL state,
and shrink the visible card list accordingly.

This unblocks the high-volume Approver use case — when the
inbox has 50+ pending items, the Delegator wants to see only
the ones from one teammate, or only the high-priority channel
they actually watch, without losing the rest.

## Scope

- A `FilterPopover` component, opened by the existing
  `Filter` button in the page header.
- Three filter dimensions:
  - **Worker**: multi-select of worker names extracted from
    the loaded approval set.
  - **Channel**: multi-select over the 7 channel kinds
    (slack, imessage, sms, push, web, voice, whatsapp).
  - **Status**: multi-select over `pending` / `approved` /
    `refused` (post-decision cards stay visible for the
    undo window).
- URL state persistence via query params:
  `?worker=mariah,priya&channel=slack,push&status=pending`.
- A small inline "filter chip rail" between the search input
  and the timeline showing active filters with one-click
  remove (×).
- "Clear all" link in the popover + the chip rail.
- Filter button shows a small dot badge when any filter is
  active.
- Filter logic combines with the search query
  (`?q=`): both must match.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/page.tsx` (the approvals
  page — currently locked by E1-a; this brief takes the next
  lock).

## Files you may touch

- `web/components/skycot/FilterPopover.tsx` — NEW. The
  popover content + state shape. Uses Radix Popover from
  shadcn. ≤ 300 LOC.
- `web/components/skycot/FilterChipRail.tsx` — NEW. Inline
  chip row showing active filters with × removers. ≤ 120
  LOC.
- `web/lib/filters/approvals.ts` — NEW. Pure functions:
  parse filters from URL search params, serialize to URL,
  apply filter to an `ApprovalItem[]`. ≤ 150 LOC.
- `web/lib/filters/approvals.test.ts` — NEW. ≥ 8 unit tests
  covering: empty filter (passes everything), single-
  dimension filter, multi-dimension filter (AND), URL
  round-trip, search + filter composition. ≤ 200 LOC.
- `web/app/(app)/page.tsx` — narrowly scoped: hook up
  filter state (read from `useSearchParams()`), wire the
  Filter button to open `FilterPopover`, render
  `FilterChipRail` between header and timeline, apply
  filters to `items` before grouping. ≤ 350 LOC after
  changes (was ~250 in E1-a baseline).
- `web/tests/e2e/approvals-filter.spec.ts` — NEW. Playwright
  spec: open filter popover, select worker "Mariah", assert
  only Mariah's cards show, click chip × to remove,
  assert all cards return. ≤ 100 LOC.

## Files you must NOT touch

- `web/components/skycot/ApprovalCard.tsx` — locked.
- `web/lib/api/approvals.ts` — locked by E1-a's filter-
  unaware fetching. (URL filters are client-side for v1;
  server-side filtering is a future optimization.)
- `web/lib/sample-approvals.ts` — locked.
- Any file in `crates/**` — backend untouched.

## Acceptance criteria

- [ ] Clicking the existing `Filter` icon button opens a
      popover anchored below it.
- [ ] Popover contains three labeled multi-select groups:
      Worker, Channel, Status. Each shows the available
      options as checkboxes.
- [ ] Selecting any option immediately filters the card
      list (no Apply button needed).
- [ ] Active filters surface as chips in `FilterChipRail`
      between the search input and the timeline. Format:
      `Worker: Mariah ×`, `Channel: Slack, Push ×`.
- [ ] Clicking a chip × removes that filter dimension.
- [ ] Clicking "Clear all" in the popover or rail
      removes all filters.
- [ ] Filter state persists to URL: `?worker=...&channel=
      ...&status=...`. Shareable + back-button restorable.
- [ ] Filter button shows a dot indicator when any filter
      active.
- [ ] Filtered counts in the day-group headers update
      (e.g. "TODAY · 2 requests" when filtering to 2).
- [ ] Empty state when filters return zero results:
      Title `"No matching approvals"` · Body `"Try
      different filters or [ Clear all ]."`.
- [ ] Filter + search compose correctly: both must match
      for a card to show.
- [ ] 8 unit tests pass.
- [ ] Playwright e2e passes.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 8KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E1-b-approvals-filter-popover`
- PR title: `web(approvals): filter popover with URL-persistent state (E1-b)`

## Notes for the implementer

- **Popover primitive.** Use Radix Popover via shadcn's
  `<Popover>` / `<PopoverContent>`. If it's not installed
  yet, copy it from shadcn's registry. The Popover sits
  in `components/ui/Popover.tsx`. ≤ 100 LOC for the wrapper.

- **State shape.** Keep filter state in URL only —
  no local component state. Use Next's
  `useRouter().replace()` (not `push`) when filters change
  so the back stack stays clean. Reading from
  `useSearchParams()`.

- **Worker option list.** Derive from the loaded approval
  set — `Array.from(new Set(items.map(i => i.worker.name)))`.
  Order alphabetically. If a worker has no pending items
  after another filter is applied, the option still shows
  (so the user can re-include them).

- **Channel option list.** Hardcoded — the 7 channel kinds
  ship in `ApprovalCard.tsx`'s `CHANNEL_LABEL` map. Reuse
  the labels.

- **Status option list.** Three options: `Pending`,
  `Approved`, `Refused`. `Approved` and `Refused` match
  post-decision state on the card. Default selection:
  Pending only? Or all three? Default: all three (no
  filter applied initially).

- **Chip rail copy.** Comma-join values within a dimension:
  `Channel: Slack, Push ×`. If > 3 values, abbreviate:
  `Channel: 4 selected ×`. Click anywhere on the chip
  (not just ×) opens the popover scrolled to that
  dimension (nice-to-have; cut if it adds complexity).

- **Filter dot badge on the trigger button.** Tiny
  ([`var(--primary)`]) dot at the top-right of the Filter
  icon when any filter is applied. Achievable via a
  `<span>` with absolute positioning inside the Button.

- **Performance.** With < 1000 cards filtering is cheap
  client-side. Don't over-engineer; just `items.filter()`
  inside `useMemo`. A server-side query param is a future
  optimization noted in the wireframe's open questions.

- **Test the URL round-trip.** Filters → URL string →
  parsed filters → same shape. Property-based test if
  the worker is comfortable; otherwise hand-rolled cases
  covering the comma-separated lists.

- **What this brief explicitly does NOT do.**
  - Doesn't add a "saved filters" feature. That's a v1.5
    nice-to-have.
  - Doesn't add a date-range filter. Out of v1 scope.
  - Doesn't filter on the audit-replay-style fields
    (model, skill version, approver). Those live on the
    audit timeline (E5-b), not the approvals inbox.
  - Doesn't query the gateway with the filters. Client-
    side only.
