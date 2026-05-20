# Wave-12 Audit #3 — Switching Between Teammates' Workflows

**Date:** 2026-05-21
**Auditor:** Wave-12 agent #3
**Scope:** The "operator manages 5 teammates in quick succession"
journey — `/team` → `/workers/[id]/[tab]` → `/workflows` →
`/workflows/[v]/[name]` → `/workflows/[v]/[name]/build` and back.

## Method

Static read-through of the routes, layouts, shells, hooks, and
components that participate in the journey:

- `web/app/(app)/team/page.tsx`
- `web/app/(app)/workers/[workerId]/layout.tsx` +
  `WorkerLayoutShell.tsx` + `[tab]/page.tsx` + `activity/page.tsx` +
  `overview/page.tsx`
- `web/components/skycot/WorkerTabs.tsx`
- `web/components/skycot/TeammateCard.tsx`
- `web/app/(app)/workflows/page.tsx` +
  `[verticalId]/[name]/page.tsx` + `[verticalId]/[name]/build/page.tsx`
- `web/components/skycot/AppShell.tsx` + `nav-items.tsx`
- `web/lib/hooks/use-worker.ts` + `use-notifications.ts` +
  `lib/sample-notifications.ts`

Six switch-flow dimensions were checked, mirroring the wave-7
search-filter shape:

1. URL update on tab change inside a worker
2. State clear when switching between workers
3. `/workflows` filter URL-persistence (per-worker filter)
4. Builder unsaved-state preservation when switching workflows
5. Breadcrumb back-navigation after deep dives
6. Notifications badge per-worker scoping

## Findings

### F1 — No "Workflows" tab inside a worker profile  (HIGH)

`WorkerTabs.tsx:19-39` defines exactly seven tabs:

```
overview, activity, memory, skills, trust, escalation, settings
```

There is **no "Workflows" tab** under `/workers/[id]/*`. A persona
who runs five teammates can see each worker's **Skills**, **Memory**,
and **Activity**, but the workflows the teammate executes are only
reachable through the global `/workflows` catalog, which has no
worker filter (see F3). Result: to compare "what workflows is
Mariah doing this week vs Priya?" the operator has to leave the
worker context entirely, re-find each workflow by reading the
`lastEditor` byline on `/workflows`, and infer which teammate owns
it.

The audit page builds a worker-filter URL pattern
(`/audit?worker=<id>`, see
`web/components/skycot/audit/ActionSummaryHero.tsx:202` and
`web/components/skycot/worker/SkillsActiveCard.tsx:93`). Workflows
deliberately doesn't.

### F2 — Worker tabs DO update the URL and DO clear state cleanly  (PASS)

`WorkerTabs.tsx:80-101` renders each tab as a real Next `<Link>` to
`/workers/<id>/<tab>`, with `aria-current="page"` driven from
`usePathname()`. URL updates per tab, reload preserves tab, deep
linking works.

State-clear between workers also works: `WorkerLayoutShell.tsx:39`
calls `useWorker(workerId)` and the TanStack key is
`["worker", workerId]` (`use-worker.ts:7`). When `workerId` changes,
the cache key changes, so the layout re-suspends and the hero
re-renders with the new teammate. No stale name flash.

The **redirect index** at `workers/[workerId]/page.tsx:16` also
short-circuits cleanly: `/workers/X` → `/workers/X/overview`.

### F3 — `/workflows` filter does NOT URL-persist  (CRITICAL — wave-7 regression)

`workflows/page.tsx:118` is the smoking gun:

```ts
const [query, setQuery] = useState("");
```

The workflows-catalog search is plain component state. No
`useSearchParams`, no `router.replace`, no debounce-to-URL. Compare
the team-catalog page which spent 30+ lines wiring search to the
URL (`team/page.tsx:64-130`).

Switch-friction consequences when juggling five teammates:

- Operator types "approval" to find every approval workflow → tab
  away to triage a notification → tab back → search box is empty
  and the catalog has re-sorted
- Cmd+click a workflow card to inspect in a new tab → that new tab
  has no idea what search context generated the click → operator
  loses orientation
