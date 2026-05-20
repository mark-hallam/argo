# Wave-8 Audit #4 — Bulk Actions + Mass-Data

**Date:** 2026-05-21
**Scope:** `~/code/argo/web/` — pages: `/`, `/audit`, `/team`
(=workers list), `/workflows`, `/skills`, `/settings/members`, `/orchestration`,
`/cohorts`. Read-only.
**Question:** does SKYCOT scale to a real workspace with 50+ workers, 500+
approvals/day, 10,000+ audit rows? Specifically — bulk-select (checkbox + shift
range + toolbar), pagination strategy, and virtualization for long lists.

---

## Bottom line

SKYCOT has **zero classical bulk-action infrastructure**. No checkboxes on any
list row. No multi-row selection. No bulk-action toolbar that appears when
selection > 0. There is **one** specialized batch surface — the similarity-
grouped "Approve all N" bar on the Approvals inbox — and one server-side
bundling surface (`BundleCard`). Both are *single-decision-per-group*, not
*multi-row-select-then-act*.

Virtualization library: **none installed**. `package.json` has no
`react-virtual`, `@tanstack/react-virtual`, `react-window`, or `react-virtuoso`.
Long lists are rendered via plain `.map()` and the audit feed is paginated by
50-row "Load more" pages.

Pagination shape across the app is consistent: **cursor-based** server API +
**TanStack `useInfiniteQuery`** + **button-driven "Load more"** in the UI. No
classic page-numbers control. No infinite-scroll IntersectionObserver. Page
size constant `PAGE_SIZE = 50` in both `lib/api/audit.ts:27` and
`lib/api/worker-activity.ts:80`.

---

## Five-dimension scorecard

| Dimension | / (Approvals) | /audit | /team (Workers) | /workflows | /skills | /settings/members | /orchestration | /cohorts |
|---|---|---|---|---|---|---|---|---|
| 1. Select-all checkbox | NO | NO | NO | NO | NO | NO | NO | NO |
| 2. Shift-click multi-select | NO | NO | NO | NO | NO | NO | NO | NO |
| 3. Bulk-action toolbar (selection > 0) | NO* | NO | NO | NO | NO | NO | NO | NO |
| 4. Virtualization for ≥1000 rows | NO | NO | NO | NO | NO | NO | NO | NO |
| 5. Pagination UX | N/A (live feed) | Load more (cursor, 50/page) | None (full list) | None (seed array) | None (full list) | None (seed list) | None | None |

*The Approvals page has the `BatchActionBar` (a per-similarity-group "Approve
all N" affordance) but it is NOT a row-checkbox bulk-action. See the
"specialized batch surfaces" section below.

**Summary:** 0 / 8 pages with classical bulk-actions. 0 / 8 with virtualization.

---

## Specialized batch surfaces that DO exist

These are not bulk-actions in the multi-row-select sense, but they exist and
solve a slice of the "operator approves 10 things in one tap" problem.

### 1. `BatchActionBar` — similarity-grouped approve-all (Approvals inbox)

Path: `~/code/argo/web/components/skycot/BatchActionBar.tsx`,
`~/code/argo/web/components/skycot/BatchGroupBlock.tsx`,
`~/code/argo/web/app/(app)/page.tsx`.

How it works:
- `lib/similarity/group-approvals.ts` clusters incoming approvals by worker +
  action shape into "groups" (e.g. "Mariah proposed 8 similar replies").
- For each group, a `BatchActionBar` renders ABOVE the first card with a
  one-tap "Approve all N" button and a "Review each" button that opens the
  `BatchCarousel`.
- The bar has three phases: idle → loading (live `Approving X…` countdown) →
  settled (with Undo + per-failure Retry).
- Critically: **approve-only**. "Refuse-all is intentionally not offered (per
  journey doc: refusal requires per-case judgment, batch is approve-only)" —
  comment from `BatchActionBar.tsx:13-14`.
- Implementation: `useBatchSubmitDecisions` in `lib/hooks/use-approvals.ts:105`
  fires N parallel POSTs (one per approval), each with its own optimistic
  update, then resolves a single Promise with per-item ok/err outcomes.

