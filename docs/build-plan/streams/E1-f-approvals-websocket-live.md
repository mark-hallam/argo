# Stream E1-f — Approvals inbox: WebSocket live updates

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E1-a-approvals-gateway-wireup MERGED.
**Authored against**: origin/main `3f4bd5f2` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E1-f-approvals-websocket-live`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E1. **Last brief in slice E1** — after merge, E1 closes.

## Why

Today the approvals inbox refreshes via TanStack Query's
`refetchOnWindowFocus` + a 30-second stale-time. That's fine
for low-volume use but produces a noticeable "stale-feeling"
gap for active operators — a new approval lands and they
don't see it until they tab away and back.

The gateway already exposes a WebSocket at `/inbox/ws`
(see [`crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`](../../../crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs)
line 29 — `GET /inbox/ws`). Today that WS streams reload-
signal events to the HTML inbox. This brief subscribes from
the SPA so new approvals arrive in real time, with the
optimistic-update + cache machinery from TanStack Query
absorbing the new items into the existing list without a
full refetch.

## Scope

- A `useApprovalsLive()` hook that opens a WebSocket
  connection to `${NEXT_PUBLIC_GATEWAY_URL}/inbox/ws` (with
  bearer-token auth), parses incoming events, and updates
  the TanStack Query cache.
- Connection lifecycle: open on mount, close on unmount,
  reconnect with exponential backoff on close + heartbeat
  every 30s.
- Event types handled:
  - `approval.created` — append to the cache list.
  - `approval.decided` — flip the matching item's
    `decision` field; if cached card is mid-undo on the
    user's side, prefer local state until undo window
    elapses.
  - `approval.undone` — flip back to `decision: null`.
  - `approval.expired` — remove the item from cache.
- Visual feedback: a tiny "● live" indicator next to the
  page subtitle showing connection state.
- Tab focus / blur handling: connection stays open in
  background; the user sees new approvals when they tab
  back (but the cache stayed warm).
- Graceful degrade: if WS fails to connect or closes 3
  times in 60s, fall back to polling. User sees
  `● polling (offline)` indicator. Polling cadence: 60s.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/lib/hooks/use-approvals.ts`,
  `web/app/(app)/page.tsx`

## Files you may touch

- `web/lib/api/websocket.ts` — NEW. Reusable WS client
  wrapper. Auto-reconnect, heartbeat, exponential backoff.
  Generic `<T>` for event payload type. ≤ 250 LOC.
- `web/lib/api/websocket.test.ts` — NEW. ≥ 6 unit tests
  using a stubbed `WebSocket` global. Cover: open + close,
  reconnect after drop, heartbeat fires, exponential
  backoff caps at 30s, error → fallback path triggers.
  ≤ 250 LOC.
- `web/lib/hooks/use-approvals-live.ts` — NEW. Opens the
  WS, applies events to the TanStack Query cache, returns
  `{ status: "live" | "polling" | "offline", lastEventAt:
  Date | null }`. ≤ 200 LOC.
- `web/lib/hooks/use-approvals.ts` — narrowly scoped: add
  one line that invokes `useApprovalsLive()` from
  `useApprovals()`. Existing return shape is preserved.
  ≤ 200 LOC.
- `web/components/skycot/LiveIndicator.tsx` — NEW. Tiny
  StatusPill-style indicator with a pulsing dot. ≤ 80 LOC.
- `web/app/(app)/page.tsx` — narrowly scoped: render
  `<LiveIndicator>` next to the page subtitle. ≤ 400 LOC
  after changes.

## Files you must NOT touch

- `web/components/skycot/ApprovalCard.tsx` — locked.
- `web/lib/api/approvals.ts` — the fetch path is owned
  by E1-a. WS lives parallel.
- `crates/**` — `/inbox/ws` already exists. Backend
  changes are out of scope.
- Sibling E1 deliverables (E1-b through E1-e).

## Acceptance criteria

