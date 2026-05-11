# Stream E1-d — Approvals inbox: batch decide (group + approve N similar)

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E1-a-approvals-gateway-wireup MERGED (uses the same mutation path).
**Authored against**: origin/main `52dc9067` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E1-d-approvals-batch-decide`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E1.

## Why

The approver journey
([journey-approve-on-mobile.md §"approve all 8" batching](../../skycot/ux-research/journey-approve-on-mobile.md))
calls out a critical UX pattern: when the worker proposes 8
similar actions in a row, the approver should be able to approve
them in one tap with quick swipe-through to spot-check. Without
this, a high-volume inbox produces decision fatigue and the
approver rubber-stamps everything.

The wireframe
([wireframe-approvals-inbox.md §"E1-d: batch decide"](../../skycot/ux-research/wireframes/wireframe-approvals-inbox.md))
sketches the surface but not the detection logic. This brief
ships both.

## Scope

- **Similarity detection.** Group approval cards that share
  worker + action_type + a normalized-detail hash. Show
  groups of ≥ 3 similar items with a "batch" affordance.
- **Batch action bar.** When a similarity group exists, the
  first card in the group renders a small banner above it:
  `"Mariah proposed 5 similar listings · [ Approve all 5 ]
  [ Review each ]"`.
- **Approve-all flow.** Tapping "Approve all 5":
  - Optimistic update on all 5 cards simultaneously
    (visible decision-flip + 30s undo countdown).
  - 5 parallel `submitDecision` mutations.
  - Single combined Toast confirmation: "Approved 5 of
    Mariah's listings · [ Undo ]".
  - If any individual mutation fails, that card rolls back
    and a per-card error chip appears.
- **Review-each flow.** Tapping "Review each" expands the
  group inline as a small carousel — left/right arrows or
  swipe — showing one card at a time. Decisions per card.
  Closes when last card decided.
