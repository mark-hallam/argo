# Wave-7 Network Throttling + Offline Behavior Audit

**Date:** 2026-05-21
**Auditor:** Wave-7 audit agent #3
**Scope:** `~/code/argo/web/` — SKYCOT product SPA (Next.js 15 + React 19 + TanStack Query 5)
**Focus:** behavior on Slow 3G, mid-action network drop, stale-data-after-reconnect, offline mode, slow uploads

---

## Executive summary

The SKYCOT SPA has **partial** network-resilience scaffolding but several load-bearing surfaces will degrade poorly under throttled or flaky networks. The good news: TanStack Query is wired with sensible defaults (2 retries on 5xx/network, no retry on 4xx, `refetchOnWindowFocus`), a real WebSocket reconnect-with-exponential-backoff lives in `web/lib/api/websocket.ts`, and a service worker + `/offline` shell shipped recently. The bad news: the base API client has **zero timeout**, **zero AbortController** wiring, **zero `navigator.onLine` listeners**, and the LiveIndicator was deliberately *hidden* for the demo recording so the operator gets no UI signal when the WebSocket falls into polling/offline state. Two of the most demoed pages (Cohorts, Workers landing) have skeleton-on-fetch but **no error state at all** — if `useCohorts()` fails they spin forever; if `useTeam()` errors the workers page sits on its loader. The service worker is registered **only in production** (`process.env.NODE_ENV !== "production"` gate), so `next dev` exhibits none of the offline behavior procurement testers would see in a deployed build.

---

## 1. Slow 3G first paint — page-by-page

Test condition: Chrome DevTools → Network → "Slow 3G" preset (50 KB/s, 2 s RTT). Tester opens `/approvals`, `/audit`, `/team`, `/cohorts`, `/skills`, `/workflows`, `/orchestration`, `/retro`, `/settings`, `/workers/<id>/overview`, `/hire`, `/auth/signup`.

| Surface | Skeleton on fetch? | Error on fetch fail? | Notes |
|---|---|---|---|
| `/` (Approvals inbox) | YES — `<LoadingState>` with `Loader2` spinner + "Loading approvals…" | YES — `<ErrorState>` with retry button (`approvalsQuery.refetch()`) | Best-in-class on the demo path. |
| `/audit` | YES — `<LoadingState>` spinner + "Loading audit timeline…" | YES — `<ErrorState>` with retry | Plus `error.tsx` boundary above. |
| `/audit/action/<id>` | YES — `<DetailFallback>` spinner | YES — `<DetailErrorState>` with retry | Distinct not-found vs. network-error states. |
| `/team` | YES — `<LoadingGrid>` 6 skeleton cards (`TeammateCardSkeleton`) | YES — `<TeamErrorState>` "Couldn't load your team" + retry | Reference implementation. |
| `/workers` | YES — spinner only | **NO** — falls through to `<EmptyState>` "No teammates yet" | If `useTeam()` errors the page lies — says "no teammates" when really the gateway is down. |
| `/cohorts` | YES — `<LoadingGrid>` 6 pulse cards | **NO** — falls through to `<EmptyState>` "No cohorts" | Same lie as workers: error-state silently rendered as empty. |
| `/skills` | YES — but text-only "Loading skills from the marketplace…" (no skeleton cards) | YES — `<state-panel-error>` with `error.message` | Inconsistent loading register (no skeleton). |
| `/orchestration` | YES — `<LoadingState>` spinner only | YES — `<ErrorState>` red banner — **no retry button** | Operator must hard-reload to recover. |
| `/retro` | YES — "Loading retros…" | YES — `retros.isError` branch | Has loading + error states. |
| `/workflows` | N/A — static SEED data, no fetch | N/A | Will paint instantly on 3G — but the data is fake. |
| `/settings` (landing) | N/A — static tiles, no fetch | N/A | Will paint instantly. |
| `/workers/<id>/overview` | YES — `<WhatSheDoesCardSkeleton>` + `<DutiesCardSkeleton>` + `<KpiCardSkeleton>` + `<RecentActivityCardSkeleton>` | YES — `overviewQuery.isError` branch | Skeletons are the gold standard. |
| `/billing` | "Loading…" text only | YES — "Failed to load usage" | No skeleton; pure text loader. |
| `/support/inbox` | YES — `all.isPending` branch | YES — `all.isError` branch | Has both. |
| `/settings/api-keys` | YES — `isLoading` prop | YES — `submitError` surfaced | Has both. |
| `/setup` (wizard) | "Loading setup" text-only label | YES — `<setup-error>` paragraph | Uses raw `apiGet` / `apiPost` (no React Query), no retry. |
| `/auth/signup` | YES — `state.kind === "submitting"` pending state | YES — `state.kind === "error"` branch | Magic-link send. |
| `/auth/verify` | YES — `<VerifySpinner>` Suspense fallback | YES — error rendered | Has both. |
| `/hire` (Step 2 Listen) | YES — `useGenesisQuestions` returns `status: "loading"` | YES — `status: "error"` → swaps to stub questions | The only hook in the app with **bounded retry** (3 attempts at 1s/2s/4s backoff). |
| Landing / Compare / Pricing / Customers / Privacy / Terms / Docs | N/A — static, prerendered | N/A | Slow 3G shows them after one round-trip. |

