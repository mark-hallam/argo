# fix-spa-sessionprovider-disable-in-demo-bypass

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-sessionprovider-disable-in-demo-bypass`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-1-and-2-claim-1-and-2.md` F2 [P0].

🔴 **P0 DEMO-BLOCKER — NextAuth `<SessionProvider>` spams `ClientFetchError` on every navigation when running in `NEXT_PUBLIC_ARGO_DEMO_MODE=1` (the demo runtime). Continuous browser-console error spew during the e2e walk; the preview renderer became unresponsive after ~60 navigations. If the operator opens devtools during recording — or screen-shares the console pane — this is the first thing visible.**

## Root cause

`<SessionProvider>` is mounted globally at `web/components/skycot/AuthProvider.tsx:7` and polls `/api/auth/session` on a loop. In demo-bypass mode there is no minted `__Host-session` cookie, so every poll returns a 5xx / config-error and surfaces as a `ClientFetchError` in the browser console:

```
ClientFetchError: There was a problem with the server configuration. Check the server logs for more information.
  at SessionProvider.useEffect [as _getSession]
```

The polling fires on every route change. Worse: the audit suggests the renderer becomes janky after sustained polling — a real performance side-effect, not just console noise.

For the demo recording this is dressing-camera-might-see (the recording is unlikely to have devtools open by default — operator dismisses banner pre-clip-1 per `feedback_no_theater_in_demo`), but the performance side-effect AND the risk of any screen-share with devtools makes this P0. Also it's a 5-line fix.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F2 [P0] — NextAuth `SessionProvider` spams `ClientFetchError` on every navigation in demo-bypass mode

## Fix recipe

1. **Guard `<SessionProvider>` in `web/components/skycot/AuthProvider.tsx`**:
   ```tsx
   "use client";
   import { SessionProvider } from "next-auth/react";

   const DEMO_BYPASS =
     process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1" ||
     process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1";

   export function AuthProvider({ children }: { children: React.ReactNode }) {
     if (DEMO_BYPASS) return <>{children}</>;
     return <SessionProvider>{children}</SessionProvider>;
   }
   ```
   Renders children directly under demo-bypass; no polling, no fetch errors.

2. **Verify any downstream consumer of `useSession()`** — if any component hard-requires `useSession()` to return a session, wrap the consumer in a `try { useSession() } catch { return null }` or have it short-circuit when demo-bypass is on. Most likely the consumers already handle the un-authed shape (because demo-bypass works by skipping the SessionProvider entirely in some pages).

3. **Cross-check `web/lib/api/*.ts` mock-bypass branches** — these read the same env-var that `fix-web-mock-bypass-env-var-rename` standardized. Make sure the env-var being read here matches the one in those files (per `fix-web-mock-bypass-env-var-rename` the canonical name is `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`; the demo runtime ALSO sets `NEXT_PUBLIC_ARGO_DEMO_MODE=1` which is what middleware checks). Both should be respected here.

4. **Smoke-test** by booting `pnpm --dir web dev` with `NEXT_PUBLIC_ARGO_DEMO_MODE=1`, navigating to `/`, `/team`, `/audit`, `/approvals`, `/skills`, `/workflows`. Browser console must be **clean** of `ClientFetchError` after 30s of navigation.

5. **Add a Playwright spec** at `web/playwright/auth/no-session-fetch-error-in-demo.spec.ts`:
   - Boot in demo-bypass mode.
   - Navigate to 5 pages.
   - Assert `page.on('console')` records zero `ClientFetchError` errors.

## Acceptance criteria

- [ ] Browser console under demo-bypass shows no `ClientFetchError`.
- [ ] All app pages still render correctly under demo-bypass.
- [ ] Renderer remains responsive after sustained navigation (no jank from polling loop).
- [ ] New Playwright spec passes.
- [ ] No regression on the real-auth path (NEXT_PUBLIC_ARGO_DEMO_MODE unset / 0).

## May-touch

- `web/components/skycot/AuthProvider.tsx`
- Optionally any component that calls `useSession()` directly and assumes a real provider.
- `web/playwright/auth/no-session-fetch-error-in-demo.spec.ts` (NEW)

## Out of scope

- The env-var rename (covered by `fix-web-mock-bypass-env-var-rename`).
- The signup-page demo-mint proxy (covered by `fix-auth-demo-mint-route-returns-gateway-unreachable`).
- The DemoBanner sidebar overlap (covered by `fix-demo-banner-sidebar-overlap-desktop`).

## Effort

~30-60 LOC + 1 spec. ~2-3 hours. P0 because it's both a demo recording risk (devtools / screen-share) and a real renderer-stability problem.