- **Refuse-all** is intentionally NOT offered. Refusal
  requires per-case judgment by design (per the journey
  doc's "over-approval risk" section). Group affordance is
  approve-only.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/app/(app)/page.tsx`,
  `web/lib/hooks/use-approvals.ts` (mutation path)

## Files you may touch

- `web/lib/similarity/group-approvals.ts` — NEW. Pure
  function: takes `ApprovalItem[]`, returns
  `{ groups: ApprovalGroup[], ungrouped: ApprovalItem[] }`.
  A group is `{ items: [...], reason: "5 similar
  listings" }`. ≤ 200 LOC.
- `web/lib/similarity/group-approvals.test.ts` — NEW.
  ≥ 8 unit tests: empty input, single item, two similar
  items (< 3 threshold, no group), three similar (group),
  mixed similar + dissimilar, edge cases on
  normalized-detail hashing. ≤ 200 LOC.
- `web/components/skycot/BatchActionBar.tsx` — NEW. The
  banner that appears above grouped cards. Inputs:
  group + handlers. ≤ 200 LOC.
- `web/components/skycot/BatchCarousel.tsx` — NEW. Inline
  review-each carousel: shows one card at a time, prev/
  next arrows, escape to close. ≤ 250 LOC.
- `web/lib/hooks/use-approvals.ts` — narrowly scoped: add
  `useBatchSubmitDecisions()`. Same shape as
  `useSubmitDecision` but takes an array. Fires N parallel
  mutations and aggregates results. ≤ 200 LOC.
- `web/app/(app)/page.tsx` — narrowly scoped: group items
  via `groupApprovals()`, render `BatchActionBar` above
  grouped sections, render `BatchCarousel` inline when
  user picks "Review each". ≤ 400 LOC after changes.
- `web/components/skycot/BatchActionBar.test.tsx` — NEW.
  ≥ 4 tests: renders banner, approve-all fires N
  mutations, review-each opens carousel, error chip on
  per-card failure. ≤ 200 LOC.

## Files you must NOT touch

- `web/components/skycot/ApprovalCard.tsx` — locked. Cards
  stay independent; batch logic lives above them.
- `web/lib/api/approvals.ts` — locked. The mutation path
  is unchanged; only the hook adds a parallel-fire
  helper.
- `web/lib/sample-approvals.ts` — locked. The grouping
  test should add additional fixture data inside
  `group-approvals.test.ts` rather than mutating shared
  samples.

## Acceptance criteria

- [ ] When 3+ approvals share worker + action_type +
      normalized-detail-hash, a `BatchActionBar` appears
      above the first one.
- [ ] Banner copy: `"{worker.name} proposed {n} similar
      {action-noun}"`. Action-noun mapping: marketplace-
      reply → "replies"; calendar-book → "interviews";
      refund-issue → "refunds"; etc. Fallback: "actions".
- [ ] Banner shows two buttons: `[ Approve all {n} ]`
      (primary) and `[ Review each ]` (secondary).
- [ ] Approve-all triggers N parallel `submitDecision`
      mutations; each card flips to "Approved" state
      independently.
- [ ] If 1+ mutations fail, those cards roll back and
      a small error chip appears on them.
- [ ] A combined success Toast appears:
      `"Approved {n} of {worker}'s {action-noun}"` with
      an `[ Undo ]` action that calls
      `useUndoDecision` on each.
- [ ] Review-each opens an inline carousel above the
      grouped cards (collapses the grouped cards while
      open). Prev/next arrows. Card decisions update the
      underlying list. Carousel closes when last card
      decided OR user clicks Escape/Close.
- [ ] No refuse-all option exists anywhere. Per-card
      refuse remains available within the carousel and
      on each card directly.
- [ ] Similarity threshold ≥ 3 is configurable via a
      module-level constant; doc-comment explains why
      and how to change.
- [ ] 12 unit tests pass total (8 grouping + 4 batch
      action).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 7KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E1-d-approvals-batch-decide`
- PR title: `web(approvals): batch-decide for similar approval groups (E1-d)`

## Notes for the implementer

- **Normalized-detail hash.** Use a simple lowercase + word-
  tokenize + sort + join approach:
  ```ts
  function normalizeDetail(s: string): string {
    return s.toLowerCase()
      .replace(/[^\w\s]/g, " ")
      .split(/\s+/)
      .filter(t => t.length > 3)
      .sort()
      .join(" ");
  }
  ```
  Two details normalized to the same string → similar.
  This is intentionally fuzzy; it groups things like:
  - "Bauer Vapor X5, $90" and "Bauer Vapor X5, $85"
  - "Reply to Aroha: yes still available"
    and "Reply to Diego: yes still available"
  More sophisticated similarity (LLM-based) is a future
  optimization; this rule-based approach is enough for
  v1 and is testable.

- **Action-noun mapping.** Live in a small module:
  ```ts
  const ACTION_NOUN: Record<string, [string, string]> = {
    "marketplace-reply":   ["reply",    "replies"],
    "calendar-book":       ["interview","interviews"],
    "refund-issue":        ["refund",   "refunds"],
    // ... etc per shipped verticals
  };
  ```
  Returns singular + plural. Default to ["action",
  "actions"]. Easy to extend.

- **Group sort.** When grouping is applied, keep group
  members contiguous in the rendered list (sort by
  similar-group → recency). Don't intersperse ungrouped
  cards between group members — that breaks the visual
  affordance.

- **BatchActionBar placement.** Renders ABOVE the first
  card in the group, not as a wrapper around the whole
  group. This way, each card still has its own approve/
  refuse buttons; the batch bar is *additive*. The user
  can still decide one-by-one if they prefer.

- **Approve-all loading state.** The batch bar enters a
  loading state showing `"Approving 5..."` (count
  decremented as mutations complete) while the parallel
  fires happen. If 4 succeed and 1 fails, the bar shows
  `"Approved 4 — 1 failed"` with a `[ Retry ]` on the
  failed one.

- **Undo for batch.** The combined Toast's Undo
  fires N parallel `useUndoDecision` mutations. Same
  rules as individual undo (30s window per card).

- **Carousel UX.** When opened:
  - The grouped cards collapse into a single
    `BatchCarousel` block.
  - Inside the carousel, one card at a time renders at
    full-width with prev/next arrows.
  - Decided cards (within the carousel) collapse to a
    small chip strip at the top: `[ ✓ #1 ] [ ✗ #2 ]
    [ #3 ] [ #4 ] [ #5 ]` showing progress.
  - Pressing Escape or clicking the carousel × closes
    the carousel; un-decided cards return to the
    grouped list (with the batch bar restored).

- **Mobile.** Carousel becomes full-screen on `< sm`.
  Swipe gestures (using `react-swipeable` if not too
  heavy, otherwise touch-handlers inline) advance the
  carousel. Approve/Refuse buttons stay at their bottom
  position so thumb-reachable.

- **What this brief does NOT do.**
  - Doesn't add LLM-based similarity scoring. The fuzzy
    rule-based approach is good enough for v1; LLM
    similarity is a v1.5 enhancement.
  - Doesn't add "approve all by worker" or "approve all
    by channel" — those are filter-driven, not
    similarity-driven.
  - Doesn't change the gateway API. Batch is just N
    parallel single-decision POSTs; if/when a real
    batch endpoint lands, that's a follow-up brief.
  - Doesn't add user-defined batch rules. Future.
