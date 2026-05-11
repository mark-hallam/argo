# Wireframe — Team Page

The "meet your AI teammates" roster view. Each worker appears as
a card with avatar, status, and today's headline. This is where
the *teammate, not tool* metaphor lives at the surface level.
Click a card → worker profile.

## Purpose

Show the Delegator their AI teammates as people-with-roles, not
config rows. Make the team feel real. Surface today's headline
per teammate at-a-glance. Provide the entry-point to deeper
worker views (profile, retrain, audit).

**Serves:**
- Persona: [delegator.md](../delegator.md) (primary),
  [operator.md](../operator.md) (light — operator's view is
  Workers, not Team)
- Journey: [journey-hire-a-teammate.md](../journey-hire-a-teammate.md) (post-hire landing),
  [journey-delegate-ongoing-work.md](../journey-delegate-ongoing-work.md) (entry point)

**Implementation primitives:**
- [Card](../../../../web/components/ui/Card.tsx),
  [Avatar](../../../../web/components/ui/Avatar.tsx),
  [Button](../../../../web/components/ui/Button.tsx),
  [StatusPill](../../../../web/components/ui/StatusPill.tsx)
- NEW: `TeammateCard` (composite), `TeammateGrid` (layout).

## Information architecture

- URL: `/team`.
- Nav placement: 2nd nav item, between Approvals and Workers.
- No breadcrumb (top-level surface).
- Deep-link filters via query: `?status=working`,
  `?vertical=gtm-engineer`, `?q=<search>`.
- Each card links to `/workers/<workerId>` (the profile —
  [wireframe-worker-profile.md](wireframe-worker-profile.md)).

## Layout (default state, desktop)

```
+----------------------------------------------------------------+
| (sidebar same as approvals inbox)                              |
|                              |                                 |
|                              |  +---- STICKY HEADER --------+ |
|                              |  | (1) Team                  | |
|                              |  |     4 teammates working   | |
|                              |  |                                                  (2)|
|                              |  |  [ Status v ] [ + Hire ] |
|                              |  +-----------------------------+
|                              |  | (3) [ 🔍 Filter by name... ]   |
|                              |  +-----------------------------+
|                              |                                 |
|                              |   (4) GRID — 2 columns desktop  |
|                              |                                 |
|                              |  ┌─────────────┐  ┌─────────────┐|
|                              |  │ (M)  Mariah │  │ (P)  Priya  │|
|                              |  │ ● Working  │  │ ● Waiting   │|
|                              |  │             │  │             │|
|                              |  │ Marketplace│  │ Recruiter   │|
|                              |  │ assistant   │  │ for Acmeco  │|
|                              |  │             │  │             │|
|                              |  │ Today:      │  │ Today:      │|
|                              |  │ • 8 replies │  │ • 2 inter-  │|
|                              |  │   sent (2   │  │   views     │|
|                              |  │   waiting   │  │   booked    │|
|                              |  │   your OK)  │  │ • 1 rejec-  │|
|                              |  │ • $340 made │  │   tion      │|
|                              |  │             │  │   waiting   │|
|                              |  │ [Open →]    │  │   you (5)   │|
|                              |  └─────────────┘  └─────────────┘|
|                              |                                 |
|                              |  ┌─────────────┐  ┌─────────────┐|
|                              |  │ (J)  Jamal  │  │ (N)  Nori   │|
|                              |  │ ● Idle      │  │ ● Working   │|
|                              |  │ ... etc ... │  │ ... etc ... │|
|                              |  └─────────────┘  └─────────────┘|
|                              |                                 |
|                              |  ┌─────────────────────────────┐|
|                              |  │  + Hire another teammate     │ (6)|
|                              |  │  ··· dashed border ···      │|
|                              |  └─────────────────────────────┘|
+----------------------------------------------------------------+
```

## Annotations

