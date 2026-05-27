# fix-useteam-query-stuck-loading-spa-fallback-never-resolves

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-useteam-query-stuck-loading-spa-fallback-never-resolves`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-1-and-2-claim-1-and-2.md` F1 [P0].

🔴 **P0 — `/workers` redirects to `/team`. `/team` is stuck on an infinite loading skeleton — 12 skeleton tiles render and never resolve. `data-testid=team-data-source-chip` stays `"loading"` for 60s+. The 5-card SAMPLE_TEAM deck (Mariah/Peta/Priya/Jamal/Nori) never renders. Scene 1 step 1 is unrecordable from the nav-bar "Workers" link.**

## Root cause

`web/lib/api/team.ts:55-101` (`fetchTeam`) calls `apiGet("/api/v1/workers")`, which the gateway 404s (the collection endpoint isn't implemented — see `fix-gateway-workers-collection-route-alias-404-add-canonical`). The catch block then checks `isDemoMode()` and SHOULD return `SAMPLE_TEAM` synchronously. But in this build the `useTeam` query (`web/lib/hooks/use-team.ts`) stays `pending` forever — likely an unhandled rejection in the catch chain that React Query interprets as "still pending".

Hypothesised interaction with F5 (the SSR/client hydration mismatch on AppNav data-active drift): server-rendered HTML may be hydrating against a stale pathname, causing React Query to retain its initial loading state across the hydration boundary without re-running the queryFn.

Compounding effect from F2 in the same audit: the auth-bypass env-var name mismatch (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` in env vs `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` in code) means the fast-path mock short-circuit at the top of `fetchTeam` is dead, so every call goes through the gateway 404 + catch-block fallback. Fixing the env-var rename will short-circuit before the 404 round-trip and likely (but not certainly) resolve this finding too.

Live evidence:
- `curl http://127.0.0.1:7400/api/v1/workers` → 404
- `curl http://127.0.0.1:7400/api/v1/team` → 200 with 1 worker "Maya"
- Browser `document.querySelector('[data-testid=team-data-source-chip]')?.dataset.source` → `"loading"` after 30s wait
- 6-12 `[data-testid=teammate-loading-*]` skeleton tiles in DOM, 0 cards

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F1 [P0] — `/workers` → `/team` infinite skeleton; 5-card deck never appears

## Fix recipe

1. **First, claim and merge `fix-claude-launch-json-env-var-naming-mismatch` + `fix-web-mock-bypass-env-var-rename`.** If env-var rename ships and the fast-path short-circuit activates, `fetchTeam` returns `SAMPLE_TEAM` instantly without ever hitting `apiGet`. Verify by reloading `/team` and inspecting the chip.
2. **If the stuck-loading persists after env-var fix**, investigate `web/lib/hooks/use-team.ts` for query-key staleness or `enabled: false` race. Add explicit `staleTime: 0` + `refetchOnMount: 'always'` to force re-fetch on hydration.
3. **Make the catch-block fallback resilient.** In `web/lib/api/team.ts`, wrap the `isDemoMode()` branch so any thrown exception from `isMissingOrUnreachable` doesn't propagate as an unhandled rejection — return `{ workers: SAMPLE_TEAM, source: "mock-offline" }` synchronously regardless.
4. **Add a regression vitest** under `web/lib/api/team.test.ts` (or hooks-side) that mocks `apiGet` to throw a 404 and asserts `useTeam()` resolves to `SAMPLE_TEAM` within 1s.
5. **Tighten the skeleton timeout.** If the query is still loading after 5s, render the SAMPLE_TEAM fallback as a soft surrender state instead of perpetual skeleton — better UX even when the query is genuinely slow.

## Acceptance criteria

- [ ] `/team` cold-load in demo mode renders the 5-card SAMPLE_TEAM deck within 2s of paint.
- [ ] `data-testid=team-data-source-chip` flips from `loading` → `mock-offline` (or `gateway` if gateway has data) within 2s.
- [ ] Vitest pin: mock `apiGet` throws 404 → `useTeam()` resolves to `{ workers: SAMPLE_TEAM }` within 1s of mount.
- [ ] No infinite skeleton state visible in dev or production builds.
- [ ] Scene 1 step 1 ("nav-bar Workers → 5-card deck") is recordable cold.

## May-touch

- `web/lib/api/team.ts` (catch-block resilience)
- `web/lib/hooks/use-team.ts` (query options)
- `web/app/(app)/team/page.tsx` (skeleton-timeout surrender state)
- `web/lib/api/team.test.ts` (new vitest)

## Out of scope

- Env-var rename (covered by `fix-claude-launch-json-env-var-naming-mismatch` + `fix-web-mock-bypass-env-var-rename`)
- Gateway `/api/v1/workers` 404 (covered by `fix-gateway-workers-collection-route-alias-404-add-canonical`)
- SSR/client hydration mismatch on global nav (covered by `fix-app-shell-nav-hydration-mismatch-data-active-drift`)

## Effort

S-M — ~50-100 LOC. ~1-2 hours. P0 because Scene 1's first beat is unrecordable until this resolves.