- Browser back/forward across the catalog cannot replay state
  because the state never existed in URL or history

The page is also missing the **worker filter** entirely. There is
no `?worker=wkr_01J0MARIAH` URL pattern, no chip-rail, no dropdown
to scope by editor. The wave-7 search-filter audit explicitly
called out URL-persistence as a SHOULD; the workflows catalog
shipped without it.

### F4 — Builder destroys unsaved state silently  (CRITICAL)

`workflows/[verticalId]/[name]/build/page.tsx:311-317`:

```ts
const [selectedNode, setSelectedNode] = useState<Node | null>(null);
const [promptText, setPromptText] = useState("");
const [proposing, setProposing] = useState(false);
const [triggerConfigOpen, setTriggerConfigOpen] = useState(false);
const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
```

Five `useState` hooks own everything the operator types or toggles:

- The AI-prompt textarea ("Add a step that emails the seller before posting")
- Which node's side panel is open
- The trigger-config drawer state
- The map of step-id → trigger config (the real editing payload)

When the operator clicks "ALL WORKFLOWS" (`build/page.tsx:635`), or
the sidebar nav, or back-arrows out, **no `beforeunload`
listener fires**, **no `onbeforeunload` toast**, **no
`sessionStorage` snapshot**. `triggerConfigs` is gone the instant
the route changes.