`BatchUiState` is what the operator was asking about. Defined in
`components/skycot/BatchGroupBlock.tsx:36`. Carries `{phase, completed,
failed, failedIds, carouselOpen}` per group key. Lives in
`app/(app)/page.tsx:82` as `Record<string, BatchUiState>`.

### 2. `BundleCard` — server-defined bundle approval

Path: `~/code/argo/web/components/skycot/BundleCard.tsx`,
`~/code/argo/web/lib/hooks/use-approval-bundles.ts`.

When the gateway has assigned a `bundleId` (server-side bundling — distinct
from client-side similarity grouping), the inbox renders a `BundleCard` that
shows "N actions as one stack" and offers a single Approve / Refuse. The
backend then applies the decision atomically to every member.

Unlike `BatchActionBar`, this one supports both Approve AND Refuse.

### 3. `EvidenceBundleExporter` — bulk audit export

Path: `~/code/argo/web/components/skycot/audit/EvidenceBundleExporter.tsx`.

The audit page's `[ ⤓ Export ]` button opens a dialog that bulk-exports
audit rows by scope (filtered / single-action / replay / sampled). Three
size paths: ≤50MB inline download, 50-100MB confirm-then-download, >100MB
queued + email when ready. This is the closest thing to a "select 10000
audit rows, do thing X to them" the codebase has — but it's export, not a
mutating bulk-action.

---

## Page-by-page detail

### `/` — Approvals inbox (`app/(app)/page.tsx`, 564 LOC)

**Bulk infra:** None at the row level. Each `ApprovalCard` is approve-one /
refuse-one. The only multi-decision affordance is the similarity-group
`BatchActionBar` (approve-only, opt-in per group) and the server-bundle
`BundleCard` (approve/refuse, server-defined).

**Mass-data:** The Approvals inbox uses `useApprovals()` (not paginated; one
full fetch + websocket live updates). Day-grouping into "Today / Yesterday /
Earlier" sections, then similarity grouping inside each day. At 500/day on
a 30-day view (~15k rows) the inbox would render every card. No
virtualization.

**Verdict:** Approve-all-from-one-worker = decent. Multi-worker, multi-action
bulk = absent. "Freeze these 3 workers" or "archive these 12 cards" is
impossible without per-row selection.

### `/audit` — Forensic timeline (`app/(app)/audit/page.tsx`, 426 LOC)

**Bulk infra:** None. Read-only timeline. Each `ActivityRow` is a link to
the action-detail page.

**Mass-data:** Uses `useAuditFeed` → `useInfiniteQuery` over a cursor-based
`fetchAuditFeed` endpoint with `PAGE_SIZE = 50`. Pagination UX is a
button-driven "Load more — page N of M" at the bottom of the list
(`app/(app)/audit/page.tsx:237-251`). All loaded pages are flattened into
one `rows` array (`feed.data?.pages.flatMap((p) => p.rows)`) and rendered as
plain `<div>`s inside `AuditDayGroup` sections. No virtualization. The
`AuditDayGroup` is dumb-`.map()` over the bucket's rows
(`components/skycot/audit/AuditDayGroup.tsx:39-49`).

**Verdict:** Works for the first few pages. After the operator clicks "Load
more" 5-10× (250-500 rows in DOM), scroll smoothness will degrade. The
inline file comment at `page.tsx:53-54` says "Server-side push for scopes
> 5000 rows is a Track-H hardening brief (mission-enterprise-audit-server-
filter)". So the team has thought about it but has not landed a fix.

The forensic use case (10000+ rows) will likely require the auditor to
filter aggressively first (range + worker + action-type) to keep the DOM
under a few hundred rows.

### `/team` (workers list) (`app/(app)/team/page.tsx`, 463 LOC)

**Bulk infra:** None. Each worker is a `TeammateCard` in a CSS grid. Click
opens worker detail. No checkbox, no bulk freeze / archive / re-train.

**Mass-data:** `useTeam()` returns the full worker list — no pagination. At
50+ workers, all render. The cards are trading-card-deck visuals
(`TeammateGrid` + `TeammateCard`) — each card carries ~10 SVG / gradient
elements. At 100+ workers, scroll will degrade.

