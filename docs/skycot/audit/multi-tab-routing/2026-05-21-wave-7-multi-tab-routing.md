# Wave-7 Audit #4: Multi-Tab Sync + Back-Button + URL-Share Fidelity

**Author**: claude-opus-4-7 (wave-7 audit agent #4)
**Date**: 2026-05-21
**Scope**: `web/` (Next.js 15 frontend, App Router, "use client" pages)
**Method**: ripgrep-led, callsite-level audit of routing + history primitives. No runtime probing.

---

## TL;DR

- **Multi-tab sync**: PARTIAL. **Only the approvals inbox** (`/`) auto-syncs across tabs via a server-side WebSocket fan-out (`/inbox/ws`). One client-side `storage` event listener (elevated-view grants). **No other surface** propagates changes. Tab A approves → Tab B sees it. Tab A freezes a worker → Tab B's `/workers/...` view stays stale until you reload or tab-focus refetches (TanStack default).
- **Back/Forward**: Mostly respected. Filter changes use `router.replace()` (no history pollution). Navigation between pages uses `router.push()` (back works). Audit detail's `/audit/action/<id>` back button uses `router.back()` ✓. Forward-button correctness is implicit (Next.js handles it). **5 modals do NOT integrate with history** — pressing Back while a modal is open navigates the page, not closes the modal.
- **URL-share fidelity**: MIXED. 7 pages encode all filters in URL (audit, team, retro, dashboard filters, workers activity, billing subscribe, replay). 3+ pages use `useState("")` for search query that does NOT round-trip (workflows, workflow templates, skills marketplace, cohorts open-row, orchestration filters, dashboard search-text only).

**Verdict**: SPA respects browser-history conventions at the route level, but **state-in-URL coverage is partial** and **multi-tab sync is missing for everything except approvals**. Procurement reviewers using Back constantly will be fine; URL-pasting will sometimes silently fail to apply filters; multi-tab "approve in A, see in B" works only on the approvals inbox.

---

## 1. Multi-tab sync inventory

### Cross-tab sync primitives in the codebase

| Primitive | Files | Coverage |
|---|---|---|
| `BroadcastChannel` | 0 | none |
| `addEventListener("storage", …)` | 1 (`lib/hooks/use-elevated-view-grant.ts:118`) | elevated-view grants only |
| `SharedWorker` / leader election | 0 | none |
| Server-push WebSocket | `lib/api/websocket.ts` + `lib/hooks/use-approvals-live.ts` | approvals inbox only |
| `refetchOnWindowFocus` (TanStack default) | global (`components/skycot/QueryProvider.tsx:23`) | all TanStack-Query-backed pages |
| Manual `window.addEventListener("focus", …)` | 1 (`use-approvals-live.ts:216`) | approvals inbox (defense-in-depth) |

### What this means in practice

- **Approvals inbox (`/`)**: a 2-tab user gets near-real-time sync. The gateway emits `approval.{created,decided,undone,expired}` over `/inbox/ws` and every connected tab applies the cache mutation. Tab A approves → tab B's card flips in <100ms (per the `applyEvent` logic at `use-approvals-live.ts:126`). **This is the gold standard.**
- **Every other page** (workers, audit timeline, audit detail, skills, workflows, cohorts, retros, settings, orchestration, support inbox, evals): tab A makes a change, tab B sees it only when:
  - The user manually refreshes, OR
  - The user switches to tab B and TanStack Query's `refetchOnWindowFocus: true` fires (30s stale-time gate).
- **Orchestration page is worse**: it uses raw `useState + fetchOrchestrationRuns()` in `useEffect`, NOT TanStack Query (`app/(app)/orchestration/page.tsx:24-41`). So tab-focus refetch does NOT happen — the page is frozen on its initial fetch until you reload.
- **Hire draft + onboarding draft localStorage**: two tabs starting a hire flow concurrently will silently clobber each other's drafts (`lib/hire/draft-store.ts`, `lib/onboarding/use-onboarding-draft.ts`) — no storage-event sync, no last-writer detection.
- **Per-tab WebSocket cost**: each tab opens its own `/inbox/ws` connection — no SharedWorker dedupe. A procurement reviewer running 5 tabs = 5 WebSocket connections + 5x the ping/pong heartbeat traffic.

---

## 2. Back-button correctness

### `router.push()` vs `router.replace()` discipline

| Pattern | Used by | Effect |
|---|---|---|
| `router.replace()` for filter changes | `/` (dashboard), `/team`, `/audit`, `/workers/[id]/activity`, `/retro` | ✓ Back skips through filter steps — back closes the page, not 12 filter taps |
| `router.push()` for page navigation | hire-flow, onboarding, audit detail → replay, command palette, billing | ✓ Back returns to prior page |
| `router.back()` for explicit Back buttons | audit detail, hire shell, auth wizard | ✓ Native browser-history semantics |
| `router.refresh()` after mutation | worker freeze | ✓ Stays on same URL, just re-renders SSR data |

### Confirmed bugs / smells

1. **Audit-detail back-button has a documented fallback that doesn't exist.** The doc comment at `app/(app)/audit/action/[actionId]/page.tsx:36-37` claims: "If history is empty (deep-link), the button degrades to a static link back to `/audit`." But the implementation at line 123 unconditionally calls `router.back()`. Deep-link visitors who press Back will navigate to wherever they came from outside the SPA (e.g., Slack), not `/audit`. Severity: minor UX confusion.
2. **Modals do not push history entries.** `FreezeButton` modal, `AddFactDialog`, `EvidenceBundleExporter`, audit `AuditFilterPopover`, worker `RetrainDialog` all use plain `useState(false)` for open/close. Pressing the device Back button on mobile while a modal is open will navigate AWAY from the page instead of closing the modal — a standard mobile-UX expectation violation. Severity: medium for mobile; the demo path is desktop so v1 launch isn't blocked.
3. **`HireShell.attemptLeave` uses `router.push("/")` not `router.replace("/")`** (`components/skycot/HireShell.tsx:86,91`). Exiting a hire flow adds a history entry. Pressing Back from the dashboard returns to the abandoned hire flow — not what users expect from a "discard" action. Severity: minor.
4. **`useSearchParams` without `<Suspense>` boundary** in 4 production pages (`workers/[id]/activity`, `workers/[id]/skills/[id]/canvas`, `workers/[id]/skills/[id]/canvas/diff`, `workers/[id]/skills/[id]/code`). Next.js 15 forces these routes into dynamic rendering at build, generating a console warning and disabling prerender. Severity: minor (build warning, not a runtime failure).
5. **Forward button**: Untested but inferred-correct from `router.push` discipline. If Back returns to a prior URL with `?worker=…`, the URL state is restored from `useSearchParams`, so Forward should restore the next URL. No explicit `scrollRestoration` configuration found, so scroll position on Forward may not restore — minor UX gap.

---

## 3. URL-share fidelity, per page

### Pages where URL captures all user-visible state (✓ share-safe)

| Page | URL params handled | Quality |
|---|---|---|
| `/audit` | `range`, `q`, `worker`, `channel`, `action_type`, `status`, `approver`, `cost_band`, `classification` | Excellent — 9 dimensions, URL is source of truth, parses with allow-list per dimension |
| `/team` | `q`, `status` | Excellent — `router.replace`, 150ms debounce |
| `/workers/[id]/activity` | `range`, `type`, `status` | Good (but no `<Suspense>` boundary — see bug #4) |
| `/retro` | `q` | Good — debounced replace |
| `/audit/action/[id]` | path-segment only | OK — page is itself the deep-link target |
| `/` (dashboard) approval filters | `worker`, `channel`, `status` | OK — but search-text query is NOT in URL (see below) |
| `/audit/action/[id]/replay` | search params parsed | OK |
| `/billing/subscribe` | plan param | OK |

### Pages where URL does NOT capture filter state (✗ share-breaks)

| Page | What's lost on paste | Source |
|---|---|---|
| `/workflows` | search query (`useState("")`) | `app/(app)/workflows/page.tsx:118` |
| `/workflows/templates` | search query (`useState("")`) | line 53 |
| `/skills` | search query (`useState("")`) | `app/(app)/skills/page.tsx:53` |
| `/cohorts` | `openCohortId` (which cohort row is expanded) | `app/(app)/cohorts/page.tsx:13` |
| `/orchestration` | ALL filters (`OrchestrationFilters`) — worker, skill, status | `app/(app)/orchestration/page.tsx:25` |
| `/` (dashboard) | search-text typed in input box — workers/channel/status ARE in URL, but `query` is not | line 81 |
| `/support/inbox` | confirmed no `useSearchParams` — all selection is local |

### URL-share verdict per demo path

- **`/audit?worker=wkr_01J0MARIAH`** → ✓ pre-filters correctly. CSV-aware (e.g. `?worker=wkr_01,wkr_02`).
- **`/audit?q=approval`** → ✓ search input pre-fills + filters.
- **`/workflows?q=approval`** → ✗ search input is blank; full list shown.
- **`/skills?q=triage`** → ✗ search input is blank; full list shown.
- **`/?worker=wkr_01J0MARIAH&channel=slack`** → ✓ approval cards pre-filtered.
- **`/?q=refund`** → ✗ search-text box blank; cards unfiltered.
- **`/workers/wkr_01J0MARIAH/audit`** → no such route; the worker layout has tabs `[tab]/page.tsx` + `activity` + `evals` + `memory` + `overview` + `responsibilities` + `settings` + `skills` + `trust`. Bookmarking would 404 (or fall through to the `[tab]` placeholder).

---

## 4. Top-5 back-button / multi-tab bugs

1. **No cross-tab sync outside `/`** — operator approves in tab A, then reviews the same worker in tab B's `/workers/<id>/activity` page, sees old data until tab-focus refetch. Confusing during live demos. **Fix**: server-push WebSocket or BroadcastChannel-based cache invalidation.
2. **Orchestration page is fully stale** — uses raw `useState`, no TanStack, no focus-refetch. Tab-focus does not refresh. **Fix**: migrate to `useQuery({ queryKey: ["orchestration"] })`.
3. **Modals do not integrate with browser history** — mobile Back button navigates the page instead of closing the modal. **Fix**: wrap dialog open/close in a history-state push, or accept this as a known v1 limitation.
4. **`/workflows`, `/skills`, `/cohorts`, `/orchestration`, dashboard search-text** — share-via-URL silently fails. Slack-paste lands on the right page with empty filter state. **Fix**: lift `useState` to `useSearchParams` (existing pattern in `/audit`, `/team`).
5. **Audit-detail back button claims a `/audit` fallback it doesn't implement** — `router.back()` will exit the SPA on deep-link visits. **Fix**: detect `window.history.length <= 1` and fall through to `router.push("/audit")` per the doc comment.

---

## 5. Verdict on browser-history conventions

The SPA **respects browser-history conventions at the navigation level**:
- Filter changes use `router.replace()` (no spam in the back-stack).
- Page navigation uses `router.push()` (back works).
- Explicit Back buttons use `router.back()`.
- Forward is implicit via Next.js's history adapter.

It **partially respects conventions at the state-in-URL level**:
- 7 pages encode their state in the URL.
- 5+ pages keep state in local `useState`, breaking URL-share.

It **does not implement multi-tab sync except for the approvals inbox**:
- The WebSocket fan-out for `/` is the only cross-client mechanism.
- One storage-event listener exists, but only for elevated-view grants.
- No `BroadcastChannel`, no `SharedWorker`, no leader election.
- TanStack's `refetchOnWindowFocus: true` papers over the gap for most pages — but it's still a "switch tabs, wait, see update" experience, not "open both tabs, see live".

For a procurement-reviewer / enterprise-buyer audience used to multi-tab workflows, this is a **demo risk** but not a v1-launch blocker. The approvals inbox — the surface most likely to be open in 2 tabs — is the one that's done right.

---

## Investigation evidence

- `web/components/skycot/QueryProvider.tsx:23` — global `refetchOnWindowFocus: true`
- `web/lib/api/websocket.ts:1-236` — `ManagedWebSocket` reusable client with auto-reconnect + polling fallback
- `web/lib/hooks/use-approvals-live.ts:126-161` — `applyEvent()` mutates TanStack cache from WS frames
- `web/lib/hooks/use-elevated-view-grant.ts:115-122` — sole `storage` event listener (multi-tab grant sync)
- `web/app/(app)/orchestration/page.tsx:24-41` — raw `useState + useEffect` fetch, NOT TanStack (the worst page for staleness)
- `web/app/(app)/workflows/page.tsx:118`, `web/app/(app)/skills/page.tsx:53` — search query in local state, not URL
- `web/app/(app)/audit/page.tsx:307-335` — model citizen: URL-as-truth, `router.replace`, `scroll: false`
- `web/app/(app)/audit/action/[actionId]/page.tsx:36-37,123` — doc claims a fallback the code doesn't implement
- `web/app/(app)/workers/[workerId]/FreezeButton.tsx:118-202` — Radix dialog, plain `useState` open/close, no history integration
