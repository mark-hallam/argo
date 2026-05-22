# Eval: SPA cookies() import — round-1 P0-A verified CLOSED

**Date**: 2026-05-22 → 2026-05-23 (round 3)
**Pillar**: All (every `/app/(app)/*` route was 500'ing in round 1)
**Mission claim**: 1 / 2 / 3 / 4 / 6 — every claim with an SPA surface
**Verdict**: 🟢 **PASS** — fix landed within 8h of the round-1 brief recommendation; verified at runtime.

## Setup

Round 1 found `web/auth.config.ts:3` importing `cookies` from `next/headers` (server-only), which broke every `/app/(app)/*` route on cold dev start. The recipe I left was:

> **Brief: `web-auth-config-server-only-split`** — Split `auth.config.ts` into:
> 1. `auth.config.ts` (current name) — client-safe, no `next/headers` import.
> 2. `auth.server.ts` (new) — server-only file …

## Action

```bash
git log --oneline 58bc7df16..HEAD -- web/auth.config.ts
head -5 web/auth.config.ts
cd web && pnpm dev > /tmp/eval-web-dev-r3.log 2>&1 &
# wait for compile, then probe the round-1 failing routes:
for p in /landing /auth/signup /audit /workflows /hire /; do
  curl -sL --max-time 15 -o /dev/null -w "$p -> %{http_code}\n" http://127.0.0.1:3000$p
done
```

## Expected

- `auth.config.ts:3` no longer imports `cookies` from `next/headers`.
- All round-1 failing routes return 200 (or 302/307 to /auth/signin).

## Observed

```
$ git log --oneline 58bc7df16..HEAD -- web/auth.config.ts
fba4b1a11 web(auth): split server-only cookies() out of auth.config.ts (web-auth-config-server-only-split) (#2027)

$ head -5 web/auth.config.ts
import type { NextAuthConfig } from "next-auth";
import Credentials from "next-auth/providers/credentials";

/**
 * NextAuth v5 config — edge-safe, client-bundle-safe surface.
```

The `cookies()` import is gone. The file's docstring now explicitly states "edge-safe, client-bundle-safe surface".

Route probes after cold dev restart:

```
/landing      -> HTTP 200   ✅ (unchanged from round 1)
/auth/signup  -> HTTP 200   ✅ (was 500 in round 1)
/audit        -> HTTP 200   ✅ (was 500 in round 1)
/workflows    -> HTTP 200   ✅ (was 500 in round 1)
/hire         -> HTTP 200   ✅ (was 500 in round 1)
/             -> HTTP 200   ✅ (was 500 in round 1)
/hire/step/1  -> HTTP 404   ⚠ by-design (see below)
/onboarding   -> HTTP 404   ⚠ unchanged from round 1
```

### `/hire/step/1` → 404 is by-design, not a regression

The wizard's [step] route at `web/app/(hire)/hire/step/[step]/page.tsx` has:

```typescript
const VALID_STEPS = new Set<number>([2, 3, 4, 5]);
```

Step 1 is the welcome screen at `/hire` directly (no `/step/1` segment). Probing `/hire/step/{2,3,4,5}` returns 200 for all four.

### `/onboarding` → 404 unchanged from round 1

Still a real gap. Per the brand consistency memory, the F1 onboarding wizard was design-locked but never adopted at the route level. Not a P0 — just stale.

## Anti-theater check

Two independent confirmations:

1. **Source code confirms** the bug class is gone: `head -5 web/auth.config.ts` shows no `next/headers` import. The fix split the surface as recommended.
2. **Runtime confirms** the symptom is gone: cold `pnpm dev` + curl returns 200 on all the routes that 500'd in round 1.

I also confirmed PR [#2027](https://github.com/mark-hallam/Argo-Agent/pull/2027) is on `origin/main` (commit `fba4b1a11`), not just in a worktree.

## Gap / follow-up brief candidates

**🟢 NONE** for this scenario — P0-A is closed.

**🟡 P3 — `web-onboarding-f1-route-restore`**: `/onboarding` still 404s. Per the operator's `project_onboarding_design_locked.md` memo, F1 was design-locked but not adopted. Either restore the route to point at the F1 implementation, OR remove the orphan reference from any nav/sitemap. Low priority.

## What this scenario proves

🟢 **The evaluator-conductor-worker loop closed BOTH round-1 P0s within 24 hours.**

| Round-1 P0 | Brief I recommended | PR shipped | Result |
|---|---|---|---|
| P0-A — SPA cookies import | `web-auth-config-server-only-split` | [#2027](https://github.com/mark-hallam/Argo-Agent/pull/2027) | ✅ runtime verified |
| P0-B — migration `;` in comments | `claude-migration-comment-aware-splitter` | [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) | ✅ both cargo tests now pass |

This is what success looks like for the evaluator: surface a P0, the conductor authors the brief with the recommended name + recipe, a worker ships the fix, the next eval verifies. The loop took **<24 hours** for both P0s.