**(1)** Page title. Strings: `"Team"`. Always plural. Below
it, subtitle: `"{n} teammates working"` where n =
count(status != "blocked" && status != "paused"). Hidden
when n=0 (uses empty state — see below).

**(2)** Action row. Two controls:
- Status filter dropdown. Strings: `"Status v"`. Options:
  `All`, `Working`, `Waiting on you`, `Idle`, `Blocked`,
  `Paused`. Default: `All`.
- `[ + Hire ]` primary button. Same component as the
  approvals-inbox header's CTA. Routes to `/hire`.

**(3)** Search input. Strings: placeholder `"Filter by
name..."`. Client-side fuzzy match on worker.name +
worker.role.tagline. Debounced 150ms. Updates URL `?q=`.

**(4)** Teammate grid. CSS grid. Responsive columns:
- `< 640px`: 1 column.
- `640-1024px`: 2 columns.
- `> 1024px`: 3 columns.

Gap: `gap-4`.

**(5)** TeammateCard (composite). One per worker.
See "TeammateCard composition" below.

**(6)** Hire-another tile. Dashed border, ghost-style
appearance, matches grid cell size. Hover state: solid
border + accent color. Tap: routes to `/hire`.

---

## TeammateCard composition

```
┌─────────────────────────────────────┐
| ╭───╮                                |
| │ M │  Mariah                  (a)   |
| ╰───╯  ● Working                (b)   |
|                                       |
|   Marketplace assistant         (c)   |
|                                       |
|   Today:                        (d)   |
|   • 8 replies sent (2 waiting        |
|     your OK)                          |
|   • $340 in pending offers           |
|                                       |
|   Tomorrow: morning routine          |
|   ↳ inbox summary at 06:30           |
|                                       |
|   ─────────────────────              |
|                                       |
|   [ Open profile → ]            (e)   |
└─────────────────────────────────────┘
```

**(a)** Avatar (lg) + name. Name uses `font-semibold
text-base text-[var(--foreground)]`. Hover: name color
shifts to accent.

**(b)** Status indicator. Dot + label:
- `Working` — `var(--info)` dot, "Working" label.
- `Waiting on you` — `var(--warning)` dot. Click-through
  goes to the approvals inbox filtered to this worker.
- `Idle` — neutral muted dot. "Idle" label.
- `Blocked` — `var(--destructive)` dot. "Blocked".
- `Paused` — muted dot + `Paused` label. Implies operator
  paused the worker.

**(c)** Role tagline. One line, generated from the
worker's charter. `text-sm text-[var(--muted-foreground)]`.

**(d)** Today block. Headline format derived from the
worker's recent activity:
- Bullet 1: highest-volume action of the day (`"{n}
  replies sent"`, `"{n} tickets resolved"`, `"{n}
  drafts queued"`).
- Bullet 2: any approvals waiting (`"{n} waiting your
  OK"`).
- Optional bullet 3: revenue / KPI surfacing (`"$340 in
  pending offers"`, `"3 interviews booked"`).

Plus a one-line "Tomorrow" preview if duties are
scheduled in the next 24h.

The exact bullets are vertical-aware — gtm-engineer
shows revenue; recruiter shows pipeline counts;
customer-support shows resolution rate.

**(e)** Open profile button. Ghost variant, full-width
at the bottom of the card. Strings: `"Open profile →"`.
Routes to `/workers/<workerId>`.

Card hover state: subtle shadow + slight lift (-2px Y).

---

## States

### Loading state

Grid renders skeleton cards (~6 of them, same dimensions).
Each skeleton shows: avatar circle, name bar, status bar,
2 content lines, button placeholder.

### Empty state — no teammates ever hired

```
+----------------------------------------------------------------+
|                              |                                 |
|                              |       (large illustration)      |
|                              |                                 |
|                              |       No teammates yet.         |
|                              |                                 |
|                              |       What do you need help     |
|                              |       with day-to-day?          |
|                              |                                 |
|                              |       [ + Hire your first       |
|                              |         teammate ]              |
|                              |                                 |
|                              |       It takes about 5 minutes. |
+----------------------------------------------------------------+
```

