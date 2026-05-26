# fix-zombie-pnpm-dev-port-3000-from-stale-worktree

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-zombie-pnpm-dev-port-3000-from-stale-worktree`.
**Source**: ux-audit-2026-05-26-2214 `onboarding-hire-an-ai-teammate.md` F-NEW1 [P0].

🔴 **P0 DEMO-BLOCKER — a `pnpm dev` worker process from a long-ago brief is squatting `localhost:3000` from `/private/tmp/fix-playwright-hire-flow-regression-cluster/web` with no `.env.local`. Operators recording the demo against the canonical `http://localhost:3000` see auth.js `MissingSecret` 500s on `/api/auth/session` and 502s on `/api/auth/demo-mint` — all camera-visible. Demo-readiness footgun: the operator does not know they're hitting the wrong server.**

## Root cause

The supervisor's worker-loop (`scripts/claude-worker-loop.sh`) spawns each brief's session in its own `/tmp/<brief-id>-target/` directory and runs `pnpm --dir web dev` to bring up the SPA for verification. When a worker crashes mid-brief — or when the supervisor reaps the bash loop without first killing its `pnpm dev` child — the Next.js process is orphaned and keeps holding port 3000.

The orphaned worktree directory (`/private/tmp/fix-playwright-hire-flow-regression-cluster/web` in this audit) typically has only `.env.example` (the per-brief checkouts don't include `.env.local` because workers source env vars from the supervisor parent's environment, not from a checked-in file). So the orphan keeps running with no `AUTH_SECRET`, no `NEXT_PUBLIC_GATEWAY_URL`, no `NEXT_PUBLIC_ARGO_DEMO_MODE` — which produces all the symptoms above.

Verified at audit:

```
lsof -nP -iTCP:3000 -sTCP:LISTEN
# node 35584 operator — cwd /private/tmp/fix-playwright-hire-flow-regression-cluster/web

ls /private/tmp/fix-playwright-hire-flow-regression-cluster/web/.env*
# only .env.example present
```

Symptoms observed via Claude Preview MCP `preview_console_logs` on `http://localhost:3000`:

- `/api/auth/session` → HTTP 500 with `auth.js MissingSecret: Please define a 'secret'`
- `/api/auth/demo-mint` → HTTP 502 with `{"error":{"code":"gateway_unreachable"}}` (even though gateway is alive at `127.0.0.1:7400`)
- next-auth `SessionProvider` retries `/api/auth/session` infinitely from any rendered page

Anyone (operator or audit agent) who naively visits `http://localhost:3000` during recording walks into a broken-looking app — the camera sees `MissingSecret` 500s in the browser DevTools console and the "We couldn't reach SKYCOT" error toast on the demo-mint button.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F-NEW1 [P0] — Zombie `pnpm dev` squatting `localhost:3000` from a stale worktree

## Fix recipe

Two complementary fixes — ship both.

**A. Pre-record kill script (operator-runnable).**

Add `scripts/pre-record-clean-ports.sh` (~30 LOC):

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "[pre-record] Killing any stray dev servers on ports 3000-3010..."
lsof -nP -iTCP:3000-3010 -sTCP:LISTEN -t 2>/dev/null | xargs -r kill -9 || true

echo "[pre-record] Confirming gateway alive on 7400..."
if ! lsof -nP -iTCP:7400 -sTCP:LISTEN -t >/dev/null 2>&1; then
  echo "[pre-record] ERROR: gateway not running on 7400. Run scripts/restart-live-gateway.sh first."
  exit 1
fi

echo "[pre-record] Clean. Safe to start pnpm --dir web dev."
```

Add a runbook-pre-flight line in `docs/skycot/demos/v1-mission-mvp-runbook.md`:

> Before take 1: `bash scripts/pre-record-clean-ports.sh` — kills any orphaned dev servers and confirms the gateway is alive.

**B. Supervisor / worker-loop reap-on-exit hook.**

Patch `scripts/claude-worker-loop.sh` (or the equivalent worker bash loop) so that when a brief's session exits (either RC=0 or crash), the bash trap also kills any `pnpm dev` child:

```bash
cleanup_brief() {
  # Kill any pnpm dev / next-dev process this brief spawned
  pkill -P "$$" -f "pnpm.*--dir web dev" 2>/dev/null || true
  pkill -P "$$" -f "next dev" 2>/dev/null || true
  # ...existing cleanup...
}
trap cleanup_brief EXIT
```

This stops new zombies from accumulating. The pre-record script handles any zombies that pre-exist the patch.

**C. Optional: have `pnpm dev` print a startup banner with the worktree path so the operator can eyeball whether the running server is from the canonical `~/code/argo` checkout vs `/private/tmp/<brief-id>`.**

Patch `web/package.json` `scripts.dev`:

```json
"dev": "echo '[dev] cwd:' $(pwd) && next dev"
```

Cheap, non-intrusive, makes the wrong-server case loud.

## Acceptance criteria

- [ ] `scripts/pre-record-clean-ports.sh` exists, is executable, kills any process holding ports 3000-3010, and confirms gateway:7400 is alive
- [ ] `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-flight checklist includes a line about running the script before take 1
- [ ] `scripts/claude-worker-loop.sh` (or equivalent) has a trap that kills child `pnpm dev` / `next dev` processes on brief exit
- [ ] Optional: `web/package.json` `dev` script prints the cwd at startup
- [ ] Repro test: spawn `pnpm --dir web dev` in background from a temp dir without `.env.local`, run the kill script, verify port 3000 is free

## May-touch

- `scripts/pre-record-clean-ports.sh` (NEW, ~30 LOC)
- `scripts/claude-worker-loop.sh` (~10 LOC trap addition)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (~3 LOC checklist addition)
- `web/package.json` (optional, 1 LOC)

## Out of scope

- Fixing the underlying brief whose worker crashed (`fix-playwright-hire-flow-regression-cluster` — that brief is its own ledger row)
- Renaming the auth-bypass env vars (separate brief `fix-claude-launch-json-env-var-naming-mismatch`)
- The next-auth `SessionProvider` polling storm (separate brief `fix-spa-sessionprovider-disable-in-demo-bypass`)

## Effort

~45 LOC across 3 files. ~30 min. P0 because the camera-visible failure mode is severe — operator records `MissingSecret` 500s if they don't notice the port collision.
