# Wave-7 Audit #5 — Search + Filter Combinations

**Date:** 2026-05-21
**Auditor:** Wave-7 agent #5
**Scope:** Search + filter UX across every catalog surface in
SKYCOT's gateway frontend (`~/code/argo/web/app/(app)/`).

## Method

For each page, I read the page source + the filter library +
the chip-rail component + the popover component (where present),
then walked the 6 dimensions:

1. Search input has `aria-label`, name in DOM, and actually filters
2. Multiple filters stack (AND across dimensions, OR within a
   dimension)
3. Empty-search empty-state friendly + offers reset CTA
4. URL persistence — back/forward navigation and reload preserve
   filter state
5. "Clear all" affordance is visible and actually wipes state
6. Active filter count visible (chip with count, dot, or label)

Method note: this is a read-only audit. I did not boot the gateway
because Wave-7 audit briefs are evidence-by-reading, not by clicking.
The behavior I document is what the code WILL do; runtime-only bugs
(network race, hydration mismatch) are out of scope and called out
where suspected.

## Page-by-page

### 1. Approvals — `/` (the root index page)

File: `web/app/(app)/page.tsx` + `web/lib/filters/approvals.ts` +
`web/components/skycot/FilterPopover.tsx` +
`web/components/skycot/FilterChipRail.tsx`.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | PASS | `aria-label="Search approvals by worker, action, or detail"` line 454 |
| Multi-filter stacking | PASS | 3 dimensions worker/channel/status via `applyFilters`; OR within, AND across (filters/approvals.ts:87) |
| Empty-search empty-state | PASS | `<EmptyState>` switches on `hasFilters` + `query`; offers "Clear all" button |
| URL persistence | **MIXED** | Filters ARE serialized to `?worker=…&channel=…&status=…` via `serializeFilters`. **BUG**: the search input (`query`) is local React state (line 81), NOT in URL — refresh wipes the search query but keeps filters. Inconsistent. |
| Clear all | PASS | Popover footer + chip rail "Clear all"; both wire to `EMPTY_FILTERS` reset |
| Active filter count | PARTIAL | Trigger has a coloured `dot` but **no number** — operator can't tell at a glance if 1 or 3 filters apply. `activeFilterCount(filters)` is computed at line 385 but never rendered as a number — only `activeCount > 0` toggles the dot |

**Score: 7/10.** Solid filter logic with the canonical
worker/channel/status triple, but the search-query/URL-filter
asymmetry is a real UX bug and the missing count number is a
visible miss against the 6-dim spec.

### 2. Team — `/team`

File: `web/app/(app)/team/page.tsx` +
`web/lib/filters/team.ts` +
`web/components/skycot/TeamFilterChipRail.tsx`.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | PASS | `aria-label="Filter teammates by name"` line 257 |
| Multi-filter stacking | PASS | 2 dimensions (status + q); AND-combined via `applyTeamFilters` |
| Empty-search empty-state | PASS | `<NoMatchesState>` branches on `filters.q.length > 0` vs. status-only; both offer "Clear all" |
| URL persistence | PASS | Both `q` and `status` go to URL via `updateParam`; `useEffect` syncs local input mirror with URL on back/forward (line 77-79). 150 ms debounce on search-write. **Best implementation of the 7 pages.** |
| Clear all | PASS | `onClearAll` deletes both `q` + `status` params (line 123-130); chip × per dimension + chip-rail "Clear all" |
| Active filter count | PARTIAL | Status filter has a `dot`; no aggregate count. `activeTeamFilterCount` is exported from team.ts but **never called** in the page |

**Score: 9/10.** The reference implementation. URL persistence is
correct, debouncing is correct, the search-input mirror prevents
caret jump, empty-state is friendly, every dimension has a chip.
Only gap: the active-count number isn't surfaced even though the
helper exists.

### 3. /workers/[id]/activity — the per-worker audit-equivalent

File: `web/app/(app)/workers/[workerId]/activity/page.tsx` +
`web/components/skycot/ActivityFilterBar.tsx`.

Note: brief specified `/workers/[id]/audit` but the route under
`[workerId]/` is `activity/` — same purpose (a forensic per-worker
feed), and the page links into the cross-worker `/audit` view.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | N/A | No search input on this page (filter-only). Each `<select>` has its own aria-label (`aria-label="Filter by time range"` etc.) |
| Multi-filter stacking | PASS | 3 dimensions (range / actionType / status), AND across, single-value within (single-select dropdowns not multi) |
| Empty-search empty-state | PASS | `<EmptyState isPristine>` distinguishes "never had data" from "filters hide it"; the filtered branch has a `<Clear filters>` button (line 217-224) |
| URL persistence | **PARTIAL BUG** | `serializeFilters` writes `?range=…&type=…&status=…` to URL — good — but `updateFilters` clobbers any other query params (line 50-52: `router.replace(qs ? "?${qs}" : "?")`). If a deep-link comes in with `?tab=audit&range=24h`, the tab param dies on first filter change |
| Clear all | PASS | `clearFilters` → `router.replace("?")`; empty-state has a "Clear filters" button |
| Active filter count | FAIL | No chip rail. No count badge. No visual indicator that filters are non-default — operator sees the dropdowns but has to read each one |