Headline: `"No teammates yet."` Subline: `"What do you
need help with day-to-day?"` Primary CTA: `[ + Hire your
first teammate ]`. Sub-CTA tagline: `"It takes about 5
minutes."` Routes to `/hire`.

The illustration can be deferred to v2 — use a large
muted-color People icon at 48px for v1.

### Empty state — search returns nothing

Same dashed-border block as the approvals-inbox empty
state, but with team-specific copy:
- Title: `"No matching teammates"`.
- Body: `"Nothing matches "${query}". Try a different
  search or clear the filter."`.

### Empty state — filter returns nothing

Title: `"No teammates in this state right now"`.
Body: dynamic per filter:
- `"No teammates are working right now."`
- `"Nothing waiting on you. Get back to it."` (matches
  approvals-inbox tone)
- `"No teammates are idle."`
- `"No blocked teammates."`
- `"No paused teammates."`

### Error state

Same dashed-border card as approvals-inbox. Title:
`"Couldn't load your team"`. Body: the error message.
CTA: `[ Try again ]`.

### Permission-denied state

Out of v1 — if user has gateway auth, they can see their
own team. Cross-team visibility (e.g. operator viewing
team rosters across tenants) is the Workers page, not
this one.

---

## Mobile layout (< 768px)

- Grid: 1 column.
- Cards take full viewport width minus padding.
- Side rail: collapsed under hamburger.
- Sticky header: action row collapses; `+ Hire` becomes
  icon-only at `< sm`.
- Status filter dropdown remains visible but compacts to
  icon + count (`🔻 4`).
- Search remains full-width.
- Hire-another tile renders at the bottom, full-width.

Touch targets all ≥ 44px (StatusPill at `size="md"`,
buttons at `size="md"`).

---

## Interactions + flows

### Open profile

- Tap card body (anywhere except the explicit button):
  routes to `/workers/<workerId>`.
- Tap `[ Open profile → ]` button: same.
- Tap card title or avatar: same (alternative tap target
  for accessibility).

### Open approvals filtered by worker

- Tap the `Waiting on you` status badge: routes to
  `/?worker=<workerId>` (the approvals inbox with the
  worker filter pre-applied).

### Hire another

- Tap `[ + Hire ]` in the header OR the `+ Hire another
  teammate` dashed tile: routes to `/hire`.

### Pause / resume

Quick action on each card via a context menu (right-click
desktop, long-press mobile, or a small `⋮` button top-
right of each card):
- `Pause Mariah` (if Working) → confirm dialog → pauses
  the worker. Card flips to `Paused` state. The pause
  carries through gateway audit (per
  [auditor.md](../auditor.md)).
- `Resume Mariah` (if Paused) → resumes immediately.

Pause does NOT delete or modify the worker's state — it
just stops new actions from firing.

### Reorder

Workers in the grid sort by:
1. `Waiting on you` first (most urgent at top).
2. `Working` next.
3. `Idle` and `Paused` after.
4. `Blocked` at the bottom (so it doesn't dominate
   visually unless filtered).

Within each status group, sort by last-action timestamp
descending.

---

## Empty + error copy (exact strings, locked)

| State | Strings |
|---|---|
| Loading | (no text — skeletons only) |
| Empty: no teammates ever | Title: `"No teammates yet."` · Body: `"What do you need help with day-to-day?"` · CTA: `"+ Hire your first teammate"` · Tagline: `"It takes about 5 minutes."` |
| Empty: search no match | Title: `"No matching teammates"` · Body: `"Nothing matches \"${query}\". Try a different search or clear the filter."` |
| Empty: filter no match | Per filter (see "States" §). |
| Error | Title: `"Couldn't load your team"` · Body: `${error.message}` (fallback: `"Something went wrong."`) · CTA: `"Try again"` |
| Pause confirm | Title: `"Pause Mariah?"` · Body: `"She'll stop taking new actions until you resume her. Current pending approvals stay where they are."` · CTAs: `[ Cancel ]` `[ Pause ]` |
| Pause toast (after) | `"Mariah paused. Resume from her profile."` |

