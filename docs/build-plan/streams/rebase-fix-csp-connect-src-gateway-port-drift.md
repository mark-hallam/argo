# rebase-fix-csp-connect-src-gateway-port-drift

Authored against: origin/main `0c7cff50f5` on 2026-05-29
Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-csp-connect-src-gateway-port-drift

## Why this brief exists

PR #2593 (517 LOC, 28 files, 2026-05-27) targeted whole-site F5 [P1]: the Content-Security-Policy `connect-src` directive is emitted from one ad-hoc fallback in `next.config.ts`, while SPA fetch hooks read the gateway URL from a SECOND ad-hoc fallback in 15+ separate source files. Any divergence makes the browser silently CSP-block every fetch with only a small browser-console line as evidence. For dev mode this is recoverable; for a production build the CSP is baked at build time and cannot be changed without a redeploy — higher-risk variant.

Original PR's approach (3 concentric guards):
1. **Source-of-truth module** `web/lib/config/gateway-url.ts` exporting `CANONICAL_LOCAL_GATEWAY_URL`, `GATEWAY_URL`, `isCanonicalLocalGateway()`.
2. **Runtime health probe** `web/components/skycot/GatewayHealthProbe.tsx` mounted once inside `AppShell`, fires a 2s `/health` ping, surfaces an amber banner on failure.
3. **Build-time guard** `web/scripts/check-canonical-gateway-port.mjs` wired as `prebuild` — fails if the two canonical constants drift; in strict mode also rejects unset / off-canonical `NEXT_PUBLIC_GATEWAY_URL`.

The bug is STILL load-bearing — verified 8+ `DEFAULT_GATEWAY` redeclarations across:
- `web/lib/api/genesis.ts:274`
- `web/lib/api/workflows-save-publish.ts:40`
- `web/lib/api/sso.ts:73`
- `web/lib/api/client.ts:22`
- `web/lib/api/auth.ts`
- `web/lib/api/connectors.ts`
- `web/lib/hooks/use-approvals-live.ts`
- `web/lib/hooks/use-skill-trace.ts`
- (plus 7 more under `web/app/api/**/route.ts`)

No canonical `web/lib/config/gateway-url.ts` module exists.

The PR conflicted on `web/components/skycot/AppShell.tsx` only, but is too large (28 files, 517 LOC) for routine manual rebase — and 7 of the touched files have evolved through #2522 (X-Argo-Tenant-Id resolver) + #2500 (CSRF wiring) + #2429 (same-origin /api/v1/*) all of which need re-coordination.

The simpler re-implementation path is identical to the original (3 concentric guards) — but re-author against current main and accept that this will be a 1-2 day PR not an evening fix. Worth doing properly because the F5 vulnerability is a production deploy hazard.

PR #2593 was closed for re-author.

## Source of truth

- Original brief: `docs/build-plan/streams/fix-csp-connect-src-gateway-port-drift.md`
- Closed PR: #2593
- Current main shape: 8+ `DEFAULT_GATEWAY = "http://127.0.0.1:7400"` redeclarations; no canonical module.

## Acceptance

Same as original brief:
- One canonical `gateway-url.ts` module; every SPA fetch base + WS builder + SSR route handler reads through it.
- Runtime health probe surfaces drift as a dismissable banner (suppressed in demo mode).
- Build-time guard refuses to ship if the two `next.config.ts` ↔ `gateway-url.ts` constants drift.
- Playwright canary at `web/tests/e2e/gateway-port-canary.spec.ts` asserts CSP `connect-src` whitelists `http://127.0.0.1:7400` and does NOT contain historical-drift port `8787`.

## Boundary

- May touch: all 8+ files redeclaring `DEFAULT_GATEWAY` + the 7 route.ts handlers in `web/app/api/**/route.ts` + `web/next.config.ts` + `web/components/skycot/AppShell.tsx` + new files `web/lib/config/gateway-url.ts` + `web/lib/config/gateway-url.test.ts` + `web/components/skycot/GatewayHealthProbe.tsx` + `web/scripts/check-canonical-gateway-port.mjs` + `web/tests/e2e/gateway-port-canary.spec.ts`.
- Must coordinate with #2522 (`X-Argo-Tenant-Id` resolver) + #2500 (CSRF wiring) + #2429 (same-origin /api/v1/* in browser) — do not regress.

## Verification

- `pnpm typecheck`
- `pnpm lint`
- `pnpm test --run`
- `pnpm check:gateway-port` (advisory)
- `pnpm check:gateway-port --strict` (simulated prod; reject off-canonical URLs)
- `pnpm test:e2e tests/e2e/gateway-port-canary.spec.ts` against booted dev server
