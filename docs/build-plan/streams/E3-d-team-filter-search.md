# Stream E3-d — Team page: filter dropdown + search with URL state

**Owner**: claude.
**Depends on**: E3-a MERGED (page shell exists); E3-b MERGED (TeammateCards render the filtered set).
**Authored against**: origin/main `16e9746f` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E3-d-team-filter-search`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E3 brief 4 of 5.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Filter scope | Client-side over the loaded `Worker[]` | Server-side filter pushed to gateway when workspace has > 50 workers | `mission-enterprise-team-pagination` (already named in E3-a) |
| Saved filters | None | Per-user "saved view" persistence — operator names a filter combination + re-applies | `mission-enterprise-team-saved-views` |
| Search semantics | Case-insensitive substring on `worker.name` + `worker.role.tagline` | Fuzzy match + token-weighted ranking; relevance-sorted results | `mission-enterprise-team-fuzzy-search` |
| Audit | None | Each filter / search event audit-logged at gateway (operator-view activity audit) | `mission-enterprise-team-audit` (already named) |

v1 is fine: filter + search are read-only UX surfaces with
no security implications beyond what's already enforced
in the data layer.

## Why

E3-a stubbed the status dropdown and search input but
neither does anything. This brief makes them real:
status filter shrinks the visible cards, search filters
in real time, both persist to URL for shareability,
filter chips show active state, clear-all returns the
full list.

## Scope

- Wire the status-filter dropdown (existing element from
  E3-a):
  - Single-select with 6 options.
  - URL state via `?status=working` (etc).
  - Selecting a status filters the visible cards.
- Wire the search input (existing element from E3-a):
  - Substring match (case-insensitive) on `worker.name`
    + `worker.role.tagline`.
  - 150ms debounce.
  - URL state via `?q=...`.
- Active filter chip rail (similar to E1-b's
  approval-inbox chips):
  - Renders below the search input.
  - One chip per active dimension (status, search).
  - Click × on a chip removes that dimension.
  - Hidden when no filters active.
- "Clear all" link in the chip rail.
- Empty states per filter scenario (already specified
  in E3-a's brief; this brief verifies the wiring).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/team/page.tsx`
  (locks for this PR)

## Files you may touch

- `web/app/(app)/team/page.tsx` — narrowly scoped:
  add filter state via `useSearchParams()` + filter
  logic + chip rail render. ≤ 450 LOC after.
- `web/lib/filters/team.ts` — NEW. Pure functions:
  parse filters from URL, serialize to URL, apply
  filter to a `Worker[]`. ≤ 150 LOC.
- `web/lib/filters/team.test.ts` — NEW. ≥ 8 tests:
  empty filter, single-dimension, multi-dimension
  AND, URL round-trip, edge cases. ≤ 200 LOC.
- `web/components/skycot/TeamFilterChipRail.tsx` —
  NEW. Inline chip row showing active filters + ×
  removers + Clear all. ≤ 150 LOC.
- `web/app/(app)/team/page.test.tsx` — narrowly
  scoped: extend the existing test from E3-a with
  ≥ 4 filter-flow tests. ≤ 300 LOC after.
- `web/tests/e2e/team-filter-search.spec.ts` — NEW.
  Playwright e2e: select status, see filter; type
  query, see filter; click chip ×, see reset. ≤
  150 LOC.

## Files you must NOT touch

- `web/components/skycot/TeammateCard.tsx`,
  `TeammateCardContextMenu.tsx` — locked from E3-b.
- `web/components/skycot/TeammateGrid.tsx` — locked
  from E3-a.
- `web/lib/api/team.ts`, `lib/hooks/use-team.ts`,
  `lib/sample-team.ts`, `lib/types/worker.ts` —
  locked from E3-a.
- All E1/E2 / non-team files — out of scope.
- `crates/**` — server-side filter is a Track-H
  sibling brief.

## Acceptance criteria

- [ ] Status filter dropdown:
      - Options: `All` (default), `Working`, `Waiting
        on you`, `Idle`, `Blocked`, `Paused`.
      - Single-select.
      - URL state via `?status=` query param. `All`
        is represented by absence of param.
      - Filter applies immediately on selection.
- [ ] Search input:
      - Placeholder `"Filter by name..."` (from E3-a).
      - 150ms debounce on URL update.
      - Case-insensitive substring match on
        `worker.name` + `worker.role.tagline`.
      - URL state via `?q=` query param.
      - Cleared via × icon at the right edge of the
        input when non-empty.
- [ ] Filter chip rail below search:
      - Renders only when at least 1 filter is
        active.
      - One chip per active dimension:
        - `"Status: Working ×"`
        - `"Search: 'mariah' ×"`
      - Click chip × removes that dimension.
      - "Clear all" link at the right end of the
        rail removes everything (status reset to All,
        search cleared).
- [ ] Filter logic combines: status AND search.
- [ ] Filter dot badge on the status-filter trigger
      button when a non-`All` status is selected.
- [ ] Empty states (already specified in E3-a):
      - Filter no match: `"No teammates in this state
        right now."` (per status) + Clear-filter CTA.
      - Search no match: `"No matching teammates"` +
        `"Nothing matches \"${query}\". Try a different
        search or clear the filter."` + Clear CTA.
- [ ] Group-header counts reflect the filtered set
      (e.g. `"3 teammates working"` becomes `"1
      teammate working"` when filtered).
- [ ] Back-button navigation restores prior filter
      state from URL.
- [ ] 8 unit tests (filter library) + 4 page tests
      + 1 playwright e2e all pass.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 3KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E3-d-team-filter-search`
- PR title: `web(team): filter + search with URL-persistent state (E3-d)`

## Notes for the implementer

- **Status mapping.** The URL param is the
  lowercase WorkerStatus enum value:
  - `?status=working` → matches `worker.status ===
    "working"`.
  - `?status=waiting` → `"waiting"`.
  - Etc. Absent param means "all".
- **Search debounce.** Use the same approach as
  E1-b's filter — `useDeferredValue` from React 19
  for the URL update; immediate state update for
  the input value (so typing feels responsive).
- **Chip rail copy.**
  - Status chip: `"Status: ${label} ×"` where label
    is the human-readable form (e.g. "Working",
    "Waiting on you").
  - Search chip: `"Search: \"${query}\" ×"`.
  - "Clear all" link at the right end, gives a
    `text-xs` ghost button feel.
- **Empty state CTAs.** Use the same "Clear all"
  affordance from the chip rail in the empty
  state's body.
- **URL serialization round-trip.** Test that:
  ```
  buildUrl({ status: "working", q: "mariah" })
  === "?status=working&q=mariah"
  parseUrl("?status=working&q=mariah")
  === { status: "working", q: "mariah" }
  ```
- **Reuse the filter-chip-rail pattern from E1-b.**
  Don't author a second chip-rail abstraction —
  factor a shared `<ActiveFilterChip>` primitive if
  E1-b's chip code can be extracted. If it's tightly
  coupled to E1-b's filter shape, leave it and write
  this rail standalone; flag the duplication for a
  v1.5 dedupe brief.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-team-saved-views` — per-user
  named-filter persistence.
- `mission-enterprise-team-fuzzy-search` — fuzzy +
  ranked search.

(Plus the already-named team pagination + audit
briefs from E3-a.)
