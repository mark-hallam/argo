# web-auth-config-server-only-split

**Owner**: claude.
**Depends on**: none.
**Authored against**: `origin/main` `d2be56888` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-auth-config-server-only-split`.

🔴 **P0 SHIP-BLOCKER — blocks v1.0 demo MP4 recording.** Source: evaluator bootstrap 2026-05-22T02:25 (`docs/skycot/evaluator/scenarios/spa-cookies-import-app-routes-500.md`) + auditor T03-39 §3.2 / §4 P0 #2. `web/auth.config.ts:3` imports `cookies` from `next/headers` at top level — a server-only Next.js API. Next bundles this file into every Client Component that touches the NextAuth context, so 5 of 6 in-product routes return HTTP 500 on cold dev start. The demo MP4 path walks through these routes; **the demo cannot be recorded until this is fixed**.

## Scope

Split `web/auth.config.ts` into a client-safe config and a server-only helper. The client-safe file holds the static `NextAuthConfig` object (providers list, page paths, callbacks that don't touch request headers). The server-only file owns anything that calls `cookies()`, `headers()`, or other `next/headers` APIs — those callsites must live behind a `"use server"` boundary or in `auth.ts` / route handlers / server actions that NextAuth already isolates.

The current callback that reads the `__Host-session` cookie via `cookies()` either (a) moves into the `auth.ts` server entrypoint (the NextAuth v5 pattern is to have `auth.ts` build on top of `auth.config.ts`), or (b) becomes a `"use server"` helper that the JWT callback delegates to. Either is acceptable; the constraint is: `auth.config.ts` must contain ZERO imports from `next/headers`.

Verify with `grep -r 'from "next/headers"' web/auth.config.ts` returning empty after the fix, AND with a cold `pnpm dev` start where `/`, `/workflows`, `/audit`, `/cohorts`, `/approvals` all return 200 (not 500) on first hit.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/auth.config.ts` — remove the `next/headers` import; keep the static NextAuthConfig shape.
- `web/auth.ts` — move the cookie-reading callback here (NextAuth v5 idiomatic split).
- `NEW: web/lib/auth/session-cookie.ts` (or similar) — server-only cookie helper if the callback can't live directly in `auth.ts`. Must start with the file-level marker so it can never be bundled client-side (use Next's server-only import: `import "server-only";` at the top).
- `web/lib/auth/*.ts` — narrowly scoped: any file that currently re-exports types from `auth.config.ts` so they keep working after the split.
- `NEW: web/__tests__/auth-config-no-server-imports.test.ts` (or co-located) — a unit test that asserts `auth.config.ts` does not import `next/headers`.

## Files you must NOT touch

- Any gateway crate (`crates/argo-gateway/**`).
- Any route handler under `web/app/api/**` other than what's strictly needed to validate the fix.
- The NextAuth session cookie schema (`__Host-session` JWT layout) — server-side decode logic moves, but the cookie format doesn't change.
- `BUILD_STATUS.md` outside the standard row flip.

## Acceptance criteria

- [ ] `grep -r 'from "next/headers"' web/auth.config.ts` returns no matches.
- [ ] `grep -r 'cookies\(\)' web/auth.config.ts` returns no matches.
- [ ] `web/auth.ts` (or the new `web/lib/auth/session-cookie.ts`) contains the cookie-reading code, with `import "server-only";` at the top.
- [ ] A cold `pnpm dev` start serves `/`, `/workflows`, `/audit`, `/cohorts`, `/approvals` with HTTP 200 on first request (not 500). Document with a curl-or-screenshot in the PR body.
- [ ] New unit test fails if a future edit re-introduces a `next/headers` import into `auth.config.ts`.
- [ ] `pnpm typecheck` (or equivalent) clean.
- [ ] `pnpm lint` (or equivalent) clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-auth-config-server-only-split`
- PR title: `web(auth): split server-only cookies() out of auth.config.ts (web-auth-config-server-only-split)`

## Notes for the implementer

- This is a P0 demo-MP4 blocker. **Worker priority**: claim ahead of feature work.
- The fix is small in lines but architecturally important. The pattern is the NextAuth v5 standard split: `auth.config.ts` = providers + static callbacks (edge-safe), `auth.ts` = `NextAuth(authConfig)` + server-only callbacks. See https://authjs.dev/getting-started/migrating-to-v5 (`#configuration` section).
- After the fix, the existing magic-link signup/verify flow must keep working end-to-end. The bearer-token-into-JWT pattern stays intact; only the `cookies()` callsite moves.
- ~150-250 LOC expected. Most of it is moving existing code to a new file and adding the test.
- Pre-flight grep: `rg 'from "next/headers"' web/` to confirm no other client-bundled files smuggle the same import. If others exist, list them in the PR body as a follow-up (don't fix them in this brief; out of scope).
