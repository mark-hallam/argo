# Wireframe — Approvals Inbox

The daily-driver surface. Already partially built in Track C at
[web/app/page.tsx](../../../../web/app/page.tsx); this doc is the
canonical spec the surface answers to. Future PRs that touch the
approvals inbox update this file in the same commit.

## Purpose

Show a Delegator the approvals their AI teammates are waiting on,
let them decide each one in ≤ 10 seconds, and confirm the
decision stuck. The single highest-frequency touchpoint in the
product.

**Serves:**
- Persona: [delegator.md](../delegator.md),
  [approver.md](../approver.md) (primary)
- Journey: [journey-approve-on-mobile.md](../journey-approve-on-mobile.md) (primary),
  [journey-audit-replay.md](../journey-audit-replay.md) (light — links from a row to its drill-in)

**Implementation primitives:**
- [Card](../../../../web/components/ui/Card.tsx),
  [Avatar](../../../../web/components/ui/Avatar.tsx),
  [Button](../../../../web/components/ui/Button.tsx),
  [StatusPill](../../../../web/components/ui/StatusPill.tsx),
  [NavDrawer](../../../../web/components/ui/NavDrawer.tsx)
- [ApprovalCard](../../../../web/components/skycot/ApprovalCard.tsx)
  (composite — load-bearing)
- [ThemePicker](../../../../web/components/skycot/ThemePicker.tsx)
  (footer)

## Information architecture

- URL: `/` (root). Also reachable at `/approvals` (alias).
- Nav placement: first item in the side drawer. Default landing
  surface after auth.
- No breadcrumb (root surface).
- Deep-linkable filters via query params: `?worker=mariah`,
  `?channel=slack`, `?status=pending`, `?q=<search>`.
- Each approval card links to its audit drill-in at
  `/audit/action/<approval-id>` via a long-press on mobile or
  via the "Why?" expansion on desktop (the "see history" link
  inside).

## Layout (default state, desktop)

```
+----------------------------------------------------------------+
| (1) LEFT RAIL                |  (2) MAIN COLUMN               |
|                              |                                 |
| +---+ SKYCOT                 |  +---------- STICKY HEADER ----+
| | S | AI TEAMMATES           |  | (3) Approvals  • 7 waiting  |
| +---+                        |  |     Your AI teammates...    |
| -----------                  |  |                       (4)(5)|
| (a)📨 Approvals       [7]    |  |  [ 🔻 ] [ 🔔 ] [ + Hire ]   |
| (b) Team                     |  | --------------------------- |
| (c) Workers                  |  | (6) [ 🔍 Search ...      ]  |
| (d) Audit                    |  +-----------------------------+
| (e) Skills                   |                                 |
|                              |  (7) TODAY · 6 requests       │ |
| -----------                  |                                 |
| ⚙ Settings                   |  +-------- ApprovalCard ----+  |
| +---+                        |  | (M) Mariah  wants to...  |  |
| | N | Sam Harlow      ⌄      |  |     Slack · 3m ago       |  |
| +---+ Acmeco · Owner         |  |                          |  |
|                              |  | Detail paragraph...      |  |
|                              |  |                          |  |
|                              |  | If YES → ...             |  |
|                              |  | If NO  → ...             |  |
|                              |  |                          |  |
|                              |  | [ ✓ Approve ] [ ✗ Refuse]|  |
|                              |  |   [ Why? ⌄ ]             |  |
|                              |  +--------------------------+  |
|                              |                                 |
|                              |  ... 5 more "Today" cards ...   |
|                              |                                 |
|                              |  (8) YESTERDAY · 1 request      |
|                              |  ... 1 card ...                 |
|                              |                                 |
|                              |  (9) [ ─ footer ───────────── ] |
|                              |       Theme picker · 2 surfaces |
+----------------------------------------------------------------+
```

## Annotations

**(1)** Side rail. Width: 256px. Background: `var(--card)`/40
with `backdrop-blur-sm`. Fixed-position; main column has
`md:pl-64`. Hidden under hamburger at < 768px.

**Items (a)–(e)**: NavItems with leading lucide icon. Active
item has accent background `var(--primary)/12` + accent icon
color. Active item is page-aware: route `/` → "Approvals" is
active.

