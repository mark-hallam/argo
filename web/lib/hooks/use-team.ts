"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchTeam, type FetchTeamResult } from "@/lib/api/team";

const TEAM_KEY = ["team"] as const;

/**
 * The team roster. Server-cached + revalidated on window focus.
 *
 * `staleTime: 0` + `refetchOnMount: "always"` defeat a SSR/client
 * hydration race observed in dev: when the server-rendered HTML
 * hydrated against a stale pathname, React Query would retain its
 * initial pending state across the boundary without re-running
 * `queryFn`, leaving /team stuck on a perpetual skeleton. Forcing
 * an always-refetch on mount makes the query authoritative on the
 * client and the loading chip flips within 2s of paint.
 *
 * `retry: 1` caps the transient-failure spin: `fetchTeam` already
 * has rich fallback paths (demo SAMPLE_TEAM, unreachable empty),
 * so one quick retry is enough to recover from a flapping gateway
 * without burning seconds on exponential backoff.
 *
 * IMPORTANT (Scene-1 regression, fixed 2026-06-01): these options only
 * take effect once the observer actually mounts. On `/team` the observer
 * was being gated by the page's `useSearchParams()` Suspense boundary,
 * which sits inside `RouteTransition`'s `<AnimatePresence>` (see
 * `lib/motion/primitives.tsx`): when that boundary suspended on hydration
 * it never committed on the client, so `queryFn` never fired and the page
 * showed a perpetual loading skeleton with NO `/api/v1/workers` request.
 * The fix lives in `web/app/(app)/team/page.tsx`: `useTeam()` is now
 * called ABOVE that boundary. Keep this fetch out of any
 * `useSearchParams()`-suspending subtree. Pinned by
 * `web/app/(app)/team/team-fetch-fires.test.tsx`.
 */
export function useTeam() {
  return useQuery<FetchTeamResult>({
    queryKey: TEAM_KEY,
    queryFn: fetchTeam,
    staleTime: 0,
    refetchOnMount: "always",
    retry: 1,
  });
}
