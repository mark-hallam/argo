# fix-auth-demo-mint-route-returns-gateway-unreachable

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-auth-demo-mint-route-returns-gateway-unreachable`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/onboarding-hire-an-ai-teammate.md` F6 [P0].

🔴 **P0 DEMO-BLOCKER — `POST /api/auth/demo-mint` returns `{"error":{"code":"gateway_unreachable"}}` even when the gateway is healthy on `http://127.0.0.1:7400/health` (200, schema v83). The signup → wizard handoff is therefore broken on its visible button — the operator can only enter the wizard via direct URL navigation, bypassing the signup beat entirely.**

## Root cause

The Next.js route handler at `web/app/api/auth/demo-mint/route.ts` (or equivalent) proxies to the gateway's `/api/auth/demo-mint` route under `crates/argo-cli/src/handlers/gateway/`. Reproduction:

```
curl http://127.0.0.1:7400/health
→ {"status":"ok","schema_version":83,"uptime_seconds":26391}

curl -X POST http://127.0.0.1:3002/api/auth/demo-mint \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo+x@skycot.test"}'
→ {"error":{"code":"gateway_unreachable","message":"Couldn't reach SKYCOT. Confirm the gateway is running on the configured port."}}
```

Two candidate root causes (audit didn't fully trace):
1. **Port-binding mismatch** — the proxy is reading `NEXT_PUBLIC_GATEWAY_URL` (which clients see) when it should be reading a server-side env (e.g. `GATEWAY_URL` or a hardcoded `http://127.0.0.1:7400`). With Next.js, `NEXT_PUBLIC_*` is also server-visible but the proxy may be defaulting to `http://localhost:3000` or some other miswire.
2. **Cookie-binding side-effect** — the audit notes a comment at `web/app/auth/signup/page.tsx:76-80` referencing a cookie-binding issue.

Per the Scene 3 narrative the camera starts at `/landing` → signup form → wizard. Without a working demo-mint route the wizard is only reachable through the middleware bypass (`NEXT_PUBLIC_ARGO_DEMO_MODE=1`) via direct URL navigation, which breaks the signup→onboarding handoff.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F6 [P0] — `/api/auth/demo-mint` returns `gateway_unreachable` even with the gateway alive

## Fix recipe

1. **Locate the proxy.** Read `web/app/api/auth/demo-mint/route.ts` (or `web/pages/api/auth/demo-mint.ts` if pages-router). Identify the upstream URL it fetches.
2. **Trace the failure mode.** Reproduce with the gateway alive on `:7400` and capture the exact `fetch` URL the proxy assembles. Most likely defects:
   - Reading a client-side-only env var or a stale default port.
   - Catching `ECONNREFUSED` and remapping to `gateway_unreachable` when the underlying error is a 5xx or timeout from the gateway's own demo-mint handler.
3. **Fix the upstream URL.** Make the proxy resolve the gateway as:
   ```ts
   const gateway = process.env.GATEWAY_URL
     ?? process.env.NEXT_PUBLIC_GATEWAY_URL
     ?? "http://127.0.0.1:7400";
   ```
   And confirm the path under `gateway` is `/api/auth/demo-mint`.
4. **Add a server-side health probe BEFORE remapping to `gateway_unreachable`.** If the gateway responds to `/health` 200 but the demo-mint route returns 5xx, surface the underlying error message, do not mask it.
5. **Add an integration test** at `web/tests/api/demo-mint-proxy.test.ts`:
   - Boot a fake gateway on a known port (or mock `fetch`).
   - POST `/api/auth/demo-mint` with a synthetic email.
   - Assert HTTP 200 + a `Set-Cookie` for `__Host-session`.
6. **Cross-link with the signup page** — confirm `web/app/auth/signup/page.tsx:68-72` (the email-required check) doesn't gate the demo-mint button incorrectly; coordinate with `fix-signup-no-email-required-button-honest-copy` if both ship.

## Acceptance criteria

- [ ] With the gateway on `:7400` healthy, `curl -X POST http://127.0.0.1:3002/api/auth/demo-mint -d '{"email":"demo+x@skycot.test"}'` returns HTTP 200 with a `Set-Cookie` header.
- [ ] Clicking the "Or try a demo workspace" button on `/auth/signup` proceeds to `/onboarding/welcome` without an error toast.
- [ ] New integration test passes.
- [ ] No regression on the regular `/auth/signup` magic-link path (covered by `fix-auth-pages-light-theme-to-match-wizard` sister briefs).

## May-touch

- `web/app/api/auth/demo-mint/route.ts` (the proxy)
- `web/lib/api/demo-mint.ts` (or wherever the URL resolution lives)
- `web/tests/api/demo-mint-proxy.test.ts` (NEW)
- Potentially `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (the upstream demo-mint route — only if its 5xx is the underlying cause)

## Out of scope

- The signup "no email required" copy mismatch (covered by `fix-signup-no-email-required-button-honest-copy`).
- The signup demo-mint router race (covered by `fix-signup-demo-mint-router-race-bypasses-wizard`).
- Magic-link / passwordless auth (already shipped via #1823).

## Effort

~100-300 LOC. ~3-6 hours. P0 because the visible signup→onboarding handoff is broken on the demo runtime.