**(a)** Approvals — `Inbox` icon. Active by default. Trailing
[StatusPill](../../../../web/components/ui/StatusPill.tsx)
showing pending count when n > 0. Strings: `"Approvals"`.

**(b)** Team — `Users` icon. Strings: `"Team"`. Route `/team`.

**(c)** Workers — `Workflow` icon. Strings: `"Workers"`. Route
`/workers`. (Distinct from Team: Workers is the
operations/admin view; Team is the "meet your AI teammates"
view. See [wireframe-team-page.md](wireframe-team-page.md) and
[wireframe-worker-profile.md](wireframe-worker-profile.md).)

**(d)** Audit — `Activity` icon. Strings: `"Audit"`. Route
`/audit`. See [wireframe-audit-timeline.md](wireframe-audit-timeline.md).

**(e)** Skills — `Sparkles` icon. Strings: `"Skills"`. Route
`/skills`.

**Settings + user pill (bottom)** — pinned to the foot via
`mt-auto`. Settings is `Settings` icon. User pill shows
avatar (sm), name, role tagline. Clicking opens a menu:
switch workspace, profile, logout.

---

**(2)** Main column. Top padding: `pt-0` because (3) is
sticky. Max content width: `max-w-3xl`. Horizontal padding:
`px-4 sm:px-6 lg:px-8`.

**(3)** Page header (sticky). Position: `sticky top-0 z-20`.
Background: `var(--background)/85` with `backdrop-blur-md`
so cards visually scroll *under* the header.

Title: `<h1>` text `"Approvals"`, `text-xl font-semibold
tracking-tight`.

Pending pill: inline next to title.
- Strings: `"{n} waiting"`. Hidden when n=0.
- StatusPill tone=`info` size=`sm` with dot.

Subtitle: `<p>` text `"Your AI teammates need a yes or no."`,
`text-sm text-[var(--muted-foreground)]`.