**Tally:** of ~18 dynamic pages on the demo path, **2 pages have skeleton + no real error state** (Workers, Cohorts — both render an empty state on error, which is misleading). **0 pages handle the "network hangs forever" case** (no `AbortController`, no fetch timeout — see §6).

---

## 2. Mid-action network drop (Approve-click in flight)

The approvals inbox uses TanStack Query mutations with **optimistic updates**:

```ts
// web/lib/hooks/use-approvals.ts:48
return useMutation({
  mutationFn: submitDecision,
  onMutate: async ({ approvalId, decision }) => {
    await qc.cancelQueries({ queryKey: APPROVALS_KEY });
    const prev = qc.getQueryData<ApprovalsResult>(APPROVALS_KEY);
    if (prev) {
      qc.setQueryData<ApprovalsResult>(APPROVALS_KEY, {
        ...prev,
        items: prev.items.map((i) => (i.id === approvalId ? { ...i, decision } : i)),
      });
    }
    return { prev };
  },
  onError: (_err, _vars, ctx) => {
    if (ctx?.prev) qc.setQueryData(APPROVALS_KEY, ctx.prev);
  },
});
```

**What works:**
- Card flips immediately (optimistic).
- On error the cache is rolled back to `prev` (the card flips back).
- `mutations: { retry: false }` is set globally in `QueryProvider.tsx` — mutations don't replay automatically (correct: avoid double-spend on a `submitDecision`).

**What's broken:**
- **The user is never told the action failed.** `onError` rolls back the cache but doesn't fire a toast. The card silently reverts to "pending" with zero explanation. The operator sees a card that "didn't take" and assumes they mis-clicked.
- **No retry button after a failed decision.** The operator has to click Approve again with no signal that the last click was lost.
- The `onError` callback in `useSubmitDecision` receives `_err` but discards it.
- `useBatchSubmitDecisions` (Approve-all) does surface failures via `<BatchUiState>.failedIds` and the day-section renders them — so the **batch** path is better instrumented than the **single-card** path. Inconsistent.

**Verdict for mid-action drop on Approve-click:** card flips back, no toast, no error, no retry. Operator is confused but eventually re-clicks.

---

## 3. Stale data after reconnect (page open for an hour, network drops + recovers)

TanStack Query is wired with `refetchOnWindowFocus: true` in `QueryProvider.tsx:23` and the global retry policy retries 2× on transient errors. **However:**

- There is **no `refetchOnReconnect`** setting — TanStack Query defaults this to `true` (good), but there's no `navigator.onLine` listener anywhere in the codebase verified via `rg "navigator\.onLine|window\.addEventListener\(.online" web/` → **0 hits**.
- TanStack Query's built-in `onlineManager` does subscribe to the browser's `online` event by default, so refetch-on-reconnect WILL fire when the browser detects reconnection. But on macOS/iOS the `online` event is unreliable for "network came back up" — it fires only on the OS-level link change, not on application-layer recovery.
- **Stale data is silently used:** `staleTime: 30_000` means data older than 30 s is marked stale but still rendered. After an hour, the page shows hour-old approvals until something triggers a refetch (window focus, mutation invalidation, manual refetch).
- The WebSocket layer (`ManagedWebSocket` in `web/lib/api/websocket.ts`) DOES auto-reconnect with exponential backoff (1 s, 2 s, 4 s, … up to 30 s with ±25% jitter) and falls back to polling after 3 failures inside a 60 s window. So the live-update channel will recover, but the underlying HTTP query cache won't necessarily refetch.