**Verdict:** "Approve onboarding for these 5 new workers" or "Pause these 3
underperformers" are obvious enterprise needs that the surface cannot do.

### `/workflows` (`app/(app)/workflows/page.tsx`, 603 LOC)

**Bulk infra:** None. Curated seed array of 9 workflows. CSS grid of
`workflow-card` links to `/workflows/<vertical>/<name>`.

**Mass-data:** Seed array; no live fetch yet. Even at scale, workflows per
tenant likely number in tens not thousands — virtualization unnecessary.

**Verdict:** Bulk publish / bulk-archive / version-bump-N workflows would be
nice for an enterprise workflow library, but the immediate need is small.

### `/skills` (`app/(app)/skills/page.tsx`)

**Bulk infra:** None. Marketplace grid of skill cards. Click opens skill
detail. No "install these 5 skills into this worker" bulk add.

**Mass-data:** `useSkills()` returns the full registry. Likely 10s of
skills, fine.

**Verdict:** Bulk skill install into a worker would be valuable but the
list is short enough that single-tap is OK.

### `/settings/members` (`app/(app)/settings/members/page.tsx`, 134 LOC)

**Bulk infra:** None. Each row is a `MemberRow` with a per-row Edit button
opening `EditMemberModal`. Invite is single-member-only via
`InviteMemberModal`.

**Mass-data:** Seed array; PATCH stubbed. Workspace membership rarely
exceeds 100 users.

**Verdict:** Bulk-invite (paste 20 emails, hit one invite) and bulk-role-
change (move 8 users from "user" to "admin") are clear enterprise gaps.

### `/orchestration` (`app/(app)/orchestration/page.tsx` + `components/skycot/orch/QueueTable.tsx`)

**Bulk infra:** None. The `QueueTable` has per-row Cancel / Reset retry /
Force escalate icon buttons — no bulk-cancel or bulk-retry.

**Mass-data:** A live queue of orchestration runs. No virtualization. At
500+ in-flight runs the table will render every row.

**Verdict:** Bulk-cancel a stuck batch of runs is the single most obvious
ops gap. SREs want "cancel all runs blocked >10min" as one click.

### `/cohorts` (`app/(app)/cohorts/page.tsx`, 148 LOC)

**Bulk infra:** None. Magazine-tile grid of cohort cards. Click opens
cohort detail drawer.

**Mass-data:** Curated; small list.

---

## Top 3 places where bulk-actions would be most valuable

Ordered by enterprise impact per unit of engineering effort:

### 1. `/orchestration` — bulk-cancel / bulk-retry stuck runs

**Why it's #1.** When a provider goes down or a misconfigured prompt
escapes review, hundreds of runs can hang simultaneously. The current UI
forces the operator to click Cancel one row at a time. This is the kind of
gap that turns a 3-minute incident into a 30-minute incident.

**Minimum viable shape:**
- Per-row checkbox + select-all-on-page checkbox in the header.
- Floating bulk-action bar appears when ≥1 selected: `[ Cancel N ]`,
  `[ Reset retry on N ]`, `[ Force escalate N ]`.
- Filter-based bulk-select: "Cancel all `status=stuck` AND `age>10min`"
  as a one-tap recipe in the page header.

### 2. `/` (Approvals inbox) — cross-worker, cross-action bulk

**Why it's #2.** The current `BatchActionBar` only batches within a single
worker × single action-shape group. A real workspace gets 500 approvals/day
from 50 workers — the operator wants to "approve everything Mariah, Priya,
and Jamal raised today" or "refuse everything tagged `risk=high`". The
current UX cannot do this.

**Minimum viable shape:**
- Per-card checkbox (additive — existing per-card Approve/Refuse stays).
- Shift-click range selection.
- Bulk toolbar: `[ Approve N ]`, `[ Refuse N ]`, `[ Snooze N ]`.
- Snap-into existing similarity-group / bundle UI: clicking the group
  header checks every card in the group.

### 3. `/settings/members` — bulk invite + bulk role change