**(4)** Action buttons. Right-aligned. Three buttons:
- Filter: ghost icon button. Lucide `Filter`. Opens a
  filter popover (NOT in this surface's scope — out of v1).
- Notifications: ghost icon button. Lucide `Bell`. Opens
  notifications popover. Out of v1.
- Hire teammate: primary button with `Plus` icon. Label
  collapses at `< sm` breakpoint to icon-only.

**(5)** Mobile-collapsed primary button. At `< sm` (640px),
the "+ Hire teammate" label is hidden via `hidden sm:inline`
and only the `Plus` icon shows.

**(6)** Search input. `<input type="search">`. Strings:
placeholder `"Search by worker, action, or detail..."`.
Filters the visible cards in real time, debounced 150ms (set
in `useApprovals` consumer code). Filter is client-side on
worker.name + verb + detail substrings, case-insensitive.

Search updates URL `?q=` for shareability.

---

**(7)** Group header — Today. Format:
```
TODAY ────────────────────────────  6 requests
```
- Label: `text-xs font-semibold uppercase tracking-wider`,
  `text-[var(--muted-foreground)]`.
- Horizontal rule: `flex-1 h-px bg-[var(--border)]`.
- Count: `{n} requests` (or `{n} request` if n=1). Right-
  aligned, `text-xs text-[var(--muted-foreground)]`.

**(8)** Group header — Yesterday. Same shape. Format
identical with label `YESTERDAY`. Third group header is
`EARLIER` for everything > yesterday.

Cards appear under each group header in chronological
order (most recent first).

**(9)** Footer. Lives at the bottom of the scroll, NOT
sticky. Contains:
- Short text: `"SKYCOT preview — design system Track C.
  Switch theme to test white-label:"`.
- Embedded ThemePicker (light/dark/system + tenant
  dropdown).

Out of v2: footer goes away. Theme picker moves to
operator settings page. The current placement is a
development artifact useful while iterating on Track C.

---

## ApprovalCard (composite — the load-bearing element)

Each card is rendered by
[ApprovalCard.tsx](../../../../web/components/skycot/ApprovalCard.tsx).
Structural contract (DO NOT reorder):

1. Avatar + worker name + verb + channel/time chip.
2. Detail paragraph.
3. Consequence box (If YES / If NO).
4. Action row: Approve · Refuse · Why? (three buttons max).

Card states:

- **Default (pending)**: action row visible. Approve and
  Refuse are equal-width filled buttons; Why is ghost.
- **Loading (mid-mutation)**: clicked button shows spinner
  via Button `loading` prop. Other buttons disabled.
- **Post-decision (approved)**: action row replaced by
  PostDecisionBar — green stripe across the bottom with
  `"Approved"` label, `Clock` icon + countdown seconds,
  ghost `[ Undo ]` button. Lives for 30 seconds. After
  countdown elapses, Undo disappears, stripe stays.
- **Post-decision (refused)**: same but neutral tone (no
  green).
- **Reason expanded (Why? tapped)**: a divider line + a
  small block below the action row with the reason text.
  Strings: full sentence the worker provides as `reason`.

Card sizing: full-width inside the max-w-3xl column.
Internal padding: `px-5 py-4`. Vertical rhythm between
cards: `space-y-3`.

---

## States

### Loading state (initial fetch)

```
+----------------------------------------------------------------+
| (sidebar same as default)    |  (sticky header same)           |
|                              |                                 |
|                              |  +---------------------------+  |
|                              |  |     [ spinner ]           |  |
|                              |  |     Loading approvals…    |  |
|                              |  +---------------------------+  |
+----------------------------------------------------------------+
```

Renders in place of card list. Component: dashed border,
centered `Loader2` icon spinning, text
`"Loading approvals…"`. Implemented as `<LoadingState />`
in [app/page.tsx](../../../../web/app/page.tsx).

### Empty state (no approvals waiting, no search query)

```
+----------------------------------------------------------------+
|                              |  +---------------------------+  |
|                              |  |        [ 📨 ]             |  |
|                              |  |       All clear           |  |
|                              |  | Your teammates have       |  |
|                              |  | nothing waiting on you.   |  |
|                              |  | Get back to work — they've|  |
|                              |  | got the rest.             |  |
|                              |  +---------------------------+  |
+----------------------------------------------------------------+
```

Title: `"All clear"`.
Body: `"Your teammates have nothing waiting on you. Get
back to work — they've got the rest."`.

### Empty state (search query, no results)

Title: `"No matching approvals"`.
Body: `"Nothing matches "<query>". Try different keywords
or clear the filter."`.

### Error state (fetch failed)

```
+----------------------------------------------------------------+
|                              |  +---------------------------+  |
|                              |  |        [ ⚠ ]              |  |
|                              |  | Couldn't reach the gateway|  |
|                              |  | <error message>           |  |
|                              |  | [ Try again ]             |  |
|                              |  +---------------------------+  |
+----------------------------------------------------------------+
```

Title: `"Couldn't reach the gateway"`.
Body: the underlying error message from the query (e.g.
`"Failed to fetch"`).
CTA: secondary button `"Try again"` — calls `refetch()`.

### Permission-denied state

Out of v1 — the SPA assumes the bearer-auth check
already happened at the gateway layer. If the user
authenticated, they have read access to the inbox by
construction. RBAC restrictions on per-action visibility
are handled inside the cards themselves (PII redaction)
rather than blocking the surface.

---

## Mobile layout (< 768px)

- Side rail collapses. Hamburger button appears top-left
  of a slim mobile header bar.
- Tapping hamburger opens the rail as a bottom-sheet
  (Radix Dialog).
- Main column takes full viewport width.
- Sticky header reflows: action buttons (Filter, Bell,
  Hire) shrink to icon-only; "+ Hire teammate" label
  hidden.
- Search input full-width.
- ApprovalCards full-width.
- Touch targets ≥ 44px on Approve / Refuse / Why
  buttons (already in Button `size="md"` defaults).

At 375px width the cards are readable; long buttons
("Approve" / "Refuse") still fit on one row. At 320px
(small Android), Approve/Refuse stack 2-wide; Why drops
to a third row.

---

## Interactions + flows

### Card decision

1. User taps `[ ✓ Approve ]`.
2. Button enters loading state (spinner replaces check).
3. `useSubmitDecision.mutate()` fires:
   - `onMutate` runs optimistic update: card flips to
     post-decision state, page header pending pill
     decrements.
   - `mutationFn` POSTs to gateway (mock today).
4. On success: optimistic state stays. PostDecisionBar
   shows for 30s with `Undo` countdown.
5. After 30s elapse: Undo disappears, bar persists
   (visible until next fetch / page reload removes
   the card).

### Card undo

1. Within 30s, user taps `[ Undo ]`.
2. `useUndoDecision.mutate()` fires:
   - `onMutate` reverts optimistic update.
   - `mutationFn` POSTs undo to gateway (mock today).
3. Card returns to pending state. Header pending pill
   increments back.

### Search

1. User types in (6).
2. Client filters card list to matching items
   (debounce 150ms).
3. URL updates `?q=<text>`.
4. Group headers + counts update to reflect filtered set.
5. If nothing matches → empty-state (search variant).

### Hire teammate

1. User taps `[ + Hire teammate ]`.
2. Routes to `/hire` (the hire flow — see
   [wireframe-hire-flow.md](wireframe-hire-flow.md)).
3. After hire, user lands on the new worker's profile
   (see [wireframe-worker-profile.md](wireframe-worker-profile.md))
   or back at `/` with the new worker's first action
   visible (depends on whether the new worker has had a
   chance to surface one).

---

## Implementation notes for Track E

This surface is already partially implemented at
[web/app/page.tsx](../../../../web/app/page.tsx) as a
result of Track C scaffolding. Track E briefs against it
should be sized as:

- **E1-a: Approvals inbox — gateway wire-up.** Replace mock
  `fetchApprovals` in [lib/api/approvals.ts](../../../../web/lib/api/approvals.ts)
  with a real GET against `/api/v1/approvals` (depends on
  Track F-adjacent backend brief for the JSON endpoint).
- **E1-b: Approvals inbox — filter popover.** Implement the
  filter button (4) opening a popover for channel /
  status / worker filters. Adds URL state.
- **E1-c: Approvals inbox — notifications popover.**
  Implement (4) bell popover for system-level
  notifications.
- **E1-d: Approvals inbox — batch decide.** When N cards
  are similar (per
  [journey-approve-on-mobile.md §"approve all 8"
  batching](../journey-approve-on-mobile.md)), surface a
  batch action. Per-channel approver may want this
  especially.
- **E1-e: Approvals inbox — PII redaction layer.** Card
  bodies render with PII masked unless the user's role
  grants unredacted view. Per [auditor.md](../auditor.md).
- **E1-f: Approvals inbox — websocket live updates.**
  Replace polling/refetch-on-focus with a WS subscription
  to `/inbox/ws` (existing gateway endpoint) for
  approval-arrived events.

Each brief is ≤ 300 LOC. None alter the surface's structure
— they extend the implementation behind the wireframe.

---

## Empty + error copy (exact strings, locked)

| State | Title | Body |
|---|---|---|
| Empty (no query) | "All clear" | "Your teammates have nothing waiting on you. Get back to work — they've got the rest." |
| Empty (with query) | "No matching approvals" | `Nothing matches "${query}". Try different keywords or clear the filter.` |
| Error | "Couldn't reach the gateway" | `${error.message}` (fallback: "Something went wrong.") |
| Loading | n/a (no title) | "Loading approvals…" |

Track E briefs are NOT permitted to change these strings
without updating this doc.

---

## Open questions

`(needs validation)`

- Is 30s the right undo window, or should it be channel-
  aware (longer for high-stakes, shorter for routine)?
- Should batch-decide live in v1 or wait for real user
  feedback that the per-card cadence is painful?
- Should the PostDecisionBar fade out after the undo
  window or persist until next fetch?
- Should the footer ThemePicker move to operator settings
  immediately, or stay through Track E for white-label
  demo purposes?
- Is `?q=` the right URL state pattern, or should we use
  hash routing for shareable filters?
- At what scale (1000+ pending) does this surface fall
  apart? Pagination? Infinite scroll?

Plan to validate by:
1. Shipping E1-a (real gateway wire-up) and watching one
   delegator use it for a week.
2. A/B the undo window (30s vs 60s) on a subset of users.
3. Telemetry on search use rate — if < 5% of users search,
   it's noise; consider hiding behind a key shortcut.

---

**File status**: D1 — authored 2026-05-11 as the canonical
spec for the approvals-inbox surface. Updated alongside any
PR that touches that surface.