**Verdict:** stale data is shown silently for up to 30 s + however long until window focus or WS event fires a cache invalidation. There's no "Reconnecting…" or "Last updated 12 minutes ago" surface that procurement would expect.

---

## 4. Offline mode

- **Service worker:** `web/public/sw.js` exists and implements: network-first for `/api/*` (no cache — fresh auth/data wins), cache-first for static assets, and a navigation fallback to `/offline`. Cache versioned via `CACHE_VERSION = "skycot-pwa-v1"`. Properly invalidates old caches on `activate`.
- **Offline shell page:** `web/app/offline/page.tsx` is a polished branded page (Audiowide hero + purple/pink gradient blob + "Try again" reload button).
- **Registration gate:** `web/components/pwa/ServiceWorkerRegister.tsx:15`:
  ```ts
  if (process.env.NODE_ENV !== "production") return;
  ```
  **Service worker DOES NOT register in dev mode.** Procurement testers running `next dev` or hitting a non-prod preview see NO offline behavior. The offline shell never appears in dev because the SW that would serve it isn't installed.
- **What's cached:** the offline page itself, `manifest.webmanifest`, favicon, brand SVG wordmark, two icon files. **No HTML routes are precached.** So if the user has only visited `/approvals` and goes offline, navigating to `/team` will get the offline shell, not the team page from cache.
- **No `navigator.onLine` UI signal:** the only offline-aware surface is the `LiveIndicator` (which is hidden for the demo, see §1).

**Verdict:** offline mode works in production for navigations only, with a polished branded shell. But it's invisible in dev, the operator gets no warning before going offline, and only the single page they're currently on stays usable (in-flight queries will fail and surface as errors per §2).

---

## 5. Slow uploads

- **`grep -r "FormData|enctype|XMLHttpRequest|upload" web/` → 0 hits in product surfaces.** Confirmed: no file upload anywhere in the v1 SPA.
- The hire flow (`/hire/step/[2..5]`) is all text-input + button-click; no resume/CV upload.
- The setup wizard (`/setup`) is text-input only.
- The evidence bundle exporter (`web/components/skycot/audit/EvidenceBundleExporter.tsx`) is a **download** flow (gateway emits a `.zip`), not an upload, and includes no progress indicator.

**Verdict:** no slow-upload scenarios exist in the demo path. The audit evidence bundle export could itself be slow over 3G (a multi-MB zip), but the UI just shows a button and a toast — no download-progress bar.

---

## 6. Top 5 network-degradation issues

### Issue #1: API client has zero timeout, zero AbortController
`web/lib/api/client.ts:75` calls `await fetch(url, {…})` with no timeout option. A hanging request (slow 3G + gateway briefly unreachable) will spin **forever** until the user gives up and refreshes. There's also no signal-on-unmount wiring — navigating away from a slow page doesn't abort the in-flight fetch.

**Fix:** add `AbortSignal.timeout(20_000)` to every request, surface the timeout as a typed `TimeoutError` distinct from generic network errors. The only fetch-with-`signal` callsite is `web/lib/api/connectors.ts:33` (one of ~40 fetch sites).

### Issue #2: Workers + Cohorts pages render "empty state" on fetch error
Both `/workers` (`web/app/(app)/workers/page.tsx:23`) and `/cohorts` (`web/app/(app)/cohorts/page.tsx:58`) skip the `isError` branch entirely. If the gateway returns 5xx the operator sees "No teammates yet — hire your first AI worker" / "No cohorts — group similar tasks…" with full CTAs to take action that won't work. This is a trust-destroying lie that procurement WILL catch.

**Fix:** add `query.isError → ErrorState` branch with retry button (the team page at `web/app/(app)/team/page.tsx:171` is the reference implementation).