- [ ] On page mount, the SPA opens a WebSocket to
      `${gateway}/inbox/ws` with the bearer token in the
      `Authorization` header (or `?token=` query param if
      browser-WS doesn't support headers).
- [ ] An `approval.created` event appends a new card to
      the list with no full refetch.
- [ ] An `approval.decided` event (from a different
      session — e.g. operator approved on Slack) flips
      the card's decision in place.
- [ ] An `approval.undone` event flips it back.
- [ ] An `approval.expired` event removes the card.
- [ ] Connection drops → reconnect with backoff:
      1s, 2s, 4s, 8s, 16s, 30s (cap).
- [ ] Heartbeat ping every 30s. If no pong in 10s,
      treat as dropped + reconnect.
- [ ] After 3 failed connects in 60s, fall back to 60s
      polling. Indicator flips to `● polling (offline)`.
- [ ] Tab blur does NOT close the WS. Tab focus
      triggers a single `cache.invalidateQueries` to
      catch up on anything missed during a possible
      throttled-tab period.
- [ ] LiveIndicator strings:
      - `● live` (tone success) when WS connected.
      - `● polling (offline)` (tone warning) when fallen
        back.
      - `● connecting…` (tone muted) on initial connect
        + on reconnect attempts.
- [ ] 6 unit tests on the WS wrapper pass.
- [ ] An e2e test (Playwright) using a mock-WS server
      verifies an event arrives → cache updates → card
      appears. ≤ 100 LOC.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E1-f-approvals-websocket-live`
- PR title: `web(approvals): WebSocket live updates with polling fallback (E1-f)`

## Notes for the implementer

- **Browser WebSocket doesn't support custom headers.**
  For bearer-token auth, use the `?token=<token>` query
  param. The gateway accepts both — confirm by reading
  `routes_channels_webinbox.rs`. If gateway only accepts
  header, a sibling backend brief adds query-param
  support. For v1, the query-param path is documented in
  the wireframe.

- **Event payload shape.** The gateway today sends a
  generic "reload" signal (per the JS in
  `routes_channels_webinbox.rs:44`). For SPA use, the
  gateway needs to emit structured events:
  `{ type: "approval.created", payload: ApprovalItem }`,
  etc. **This requires a sibling backend brief**
  (`mission-gateway-inbox-ws-structured-events`). Until
  that brief lands, this client brief subscribes but
  treats every event as `cache.invalidate` (cheap full
  refetch). When the structured-event brief lands, the
  client switches to surgical cache updates without
  needing a new release.

- **Reconnect backoff.** Standard exponential:
  ```
  delay = min(1000 * 2^attempt, 30_000)
  attempt resets to 0 on successful connect
  ```
  Add jitter ±25% to avoid thundering-herd reconnects
  if the gateway recovers and many clients reconnect
  simultaneously.

- **Heartbeat.** Send a `ping` text frame every 30s.
  Expect a `pong` within 10s. If no pong, force-close
  and reconnect. Use the WS native ping/pong if the
  gateway supports it; otherwise application-level
  ping/pong text frames.

- **Cache update strategy.** With TanStack Query:
  ```ts
  qc.setQueryData<ApprovalItem[]>(["approvals"], (prev) => {
    if (event.type === "approval.created") {
      // append, dedupe by id
      return [...prev.filter(i => i.id !== event.payload.id),
              event.payload];
    }
    // ... other event types
  });
  ```
  Tested via the unit test against a stubbed cache.

- **Status precedence.** If both WS and polling are
  attempted, the WS wins when connected; polling is
  the fallback ONLY when WS fails. The indicator shows
  whichever is actually delivering data.

- **Don't bypass the optimistic update.** When the user
  approves a card, the local optimistic update flips
  it immediately. Then `mutationFn` POSTs to gateway.
  Then `approval.decided` event arrives back over WS.
  The cache already shows decided, so the WS event is
  a no-op. Don't write code that flips the card a
  second time (visible jank).

- **What this brief does NOT do.**
  - Doesn't change the gateway's WS event format. That's
    a backend brief.
  - Doesn't add WS for other surfaces (audit timeline,
    team page). Each gets its own brief if/when needed.
  - Doesn't add SSE as a fallback. WS only; polling is
    the fallback layer.
  - Doesn't add user-controllable WS preferences ("turn
    off live updates"). The indicator surfaces state
    but the connection is automatic.

## Open questions

`(needs operator judgment at checkpoint)`

- Does the gateway's `/inbox/ws` accept `?token=` for
  auth, or only header-based? If only header, sibling
  backend brief required.
- Are the proposed event types (`approval.created`,
  `.decided`, `.undone`, `.expired`) the right
  vocabulary, or should they match an existing event
  taxonomy (e.g. CloudEvents)?
- Should the LiveIndicator be visible to all users, or
  only operators? Default: visible to all in dev,
  hidden in production unless `?debug=1`. Confirm.
