# claude-web-auth-config-next-headers-split

**Authored against**: origin/main `9933f20b` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-web-auth-config-next-headers-split`.

## Summary

`pnpm -C web build` has failed on `origin/main` since at least #1941 (cited as a "pre-existing build failure" by every PR in the wave-15 merge wave: #1931–#1945, plus surfaced again by `claude-web-test-flake-sweep-wave-5`). Reproduce:

```
> next build
./auth.config.ts
Error: You're importing a component that needs "next/headers". That only
works in a Server Component which is not supported in the pages/ directory.
Import trace:
  ./auth.config.ts → ./auth.ts → ./lib/api/client.ts → ./app/(setup)/setup/page.tsx
```

`web/app/(setup)/setup/page.tsx` is a Client Component (`"use client"`) that imports `apiGet`/`apiPost` from `lib/api/client.ts`. `client.ts:33` then does `await import("@/auth")` inside `if (typeof window === "undefined")` to read the bearer token server-side. Webpack treats the dynamic import as a static graph edge, so `auth.ts → auth.config.ts → next/headers` ends up in the client bundle graph even though it never runs in the browser.

The magic-link cookie read in `auth.config.ts` session callback is needed because the magic-link flow sets `__Host-session` directly (bypassing the Credentials provider). The 3 route handlers that proxy to the gateway (`api/workflows/propose`, `api/workers/[id]/freeze`, `api/workers/[id]/unfreeze`) all rely on `session.gatewayToken` being populated from that cookie. `useViewerPolicy` (used by audit-page client components) reads `session.role` via `useSession()`.

## Acceptance

- [ ] `pnpm -C web build` succeeds on the branch.
- [ ] Magic-link users continue to have `session.gatewayToken` populated when route handlers call `auth()`.
- [ ] `useViewerPolicy` continues to receive `session.role` via `useSession()` for magic-link users.
- [ ] No new "needs next/headers" or "server-only" build errors on any client/middleware path.
- [ ] `pnpm -C web test` stays green.

## May touch

- `web/auth.ts`, `web/auth.config.ts`.
- `web/lib/api/client.ts` (the `readSessionToken()` server branch is the static-import root cause).
- A new `web/lib/auth/magic-link-server.ts` (or similar) marked with `import "server-only"`.
- `web/app/api/auth/[...nextauth]/route.ts` only if the fix routes through a wrapper handler.
- `web/app/api/workflows/propose/route.ts`, `web/app/api/workers/[id]/{freeze,unfreeze}/route.ts` — only if the fix moves the cookie read into the route handlers themselves.

## Must NOT touch

- Locked design files.
- Seed files (`web/lib/sample-*.ts`).
- The `useViewerPolicy` client-side resolver (`web/lib/hooks/use-viewer-policy.ts`) — its contract is "session.role is a string" and that should stay stable.

## Notes on solution shape

Two viable paths:

1. **Break the static-import chain at `lib/api/client.ts`.** Remove the `await import("@/auth")` server branch entirely and have server-side callers pass an explicit `token` to `apiGet/apiPost` (the `ApiRequestInit.token` field already exists). Most server callers are in `lib/api/*` wrappers that can read `auth()` or the cookie themselves and pass through.

2. **Move magic-link reading to `auth.ts`, then make `lib/api/client.ts` resolve `@/auth` opaquely.** The session callback in `auth.config.ts` stops doing `next/headers`; `auth.ts` wraps `NextAuth(authConfig)` with an extended session callback that reads the cookie. To stop webpack from tracing `auth.ts → next/headers` into the client bundle, `client.ts`'s `await import("@/auth")` needs to use a variable specifier or `webpackIgnore: true`. (Note: a prior attempt at this in the test-flake sweep failed because just moving the import to `auth.ts` doesn't help — the trace still goes through `client.ts → auth.ts`. The variable-specifier or `webpackIgnore` step is essential.)

Path 1 is the cleaner architectural fix; path 2 is the smaller diff. Operator's call.

The audit chain: #1931, #1932, #1933, #1934, #1935, #1936, #1937, #1938, #1939, #1940, #1941, #1942, #1943, #1944, #1945 all declare this as a known pre-existing build failure. `claude-web-test-flake-sweep-wave-5` (test-only sweep) deferred it as out of scope.
