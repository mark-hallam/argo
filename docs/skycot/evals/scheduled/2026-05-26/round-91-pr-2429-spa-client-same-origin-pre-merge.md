# Round 90 · S2 · PR #2429 spa-client same-origin gateway — PRE-MERGE PASS

- **Date**: 2026-05-26 (UTC) / 2026-05-27 AEST
- **Round**: 90
- **Claim/Pillar**: Claim 6 (multi-tenant + multi-channel — but really a SPA wiring fix; demo-recording P0 unblocker for Scene 6 audit drill)
- **Verdict**: 🟢 **PASS** (P0 — closes 3× P0 + 1× P1 UX audit findings)
- **PR**: [#2429](https://github.com/mark-hallam/skycot/pull/2429) — `fix(spa-client): use same-origin /api/v1/* in browser — closes T0119 F01/F02`, head `c9f9718e7`, +52/-0 across 2 files (`web/lib/api/client.ts` +6 prod; `web/lib/api/client.test.ts` +46 tests)

## Setup

```bash
git worktree add /tmp/r90-2429-wt c9f9718e7
cd /tmp/r90-2429-wt/web && pnpm install --frozen-lockfile
pnpm vitest run lib/api/client.test.ts
pnpm tsc --noEmit
grep -nE "rewrites?|api/v1|gateway" /tmp/r90-2429-wt/web/next.config.ts
```

## Observed (production code)

The fix at `web/lib/api/client.ts:85-88` branches on execution context:

```ts
function gatewayBase(): string {
  if (typeof window !== "undefined") return "";
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}
```

- **Browser** → `""` so `fetch("/api/v1/audit")` is same-origin. Next.js's existing `rewrites()` at `web/next.config.ts:30-32` (`{ source: "/api/v1/:path*", destination: \`${gatewayUrl}/api/v1/:path*\` }`) proxies the request to the gateway — CORS doesn't enter the picture because the browser sees only its own origin.
- **SSR / route handlers** → absolute `NEXT_PUBLIC_GATEWAY_URL`. These run inside the Next.js process, which is the requester (not a browser), so cross-origin / CORS rules don't apply.

The line-185 callsite (`const url = path.startsWith("http") ? path : \`${gatewayBase()}${path}\`;`) inherits the branch transparently.

Verified the `next.config.ts` rewrite is genuinely present: `grep -n "rewrites" web/next.config.ts` returns line 30; line 32 matches the destination pattern claimed.

## Observed (test code)

Both new tests in `web/lib/api/client.test.ts:202-246`:

1. **Browser context** (lines 203-223) — happy-dom env has `window`. Stubs `globalThis.fetch`, calls `apiGet("/api/v1/audit")`, asserts:
   - `expect(calls).toEqual(["/api/v1/audit"])` — exact URL match, no prefix
   - `expect(calls[0]).not.toMatch(/^https?:/)` — explicit cross-axis confirmation that the URL is path-only
2. **SSR context** (lines 225-245) — `vi.stubGlobal("window", undefined)`. Stubs `fetch`, calls `apiGet("/api/v1/audit", { token: "ssr-token" })`, asserts `expect(calls).toEqual(["http://gw.test/api/v1/audit"])`.

`pnpm vitest run lib/api/client.test.ts` → **13/13 PASS in 9ms** (5 original + 6 session-redirect + 2 new). Test runtime 9ms inside duration 1.56s.

## Anti-theater check (dispositive)

The test is **not** structured as "spy on `gatewayBase()` directly" (which would tautologically pass). It mocks `globalThis.fetch` and asserts the URL string the production code actually constructed and dispatched. Two independent axes per test:

- Browser test: asserts both the **exact path** AND the **negative regex** (`not.toMatch(/^https?:/)`) — even if a future regression accidentally restored the absolute URL with a different protocol (e.g. `https://`), the second assertion would catch it.
- SSR test: uses `vi.stubGlobal("window", undefined)` (NOT a private `gatewayBase` knob), so the production code path is exercised through its real runtime guard.

Production fix is library-supported (Next.js rewrites are documented + already shipped at lines 30-32; the fix just uses what's there).

**89th anti-theater verification this week.**

## Process drift — 50-cycle ghost-tick zombie-claim loop

While verifying this PR I noticed `git log --since="2026-05-26 17:00:00 +0000"` shows **50 commits** alternating `status: claim fix-spa-client-use-same-origin-gateway-rewrite` ↔ `status: release fix-spa-client-use-same-origin-gateway-rewrite (zombie; no PR opened)` between **03:00:01 AEST → 04:04:36 AEST** — ~64 minutes, ~25 claim/release cycles, before a worker finally opened #2429.

The fix is 6 LOC of production code; the workers all timed out / were killed before they could commit. The supervisor's worker-respawn keeps re-claiming the same first-eligible TODO, hits the same timeout, releases, repeats.

PR #2338 (ghost-tick detection) merged at T13-30 yesterday — the detection exists, but there's no automatic **block-after-N-consecutive-zombies** circuit breaker for the same brief. Each ghost-tick commit pushes to main and pollutes `git log` (this hour's main has ~50 status commits + 5 real PRs). It also burned ~64min of worker budget.

## Findings closed by this PR

- T0119 F01 [P0] cross-origin CORS — closed by production code path
- T0119 F02 [P0] `/audit` permanent loading skeleton — root cause
- T0119 scene-1-2 F1 [P0] `/workers` mock-offline chip — root cause
- T0119 scene-4-5 F6 [P1] `/skills` `0 skills` SSR flash — root cause

## Gap / follow-up brief candidates

- 🟢 **P0 MERGE** PR #2429 (anti-theater dispositive, evaluator-pre-verified for `--skip-clippy` web-only fix-* lane)
- 🟡 **P2 NEW** — `ops-supervisor-block-runaway-zombie-claims-on-brief` (~40 LOC bash; detect ≥5 release-no-pr cycles on same brief in <30min and PAUSE worker spawning for that brief for 15min cooldown. Today's incident: 25+ cycles in ~64min burned worker budget). Recommended under ops-* `--skip-clippy` lane.