**Score: 6/10.** Single-select dropdowns are a weaker UX than the
multi-select chip-rail pattern on Audit / Approvals, the searchParam
clobbering is a real bug, and the missing chip rail means an
operator can't see at a glance what filters are active.

### 4. Workflows — `/workflows`

File: `web/app/(app)/workflows/page.tsx`.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | PASS | `aria-label="Search workflows"` line 459 |
| Multi-filter stacking | **FAIL** | No filter dimensions exist. Only search. The "TRY" chips at line 489-501 are search-prefill buttons, not filters |
| Empty-search empty-state | **FAIL** | `filtered.length === 0` renders an empty grid with no message. The operator types a query that returns nothing and sees a blank page |
| URL persistence | **FAIL** | `query` is local React state (line 118). Refresh = state lost |
| Clear all | **FAIL** | No "clear" button. The "Propose new" button at line 477-486 has `onClick={() => setQuery("")}` which is mislabeled — the only way to clear the search is to delete characters from the input |
| Active filter count | PARTIAL | Count pill shows `${filtered.length} of ${SEED_WORKFLOWS.length}` when search active — that's a count, not a filter count |

**Score: 3/10.** Search-only; was wired in wave-1 but has no
empty-state, no URL persistence, no clear button, no filter
dimensions. The hidden gotcha: "Propose new" button silently resets
search query when clicked — that's a UX trap, not a feature.

### 5. Audit — `/audit`

File: `web/app/(app)/audit/page.tsx` +
`web/lib/filters/audit.ts` +
`web/components/skycot/audit/AuditPageHeader.tsx` +
`web/components/skycot/audit/AuditFilterChipRail.tsx` +
`web/components/skycot/audit/AuditFilterPopover.tsx`.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | PASS | `aria-label="Search audit timeline"` line 114 |
| Multi-filter stacking | PASS | 7 dimensions (worker, channel, actionType, status, approver, costBand, classification) — OR within / AND across via `applyAuditMultiFilters` (filters/audit.ts:148) |
| Empty-search empty-state | PASS | `<AuditEmpty>` distinguishes pristine from filtered; filtered branch enumerates which filters are active and offers "Clear filters" CTA |
| URL persistence | PASS | All 7 dimensions + `range` + `q` round-trip via URL. 200 ms debounce on search, 250 ms debounce on filter writes. `writeMultiFilters` correctly preserves unrelated params |
| Clear all | PASS | Popover "Clear all" (disabled when no filters), chip rail × per dim + "Clear all", empty-state CTA. Three independent paths all wire to `EMPTY_AUDIT_FILTERS` |
| Active filter count | PARTIAL | Trigger button has dot only — no number. `activeAuditFilterCount` exists in filters/audit.ts:132 but is **never imported into the page**. The chip rail does surface them visibly, but a 7-dimension popover deserves a "3 filters active" badge on the trigger |

**Score: 9/10.** The most ambitious filter implementation in the
codebase — 7 multi-select dimensions, debounced URL writes, chip
rail with per-dimension clear, active dot on trigger, full empty-
state coverage. Only gap: the trigger dot should be a count badge.

### 6. Skills — `/skills`

File: `web/app/(app)/skills/page.tsx`.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | PASS | `aria-label="Search the skill marketplace"` line 334 |
| Multi-filter stacking | **FAIL** | No filter dimensions — search-only |
| Empty-search empty-state | PASS | `skills-empty` state at line 379-383: "No skills match "{query}"…" — but **no clear button**, the operator has to manually backspace |
| URL persistence | **FAIL** | `query` is local React state (line 53) |
| Clear all | **FAIL** | No clear-search button on the input (no × icon), no "Clear" link in the empty state. The TRY chips set the query but can't unset it |
| Active filter count | PARTIAL | Count pill `{skills.length} skills · {filtered.length} shown` is a count not a filter count |

**Score: 5/10.** Empty-state copy is friendly, the ⌘K hint is nice,
but no URL persistence + no clear-search-on-empty-state = poor
flow. The TRY chips are write-only — clicking one to test a query
then trying to get back to the unfiltered view requires manual
backspacing.

### 7. Cohorts — `/cohorts`

File: `web/app/(app)/cohorts/page.tsx`.

| Dimension | State | Evidence |
|---|---|---|
| Search aria-label | **FAIL** | No search input exists at all |
| Multi-filter stacking | **FAIL** | No filters exist. Brief stated "department tabs" — the design lock memory mentions them but **they aren't built yet**. The page is a flat cohort grid + detail drawer |
| Empty-search empty-state | **N/A** | Only "no cohorts at all" state exists (line 58-66) |
| URL persistence | **N/A** | Nothing to persist |
| Clear all | **N/A** | Nothing to clear |
| Active filter count | **N/A** | No filters |