**Why it's #3.** First-month customers want to invite 20 people from a CSV
or paste 20 emails. The current Invite modal is one-at-a-time. Also: when
a new admin is appointed, the operator usually wants to bump 3-5 users from
"user" to "admin" in one go.

**Minimum viable shape:**
- Multi-email invite modal (paste-comma-separated or paste-newline,
  same role + cohort scope applied to all).
- Per-row checkbox + bulk-action bar: `[ Change role for N ]`,
  `[ Remove N ]`, `[ Resend invite to N ]`.

---

## Virtualization library: ABSENT

`~/code/argo/web/package.json` does NOT include:
- `react-virtual` / `@tanstack/react-virtual` (the modern TanStack-aligned one)
- `react-window` (Brian Vaughn's classic)
- `react-virtuoso` (the high-DX modern option)

No `IntersectionObserver`-based infinite-scroll either. The only reference to
the word "virtualization" anywhere in the repo is one aspirational comment in
`docs/build-plan/streams/mission-pm-skill-canvas-diff.md:56` saying "For long
TOMLs, virtualize the diff view (react-window or similar) only if the test
fixture exceeds 500 lines."

**Recommendation:** When the team adopts a virtualization library, default to
`@tanstack/react-virtual` since TanStack Query is already in `package.json`
(consistent vendor, modern, hooks-first API). Reserve for `/audit` first
(most likely to hit 1000+ rows), `/orchestration` second (live queue can
spike), `/team` third (only matters at 100+ workers).

---

## Mass-data perf verdict

**At v1 scale** (demo tenant with ~10 workers, ~30 approvals/day, ~500
audit rows): perf is fine. DOM stays under ~200 nodes on every page.

**At enterprise scale** (50 workers, 500 approvals/day, 10000 audit rows):
- `/audit` with the operator clicking "Load more" 10× = 500 rows = noticeable
  scroll judder but tolerable.
- `/audit` at 10000 rows (operator clicks "Load more" 200×, which they
  won't, but a server-side scope export pulls 10000 rows) = unusable.
- `/team` at 100 workers = grid layout slows on resize, but scroll OK.
- `/` (Approvals inbox) at 500 cards = heavy. Each `ApprovalCard` has
  multi-line layout with conditional redaction passes; rendering 500
  cards with `redactPII` calls inline (see `app/(app)/page.tsx:142-154`)
  will block the main thread.

**Pre-launch tolerance:** The team should land virtualization for
`/audit` before the first enterprise customer with >5k audit rows.
`/orchestration` is the second-most-urgent. The Approvals inbox needs
either virtualization OR aggressive server-side pagination (currently
unpaginated).

---

## Files of interest

- `~/code/argo/web/package.json` — dependencies (no virt lib)
- `~/code/argo/web/components/skycot/BatchActionBar.tsx` — similarity-group approve-all bar
- `~/code/argo/web/components/skycot/BatchGroupBlock.tsx` — `BatchUiState` definition
- `~/code/argo/web/components/skycot/BundleCard.tsx` — server-defined bundle card
- `~/code/argo/web/components/skycot/audit/EvidenceBundleExporter.tsx` — bulk audit export
- `~/code/argo/web/app/(app)/page.tsx` — Approvals inbox (uses `BatchUiState`)
- `~/code/argo/web/app/(app)/audit/page.tsx` — Load-more pagination
- `~/code/argo/web/app/(app)/team/page.tsx` — full-list workers grid
- `~/code/argo/web/app/(app)/workflows/page.tsx` — seed grid
- `~/code/argo/web/app/(app)/skills/page.tsx` — marketplace grid
- `~/code/argo/web/app/(app)/settings/members/page.tsx` — member list
- `~/code/argo/web/components/skycot/orch/QueueTable.tsx` — orchestration table
- `~/code/argo/web/lib/hooks/use-approvals.ts` — `useBatchSubmitDecisions`
- `~/code/argo/web/lib/hooks/use-audit-feed.ts` — cursor-based InfiniteQuery
- `~/code/argo/web/lib/api/audit.ts` — `PAGE_SIZE = 50`, cursor pagination
