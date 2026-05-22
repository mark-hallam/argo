# Eval: SPA bootstrap — every in-product route returns HTTP 500

**Date**: 2026-05-22
**Pillar**: Cross-pillar (teammates / skills / workflows / approvals — all SPA surfaces)
**Mission claim**: 1 / 2 / 3 / 4 / 6 — every claim that has a SPA surface
**Verdict**: 🔴 FAIL — **v1.0 demo MP4 ship-gate blocker**

## Setup

- Clean working tree on `main` at `b566866d4`.
- Gateway already running (PID 28198, uptime 92h) and healthy: `curl http://127.0.0.1:7400/health` → `{"status":"ok","schema_version":70,...}`.
- Started Next.js dev server cold: `cd web && pnpm dev > /tmp/eval-web-dev.log 2>&1 &`.
- After cold start, `pnpm install --prefer-offline` was needed once to pull `@radix-ui/react-tooltip@1.2.8` (declared in `package.json` but missing from `node_modules`). One missing-dep round-trip is normal-ish but worth noting on its own — see "Side finding" below.

## Action

After install, hit six representative routes:

```bash
for p in /landing /auth/signup /hire/step/1 /onboarding /audit /workflows; do
  code=$(curl -sL -o /dev/null -w "%{http_code}" http://127.0.0.1:3000$p)
  echo "$p -> $code"
done
```

## Expected

All six routes return HTTP 200 in development (or HTTP 302/307 for an auth-gated redirect).

## Observed

```
/landing       -> 200   ✅
/auth/signup   -> 500   🔴
/hire/step/1   -> 500   🔴
/onboarding    -> 404   ⚠ (separate finding — no /onboarding route exists)
/audit         -> 500   🔴
/workflows     -> 500   🔴
```

Five of six routes broken. Only the marketing route group (`/landing`, `/compare`, `/pricing`, etc.) renders.

Root cause from the dev-server compile output (`/tmp/eval-web-dev.log`):

```
./auth.config.ts:3:1
Ecmascript file had an error
  1 | import type { NextAuthConfig } from "next-auth";
  2 | import Credentials from "next-auth/providers/credentials";
> 3 | import { cookies } from "next/headers";
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

You're importing a component that needs "next/headers". That only works in
a Server Component which is not supported in the pages/ directory.

Import trace:
  Client Component SSR:
    ./auth.config.ts [Client Component SSR]
    ./auth.ts [Client Component SSR]
    ./lib/api/client.ts [Client Component SSR]
    ./lib/hooks/use-approval-bundles.ts [Client Component SSR]
    ./app/(app)/page.tsx [Client Component SSR]
    ./app/(app)/page.tsx [Server Component]
```

`auth.config.ts` is transitively imported into a Client Component bundle via:

`auth.config.ts → auth.ts → lib/api/client.ts → use-approval-bundles.ts → (app)/page.tsx`.

Because `cookies()` from `next/headers` is server-only, Turbopack refuses to bundle that import chain into a Client Component. Every page that depends on the API client (which is every in-product page) fails.

## Anti-theater check

This is **not** a mock-vs-real distinction — it is a genuine compile failure on cold start. The proof is in the import trace + the line number in the source:

```bash
git log --oneline -S 'cookies' -- web/auth.config.ts
# 58bc7df16 v1 — web — JWT role-claim end-to-end + gateway per-action RBAC (#1901)
```

The import landed in PR [#1901](https://github.com/anthropics/argo/pull/1901) (commit `58bc7df16`). No subsequent commit has touched `web/auth.config.ts` since. The dev server is faithfully reporting what the source code says.

This means the v1.0 demo MP4 path that the operator memo references (`/team`, `/workers/<id>/overview`, `/workflows/<v>/<n>/build`, `/audit`) **cannot be recorded against this server**. The demo would 500 on every screen after `/landing`.

Why the swarm didn't see it: the conductor merges PRs against `main` but does not run `pnpm dev` per merge — its merge gate is `cargo build` + GH workflows. The swarm's running gateway has 92h uptime, so its `.next` cache (if any) predates #1901; but a cold `pnpm dev` reveals the bug. The operator's "the demo MP4 = ship gate" memo (`project_v1_demo_mp4_ship_gate.md` 2026-05-21) flagged the recording prep steps but did not prescribe a cold-start dev-server probe.

## Gap / follow-up brief candidate

**Brief: `web-auth-config-server-only-split`** — Split `auth.config.ts` into:

1. `auth.config.ts` (current name) — client-safe, no `next/headers` import. Holds `authConfig` const + types only.
2. `auth.server.ts` (new) — server-only file that imports `cookies()` and exports the session-callback factory.

Then either (a) inline the cookie-decode into a server-only API route handler, or (b) gate the cookie-import behind a dynamic `import("next/headers")` that only fires server-side.

Acceptance:
- `curl -sL -o /dev/null -w "%{http_code}" http://localhost:3000/audit` returns 200 (or 302 to /auth/signin), not 500.
- All six routes in the probe loop above return 200/302.
- No regression to the `__Host-session` HS256 JWT cookie flow established by #1901.
- A `pnpm dev --turbopack` cold start + a `pnpm dev --webpack-dev` cold start both produce no auth-related compile errors.

**Brief: `web-pnpm-install-missing-on-cold-start`** (smaller, side-finding) — Either lockfile drift between #1957 (tooltip primitive landing) and the operator's local install state, or a missed-by-CI scenario where `pnpm install` is not part of any pre-commit hook. Worth a one-line addition to the `scripts/dev-safe-check.sh` to fail early if `node_modules/.pnpm/<top-level-deps>` is missing.

**Brief: `web-onboarding-route-restore`** — `/onboarding` returns 404. The `project_onboarding_design_locked.md` memo (2026-05-19) says onboarding F1 was design-locked but the audit notes "F1 not adopted". Confirms gap; not blocking this eval scenario, just noting.

## Side notes

- The browser-driven Playwright suite (`pnpm test:e2e`) would catch this in CI if it ever ran cold-start against a freshly-cloned worktree. Worth checking whether the e2e job uses a cached `.next` or builds from scratch.
- Per `feedback_styled_jsx_requires_use_client.md` memory (2026-05-20): the team has had similar server/client boundary traps before. Same root pattern.