The toasts on `handleSaveAndPublish` and `handleProposeChange`
already advertise that nothing is persisted ("preview build, draft
not persisted"), but the operator does not see those until they
actually click Save. The **expected** UX — "you have unsaved
changes, leave anyway?" — does not exist.

This compounds with F3 because the operator's most likely route
into the builder is `/workflows` → click → realize wrong workflow →
back → click another → unsaved AI-prompt-text from workflow 1 is
silently lost.

### F5 — Breadcrumb is missing on the workflows detail viewer and the builder  (MEDIUM)

`WorkerLayoutShell.tsx:97-109` ships a proper breadcrumb
(`Team › Mariah`) inside the worker layout. The workflows surfaces
do not:

- `workflows/[verticalId]/[name]/page.tsx:732-735` renders
  `<Link href="/workflows" className="wfd-back">ALL WORKFLOWS</Link>`
  — a single back-link, not a breadcrumb. The user cannot see the
  hierarchy (`Workflows › support/triage_ticket`).
- `workflows/[verticalId]/[name]/build/page.tsx:635` reuses the
  same single back-link.

After a deep journey (`/team` → click Mariah → click Audit timeline
deep-link → back-button → land on `/audit?worker=…` rather than
the worker profile), the operator has **no breadcrumb on
intermediate surfaces** to anchor where they are. The audit page
itself (`web/app/(app)/audit/`) lacks a breadcrumb component too —
confirmed by `find ... breadcrumb` returning nothing under that
directory.

### F6 — Activity-tab filter `router.replace("?")` strips path  (LOW — works but fragile)

`workers/[workerId]/activity/page.tsx:49-58`:

```ts
router.replace(qs ? `?${qs}` : "?", { scroll: false });
```

The `?…` URL is relative, so Next resolves it against the current
path. This works today, but means **switching from
`/workers/A/activity?range=30d&type=email_drafted` to
`/workers/B/activity` keeps the relative reference** — so if the
operator lands on `B` via internal routing the activity filters
from worker A do not carry over (because the path segment changed,
which is what we want), but if the URL is reconstructed from a
saved link, the bare `?…` form is brittle if the route shape
changes. Today this is benign. Worth flagging because the same
pattern + a route refactor would silently leak filters cross-worker.

### F7 — Notifications badge is global, not per-worker  (MEDIUM)

`use-notifications.ts:11-28` keys on a singleton `["notifications"]`
TanStack query, and `lib/sample-notifications.ts` carries no
`workerId` field on `NotificationItem`. The bell popover therefore
shows the same global feed regardless of which worker profile the
operator has open.

This is a UX expectation gap rather than a bug: if the operator is
deep inside Mariah's profile and notification "Gmail connector needs
re-authentication" arrives because Sam's worker is the one with the
expired token, the operator has no signal that the notification is
about a teammate other than the one they're looking at.

The Activity tab links to `/audit?worker=<id>` (filtered), and
`SkillsActiveCard` links to `/audit?worker=<id>&skill=<id>`, so the
data model can carry `workerId`; the notifications surface just
isn't using it yet.

### F8 — TeammateCard click target uses `/workers/<id>` not `/workers/<id>/overview`  (LOW — works but causes a redirect)

`TeammateCard.tsx:118` sets `profileHref = "/workers/${worker.id}"`,
which lands on `workers/[workerId]/page.tsx:16` and `redirect`s to
`/overview`. Same final state, but every card click triggers a
server-side redirect chain instead of going straight to the tab.
Five teammates × five card clicks per session × redirect overhead
× perceived flash. Cheap fix: change the href to `…/overview`.

## Six-dimension scorecard

| Dimension | State | Evidence |
|---|---|---|
| 1. Worker-tab URL updates | PASS | `WorkerTabs.tsx` uses real Next `<Link>` to `/workers/<id>/<tab>` |
| 2. State clear between workers | PASS | TanStack key `["worker", workerId]` re-keys cleanly |
| 3. `/workflows` filter URL-persistence | **FAIL** | `useState("")` not `useSearchParams`; no per-worker filter; wave-7 regression |
| 4. Builder unsaved-state on switch | **FAIL** | 5 `useState` hooks, zero `beforeunload`, zero `sessionStorage` |
| 5. Breadcrumb after deep nav | PARTIAL | Worker layout has breadcrumb; workflows + audit do not |
| 6. Notifications badge per-worker | FAIL (by design) | Global TanStack singleton, no `workerId` on `NotificationItem` |

## Switch-friction score

**6 / 10** (where 10 is "switching is invisible to the operator")

Worker layout itself is solid (F2 pass). The drag is everything
**outside** the worker layout — the catalog (F3), the builder (F4),
the breadcrumb gap on workflows + audit (F5), and the global
notifications model (F7). The "manage five teammates in quick
succession" persona will lose context every time they leave the
worker profile.

## Top 3 navigation bugs

1. **`/workflows` catalog has no URL-persisted search and no
   worker filter** (F3). The operator cannot scope the catalog to
   one teammate at all, and even the basic search is reset on
   every nav. Fix: lift the `useState("")` query into
   `useSearchParams`/`router.replace` (mirroring `team/page.tsx`),
   add a `?worker=<id>` filter, and add a "Workflows" tab inside
   `/workers/[id]/*` that deep-links into this filtered view.

2. **Builder silently destroys unsaved AI-prompt and
   trigger-config state when the operator clicks away** (F4). Add
   a `beforeunload` handler when `promptText.trim().length > 0` or
   `Object.keys(triggerConfigs).length > 0`, plus a
   `sessionStorage` snapshot keyed on `${verticalId}/${name}` so
   the operator can pick up where they left off.

3. **No breadcrumb on workflows detail viewer, builder, or audit**
   (F5). Reuse the `<Breadcrumb>` primitive already in
   `WorkerLayoutShell.tsx` — `Workflows › support ›
   triage_ticket` on the viewer, `… › triage_ticket › Build` on
   the builder. This is a five-line copy-paste fix per surface
   and removes the deepest UX disorientation in the journey.

## Recommended ordering for fixes

1. Fix F8 (`profileHref` → `…/overview`) — one-line change, zero risk,
   removes the redirect flicker on every card click.
2. Fix F3 (URL-persist `/workflows` search + add worker filter +
   add Workflows tab on worker) — biggest persona win.
3. Fix F4 (builder `beforeunload` + sessionStorage snapshot) —
   prevents the worst "I just lost my work" buyer moment.
4. Fix F5 (breadcrumbs on workflows + audit) — same primitive as
   the worker layout, copy-paste pattern.
5. Defer F6 (relative URL `router.replace("?")` is benign today).
6. Defer F7 (notifications per-worker scoping) until backend
   `J*-gateway-notifications-json-api` lands; the data model isn't
   ready.
