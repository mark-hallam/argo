"use client";

import { useSyncExternalStore } from "react";
import {
  AUDIT_RANGE_PRESETS,
  DEFAULT_AUDIT_RANGE,
  type AuditFilters,
  type AuditRangePreset,
} from "@/lib/types/audit";

/**
 * Non-suspending read of the two URL params the audit *feed fetch*
 * depends on (`range` + `q`).
 *
 * Why this exists (the Scene-6 `/audit` perpetual-skeleton this closes,
 * sibling of the `/team` fix in `web/app/(app)/team/page.tsx`):
 * `useAuditFeed()` MUST mount ABOVE the `useSearchParams()` Suspense
 * boundary — that boundary sits inside `RouteTransition`'s
 * `<AnimatePresence>` (`lib/motion/primitives.tsx`) and, when it suspends
 * during client hydration, never commits, so the React Query observer
 * never subscribes and `queryFn` never fires. `/audit` then shows
 * "Loading audit timeline…" FOREVER and the browser issues NO
 * `/api/v1/audit` request at all (verified in the live demo-live SPA:
 * 14s on the page, zero audit requests).
 *
 * But the feed is *keyed* on the URL (`range`/`q`), so the outer
 * component can't just hard-code them the way `/team` could. Calling
 * `useSearchParams()` in the outer component would re-introduce the
 * suspend. Instead we read `window.location.search` through
 * `useSyncExternalStore`, subscribing to:
 *   - `popstate` — browser back/forward, and
 *   - a custom `audit:url` event the in-boundary filter writers dispatch
 *     after each soft `router.replace` (App-Router soft navigations do
 *     NOT emit `popstate`).
 * This never suspends, so the feed fetch fires on mount and re-keys when
 * the range/search change. The richer multi-select dimension filters stay
 * inside the boundary and are applied client-side over the fetched rows.
 *
 * Pinned by `web/app/(app)/audit/audit-fetch-fires.test.tsx`.
 */
export const AUDIT_URL_EVENT = "audit:url" as const;

function getSearchSnapshot(): string {
  if (typeof window === "undefined") return "";
  return window.location.search;
}

function subscribe(onStoreChange: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  window.addEventListener("popstate", onStoreChange);
  window.addEventListener(AUDIT_URL_EVENT, onStoreChange);
  return () => {
    window.removeEventListener("popstate", onStoreChange);
    window.removeEventListener(AUDIT_URL_EVENT, onStoreChange);
  };
}

// SSR snapshot is a constant empty string so `useSyncExternalStore`
// hydrates deterministically; the client snapshot picks up the real
// query string on first commit. The feed then refetches if the URL
// carried a non-default range/q (re-key via the query cache).
function getServerSnapshot(): string {
  return "";
}

function parseRangeFrom(search: string): AuditRangePreset {
  const raw = new URLSearchParams(search).get("range") ?? "";
  return (AUDIT_RANGE_PRESETS as readonly string[]).includes(raw)
    ? (raw as AuditRangePreset)
    : DEFAULT_AUDIT_RANGE;
}

/**
 * The `{ range, q }` the feed fetch keys on, read without suspending.
 * Re-renders the caller on `popstate` + `audit:url` (see module doc).
 */
export function useAuditFeedFilters(): AuditFilters {
  const search = useSyncExternalStore(
    subscribe,
    getSearchSnapshot,
    getServerSnapshot,
  );
  const range = parseRangeFrom(search);
  const q = new URLSearchParams(search).get("q") ?? undefined;
  return { range, q };
}

/**
 * Notify the non-suspending feed-filter store that the URL changed via a
 * soft `router.replace`. Called by the in-boundary filter writers right
 * after they push the new query string, so the hoisted feed fetch re-keys.
 */
export function notifyAuditUrlChange(): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new Event(AUDIT_URL_EVENT));
}