---

## Implementation notes for Track E

This surface is data-light and layout-heavy. Brief slicing:

- **E3-a: Team page — route + sticky header + grid shell.**
  Route at `/team`, sticky header, responsive grid, empty
  state when zero teammates. Connects to a (mock for now)
  `useTeam()` hook returning a `Worker[]`.
- **E3-b: Team page — TeammateCard component.** Composite
  that renders avatar + status + role + today block +
  open-profile button. Receives a `Worker` prop. Skeleton
  state included.
- **E3-c: Team page — Today block headline generator.**
  Library function that takes a worker's recent audit
  events and produces 2-3 bullet headlines per vertical.
  Tested against shipped verticals. Backend brief, not UI.
- **E3-d: Team page — filter + search.** Status dropdown
  + search input. URL state.
- **E3-e: Team page — pause/resume.** Context menu on each
  card. Confirm dialog. Mutation against the gateway.

5 briefs total. Mostly ≤ 200 LOC each.

New primitives:
- `TeammateCard` (composite, ~200 LOC) — lives in
  `components/skycot/`.
- `TeammateGrid` (layout, ~50 LOC) — responsive grid
  wrapper, probably inline in `app/team/page.tsx`.
- `DropdownMenu` from shadcn for the status filter +
  card context menu. Already a Radix primitive shadcn
  ships as copy-paste.

The `useTeam()` hook lives in `lib/hooks/use-team.ts`
mirroring `use-approvals.ts`. The `Worker` type lives in
`lib/types/worker.ts`. Both currently shipped as mock,
swap to real gateway when JSON `/api/v1/team` endpoint
lands (Track F-adjacent backend work).

---

## Cross-references

- Workers vs Team disambiguation: see
  [operator.md](../operator.md) "Operator console — the
  surface they live in" — the Workers page is the
  *fleet/admin* view (used by operators to manage across
  tenants); the Team page is the *roster/relational* view
  (used by delegators to interact with their own team).
- Worker profile is the deep view per card: see
  [wireframe-worker-profile.md](wireframe-worker-profile.md).
- Hire flow is the +Hire button destination: see
  [wireframe-hire-flow.md](wireframe-hire-flow.md).
- Approvals filtered by worker: see
  [wireframe-approvals-inbox.md](wireframe-approvals-inbox.md)
  for the `?worker=` URL param.

---

## Open questions

`(needs validation)`

- Is the "Today" block headline-per-vertical right, or do
  delegators want a uniform shape regardless of vertical?
- Is 2-3 columns at desktop the right density, or should
  it be 1 column with wider cards?
- Should there be a "team mood" indicator at the top
  ("everyone's on track" / "2 teammates need you") or is
  that gimmicky?
- Should we surface KPIs at the team level (total revenue
  across all teammates, total tickets resolved this week)?
  Out of v1 scope; revisit after first delegators give
  feedback.
- Should pause/resume require a reason for the audit log,
  or is timestamp + actor enough?
- Should the dashed "+ Hire another" tile appear above or
  below the existing teammates? Above signals "add more";
  below signals "you have these already". Defaulting to
  below; revisit with telemetry.

Plan to validate by:
1. Showing 5 delegators a paper prototype + asking "what
   would you click first?"
2. Watching first-week click rates on the card body vs
   the explicit Open Profile button.
3. Pause/resume usage telemetry — is it a real flow or
   theatre?

---

**File status**: D3 — authored 2026-05-11 as the canonical
spec for the team-page surface. Updated alongside any PR
that touches the page.