### Issue #3: Optimistic Approve-click swallows mid-flight network errors silently
`useSubmitDecision` rolls back the optimistic update on error but **does not fire a toast**. The operator clicks Approve, the card flips, the network dies, the card flips back — no explanation. The day-of-record approve-all flow (batch) DOES surface failures; the single-click path does not. Inconsistent and unsafe — operators may not notice and assume a decision was recorded when it wasn't.

**Fix:** in `web/lib/hooks/use-approvals.ts:60` `onError` callback, fire `toastError("Couldn't submit decision — try again")` with the action's text in the description, plus a `<button>` retry.

### Issue #4: LiveIndicator hidden in degraded states for the demo
`web/components/skycot/LiveIndicator.tsx:35`: `if (status !== "live") return null;`. The deliberate code comment says "leak[s] 'this is unreliable' on camera during the v1.0 demo walkthrough." The result: operators in the wild have no signal whatsoever that their WebSocket has fallen into polling mode or is offline. A procurement tester running Slow 3G + intermittent drop will see no indication their inbox is going stale.

**Fix:** gate the hide-behavior behind `process.env.NEXT_PUBLIC_DEMO_RECORDING === "1"` rather than always-on. Show the connecting/polling/offline state in real environments.

### Issue #5: Service worker registers only in production
`web/components/pwa/ServiceWorkerRegister.tsx:15` short-circuits when `NODE_ENV !== "production"`. Procurement testers running a deployed-preview-build (often built as `next build` then `next start` locally with `NODE_ENV=development` via misconfiguration) will get zero offline behavior despite the manifest claiming PWA-installable. Also: no E2E test of the offline shell exists because Playwright's `next dev` harness never sees the SW.

**Fix:** allow SW registration in non-prod builds when `NEXT_PUBLIC_FORCE_SW=1`, and add a Playwright test that hits `next build && next start` then asserts `/offline` renders with the network disabled.

---

## Additional observations (lower priority)

- **`/orchestration` ErrorState has no retry button** — the only fix is a hard reload (`web/app/(app)/orchestration/page.tsx:167`).
- **`/skills` and `/billing` use text-only loaders, not skeletons** — inconsistent with the rest of the app.
- **Audit evidence bundle download has no progress UI** — slow download on 3G is opaque.
- **`/setup` wizard uses raw `apiGet`/`apiPost` without React Query** — bypasses the global retry policy; one failed POST means the wizard step is dead until manual refresh.
- **TanStack Query's `refetchOnReconnect` defaults to `true` but is not explicitly set** in `QueryProvider.tsx`. If a future change disables `onlineManager`, this resilience evaporates silently.
- **No `navigator.onLine` event listener at app level** — there's no global "You're offline — actions paused" banner. Some apps (Linear, Notion) render a top bar when offline detected; SKYCOT does not.
- **CSP `connect-src` only allows `'self' <gateway>` + `api.skycot.com`** — verified clean. No surprise third-party fetch that could leak under degraded networks.

---

## Files referenced

- `~/code/argo/web/lib/api/client.ts` — base fetch wrapper (no timeout, no abort)
- `~/code/argo/web/lib/api/websocket.ts` — ManagedWebSocket with proper backoff
- `~/code/argo/web/components/skycot/QueryProvider.tsx` — global retry policy
- `~/code/argo/web/components/skycot/LiveIndicator.tsx:35` — hidden-in-degraded-state behavior
- `~/code/argo/web/components/pwa/ServiceWorkerRegister.tsx:15` — prod-only SW gate
- `~/code/argo/web/public/sw.js` — service worker (network-first API, cache-first static, offline-shell nav fallback)
- `~/code/argo/web/app/offline/page.tsx` — branded offline shell
- `~/code/argo/web/app/(app)/workers/page.tsx:23` — missing error state
- `~/code/argo/web/app/(app)/cohorts/page.tsx:58` — missing error state
- `~/code/argo/web/lib/hooks/use-approvals.ts:60` — silent optimistic-rollback
- `~/code/argo/web/lib/hire/use-genesis-questions.ts:33` — the only bounded-retry hook in the app