**Score: 1/10** (the only thing it does is render a grid — there's
nothing to filter against). The locked D3 magazine-tiles design
(per `project_cohorts_page_design_locked` memory) calls for 6
department tabs (Project & Ops, Customer, Finance, People, GTM,
Engineering) but the live page has zero tab UI. Drift between
brand-lock memory and shipped code.

## Aggregate scores

| Page | Score | Filter persistence | Notes |
|---|---|---|---|
| Audit | 9/10 | YES — best | 7 dims, debounced, complete chip rail |
| Team | 9/10 | YES — best | Reference debouncing pattern |
| Approvals | 7/10 | PARTIAL | Filters yes, search query no |
| /workers/[id]/activity | 6/10 | PARTIAL | Clobbers unrelated params |
| Skills | 5/10 | NO | Local-state only |
| Workflows | 3/10 | NO | Local-state only, no empty state |
| Cohorts | 1/10 | N/A | No filter UI at all |

**Mean: 5.7/10.** Median: 6/10.

## Pages with broken or missing URL filter persistence

Count: **4 of 7** pages.

1. Approvals — filters persist, **search query does not** (mixed)
2. /workers/[id]/activity — persists but **clobbers unrelated params**
3. Workflows — **no persistence**
4. Skills — **no persistence**

## Top 5 search-filter bugs

1. **Approvals: search-query lives in local state.** Refresh the
   page mid-search and the operator's typed query is gone. Filters
   survive, search doesn't. Confusing because they look co-located in
   the header (`web/app/(app)/page.tsx:81` `useState("")` vs.
   `parseFilters(searchParams)`). Fix: thread `q` through URL like
   `/audit` already does.

2. **Workflows: typing a query that returns 0 results renders a
   blank page.** No empty-state component is rendered. The filter
   condition `filtered.length === 0` is silently true and the grid
   renders zero cards (`web/app/(app)/workflows/page.tsx:508-546`).
   Fix: add an `<EmptyState>` with `<button onClick={() => setQuery("")}>`.

3. **Workflows: the "Propose new" button silently resets search.**
   `onClick={() => setQuery("")}` (line 481) on a button labelled
   "Propose new" — operator clicks expecting a propose-flow, gets
   their search erased instead. Fix: separate the clear-search
   affordance from "Propose new" (which should route to a builder).

4. **/workers/[id]/activity: filter writes clobber unrelated URL
   params.** `router.replace(qs ? "?${qs}" : "?")` (line 51) wipes
   any other `?key=val` on the URL. If anyone deep-links with extra
   context (`?tab=audit&range=24h&utm_source=email`), the first filter
   change kills the deep-link metadata. Fix: merge with existing
   `searchParams` like `/audit` does (`writeSingleParam` in
   `audit/page.tsx:307-319`).

5. **Skills + Workflows + Approvals-search: no clear-search × on the
   input.** Team has one (`web/app/(app)/team/page.tsx:260-269`),
   Audit doesn't need one because the empty-state CTA covers it, but
   for the three search-only pages without empty-state CTAs the
   operator has to backspace each character. Should be a 30-line
   shared `<SearchInput clearable />` component.

## Verdict on filter UX

**Two-tier system.** The pages that lean on the modern
`web/lib/filters/*` library (Approvals, Team, Audit) are solid:
URL-driven, chip rails, debounced writes, proper empty states.
The pages that don't (Workflows, Skills, Cohorts) are bare —
`useState("")` and no persistence.

For v1 demo readiness the gap is most visible on **Workflows**
(recently wired but missing empty-state + clear button) and
**Cohorts** (no filter UI at all, contradicting the locked
department-tab design). Both are landing-page-level surfaces an
operator will hit during the demo.

For enterprise launch the gap is most visible on **Approvals**
(headline page; the search-vs-filter asymmetry is jarring) and
**/workers/[id]/activity** (param clobbering is a real bug that
breaks audit deep-links from email/Slack).

The active-filter-count number is **missing on every page** even
though three of them (Approvals, Team, Audit) already compute it
in their filter library. This is the single fix with the highest
ratio of "user-visible improvement" to "lines of code changed" —
roughly four 1-line edits to swap a dot for a number badge.

Recommend three follow-up briefs:

- `mission-search-filter-persistence-parity-workflows-skills` —
  port the Team/Audit URL persistence pattern to Workflows + Skills
  (~80 LOC each)
- `mission-search-filter-clear-search-affordance` — extract the
  Team clearable-search pattern into a shared `<SearchInput />`
  primitive (~50 LOC + replace 3 call-sites)
- `mission-search-filter-active-count-badges` — surface
  `activeFilterCount()` numbers on every filter trigger button
  (~4 lines × 3 pages)

Plus the Cohorts department-tabs surface needs to be built (or
the design memory updated to match what shipped) — that's a
larger brief than these three, probably `mission-cohorts-department-tabs`.
