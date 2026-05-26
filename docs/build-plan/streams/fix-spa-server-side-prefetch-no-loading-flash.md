# fix-spa-server-side-prefetch-no-loading-flash

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-server-side-prefetch-no-loading-flash`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md` F12 + F13 + F14 + F15 + F16.

🟡 **P2 — `/team`, `/workers`, `/workers/<id>`, `/audit`, `/approvals`, `/skills`, `/verticals`, `/orchestration`, `/genesis` all server-render only a "Loading X…" string + empty skeleton. Camera records 100-300ms of empty state every time the operator clicks a nav item. Five separate findings, one root cause: no server-side prefetch.**

## Root cause

Every in-app page follows the `useQuery + Suspense + LoadingGrid` client-side pattern. Example `web/app/(app)/team/page.tsx:42-48`:
```tsx
export default function TeamPage() {
  return (
    <Suspense fallback={<LoadingGrid />}>
      <TeamContent />
    </Suspense>
  );
}
```
`<TeamContent>` calls `useTeam()` which fires the `/api/v1/team` fetch CLIENT-SIDE. Server-rendered HTML payload contains only the loading skeleton. SPA hydrates → fetch fires → 100-300ms later the team cards paint in.

Affected routes (verified by `curl -s :62927/<route> | grep -i loading`):
- `/team` — "Loading team…"
- `/workers/<id>` — "Loading worker…"
- `/audit` — "Loading audit…"
- `/approvals` — "Loading approvals…"
- `/skills` — "Loading skills…"
- `/verticals` — same pattern
- `/orchestration` — same pattern
- `/genesis` — same pattern

Per Next.js 16 server-component conventions, the fix is to add an `async` server component that calls `getQueryClient().prefetchQuery(...)` and passes the dehydrated state via `<HydrationBoundary state={dehydrate(queryClient)}>`. Client components hydrate with the data already in the cache → no fetch fires → no spinner.

## Findings this brief closes

- whole-site-mvp-claims-coverage F12 [P2] — `/team` client-only flash
- whole-site-mvp-claims-coverage F13 [P2] — `/workers` flash (server payload has 0 worker names)
- whole-site-mvp-claims-coverage F14 [P2] — `/audit`, `/approvals`, `/skills`, `/verticals`, `/orchestration` flash
- whole-site-mvp-claims-coverage F15 [P2] — `/workers/<id>` flash
- whole-site-mvp-claims-coverage F16 [P2] — `/genesis` barren server payload

## Fix recipe

1. **Add a `getQueryClient()` helper** at `web/lib/react-query-server.ts`:
   ```ts
   import { QueryClient } from "@tanstack/react-query";
   export function getQueryClient() {
     return new QueryClient({ defaultOptions: { queries: { staleTime: 60_000 } } });
   }
   ```

2. **Update each affected route's `page.tsx`** to be `async`:
   ```tsx
   import { dehydrate, HydrationBoundary } from "@tanstack/react-query";
   import { fetchTeamServer } from "@/lib/api/team-server";  // server-only variant
   import { getQueryClient } from "@/lib/react-query-server";

   export default async function TeamPage() {
     const queryClient = getQueryClient();
     await queryClient.prefetchQuery({
       queryKey: ["team"],
       queryFn: () => fetchTeamServer(/* forwarded cookie */),
     });
     return (
       <HydrationBoundary state={dehydrate(queryClient)}>
         <TeamContent />
       </HydrationBoundary>
     );
   }
   ```

3. **Server-only API client variants** — each `web/lib/api/<x>.ts` needs a `fetchXServer(cookieHeader)` that forwards the session cookie from `cookies()` (`next/headers`). Browser version unchanged.

4. **Sequence** — ship the highest-traffic 3 first (`/team`, `/audit`, `/approvals`) so the demo MP4 benefits early. The remaining 5 land in a follow-up wave.

5. **Tests** — `web/tests/page-ssr-prefetch.test.ts` curls each affected route on a fresh-cookie request and asserts the server HTML payload contains the data (not just "Loading…"). e.g. `curl -H 'Cookie: ...' :3000/team | grep Mariah` should succeed.

6. **Error handling** — if the server-side fetch fails (gateway down, 401, etc), render the existing loading skeleton (graceful degradation). Don't 500 the page.

## Acceptance criteria

- [ ] `curl :3000/team` HTML payload contains worker names (not just "Loading team…").
- [ ] Same for `/audit`, `/approvals`, `/skills`, `/workers/<id>`, `/verticals`, `/orchestration`, `/genesis`.
- [ ] Client hydration is a no-op for the prefetched query (network tab shows no fetch on initial paint).
- [ ] When server-side fetch fails, the page degrades to the client-side loading flow (no 500).
- [ ] SSR-prefetch test covers all 8 routes.
- [ ] No regression: each page still re-fetches on focus/interval per existing useQuery config.

## May-touch

- `web/lib/react-query-server.ts` (NEW)
- `web/lib/api/team-server.ts`, `audit-server.ts`, `approvals-server.ts`, `skills-server.ts`, `workers-server.ts`, `verticals-server.ts`, `orchestration-server.ts`, `genesis-server.ts` (NEW — one each, ~30 LOC apiece)
- `web/app/(app)/team/page.tsx`, `audit/page.tsx`, `approvals/page.tsx`, `skills/page.tsx`, `workers/[workerId]/page.tsx`, `verticals/page.tsx`, `orchestration/page.tsx`, `genesis/page.tsx` (convert to async)
- `web/tests/page-ssr-prefetch.test.ts` (NEW)

## Out of scope

- Per-worker / per-skill detail page deep-prefetch (only list pages in this brief)
- Replacing useQuery with server actions (separate architectural decision)
- Marketing pages (already SSR'd correctly)

## Effort

~400-600 LOC total across 8 server clients + 8 page conversions + 1 test. Sounds big but each is mechanical and small. ~6-10 hours. Sequence: ship `/team` first as proof, then batch the rest.
