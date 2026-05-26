# fix-csp-connect-src-gateway-port-drift

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-csp-connect-src-gateway-port-drift`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F5 [P1].

🟠 **P1 — On a fresh dev-server boot the CSP `connect-src` was observed emitting `http://127.0.0.1:8787`, while the SPA hooks default-call `http://127.0.0.1:7400` (the canonical port per CLAUDE.md). `.env.local` says 7400. If the operator boots a clean dev server with stale env (or any production build with a build-time `NEXT_PUBLIC_GATEWAY_URL` pointing somewhere else), every browser fetch will be blocked silently by CSP with only a small browser-console line — page just appears broken.**

## Root cause

`web/next.config.ts:6` reads `gatewayUrl` from `NEXT_PUBLIC_GATEWAY_URL`. The CSP `connect-src` header is assembled at the same point in `next.config.ts` (the `headers()` block). If the env var at build time disagrees with the env var the SPA hooks read at runtime, the CSP whitelists a different host than the hooks call → every fetch is browser-blocked.

For dev mode this is recoverable (operator restarts with corrected env). For a production build, the CSP is baked at build time and cannot be changed without a redeploy — making this a higher-risk variant.

The fix requires three things:
1. CSP and hook URLs derive from a single shared source.
2. A runtime assertion / health check that surfaces the mismatch as a visible banner instead of silent failure.
3. A build-time check that errors out if the canonical port has drifted from the docs/CLAUDE.md.

## Findings this brief closes

- whole-site-mvp-claims-coverage F5 [P1] — CSP `connect-src` may diverge from gateway port under prod build

## Fix recipe

1. **Centralise the gateway URL constant** at `web/lib/config/gateway-url.ts` (NEW):
   ```ts
   export const GATEWAY_URL =
     process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:7400";
   ```
   All consumers (`next.config.ts` CSP block + every `web/lib/api/*.ts` fetch) import from this module.

2. **Wire `next.config.ts` to import from the same source** — or duplicate the const at the top of `next.config.ts` and assert at build time they match:
   ```ts
   const gatewayUrl = process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:7400";
   const CANONICAL = "http://127.0.0.1:7400";
   if (gatewayUrl !== CANONICAL && process.env.NODE_ENV === "production") {
     console.warn(`[CSP] Gateway URL ${gatewayUrl} does not match canonical ${CANONICAL}`);
   }
   ```

3. **Add a runtime health probe** at `web/components/skycot/AppShell.tsx` (or a sibling Bootstrap component):
   - On first mount, `fetch(GATEWAY_URL + "/health")` with a 2s timeout.
   - If the fetch fails AND the browser console shows a CSP violation, surface a banner: "Gateway unreachable — verify your dev environment matches the documented port (7400)."
   - Suppress the banner if the fetch succeeds.

4. **Add a build-time check** at `web/scripts/check-canonical-gateway-port.ts` (or via a check in `web/package.json`'s pre-build hook):
   - Read `NEXT_PUBLIC_GATEWAY_URL` from `.env.local` / `.env.production`.
   - Assert it equals the canonical `http://127.0.0.1:7400` (or whatever the production gateway should be — the script reads from a constant matching CLAUDE.md).
   - Exit non-zero if mismatch.

5. **Add a Playwright spec** at `web/playwright/app-shell/gateway-port-canary.spec.ts`:
   - Boot in dev mode.
   - Open the app.
   - Inspect the CSP header emitted on the first request.
   - Assert `connect-src` contains the canonical port.

## Acceptance criteria

- [ ] All gateway URL reads (CSP, fetch hooks, runtime probes) trace back to a single shared constant.
- [ ] On boot, if the gateway is unreachable, a visible banner explains the likely env-var drift.
- [ ] Production-build script fails CI if `NEXT_PUBLIC_GATEWAY_URL` disagrees with the canonical port.
- [ ] New Playwright spec passes.
- [ ] No regression on dev-mode boot when env is correctly configured.

## May-touch

- `web/next.config.ts` (CSP block + gatewayUrl constant)
- `web/lib/config/gateway-url.ts` (NEW)
- `web/lib/api/*.ts` (consume the new constant — search for any local `GATEWAY_URL` redeclarations)
- `web/components/skycot/AppShell.tsx` (health probe)
- `web/scripts/check-canonical-gateway-port.ts` (NEW)
- `web/playwright/app-shell/gateway-port-canary.spec.ts` (NEW)
- `web/package.json` (pre-build hook for the canonical-port check)

## Out of scope

- The gateway-port choice itself (canonical = 7400 per CLAUDE.md; sister brief `fix-claude-md-status-block-port-and-staleness-sweep` covers any drift in docs).
- Gateway CORS preflight (covered by `fix-gateway-cors-localhost-127-and-preview-proxy-origins`).
- Demo-mint proxy URL (covered by `fix-auth-demo-mint-route-returns-gateway-unreachable`).

## Effort

~150-300 LOC + 1 spec + 1 build script. ~4-6 hours. P1 — silent-failure risk in production, recoverable in dev but with a long time-to-diagnose.
