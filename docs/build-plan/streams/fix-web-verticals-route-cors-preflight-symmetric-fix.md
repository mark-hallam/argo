# Stream fix-web-verticals-route-cors-preflight-symmetric-fix — symmetric CORS rewrite for bare `/verticals` discovery

**Owner**: claude (web frontend / Next config).
**Depends on**: none (independent of `mission-gateway-add-api-v1-verticals-cors-mounted-discovery`; this brief covers the web side, that brief covers the gateway side).
**Authored against**: origin/main `234ffd9f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-web-verticals-route-cors-preflight-symmetric-fix`.

🟡 **P1 if the v1.0 demo MP4 captures the `/verticals` discovery flow (workflows page), otherwise P2.** Source: round-49 evaluator finding `gateway-vertical-discovery-sibling-gap-confirm` + r51 carry-forward (5th-consecutive round).

## Scope

`web/lib/api/workflows.ts:7` documents the gateway contract `GET /verticals → discovery` but the only rewrite in `web/next.config.ts:35` is `/api/v1/:path* → gateway/api/v1/:path*` — the bare `/verticals` path is NOT proxied. In dev the browser fetches `http://localhost:3000/verticals` (or the gateway directly at `http://127.0.0.1:7400/verticals`) and either hits the Next.js 404 page or the gateway's CORS-preflight failure (the gateway has no OPTIONS handler — that's WHY the rewrite exists for `/api/v1/*`).

This brief covers the WEB side of the fix: add a symmetric rewrite so `GET /verticals` proxies through Next.js exactly like `/api/v1/*` does. Once the sibling gateway brief lands `/api/v1/verticals` as the canonical discovery endpoint, this rewrite becomes a temporary back-compat shim until web callers migrate to the prefixed URL — but in the meantime it unblocks any frontend that follows the documented `GET /verticals` contract.

Two acceptable implementations:

1. **Add the rewrite** to `web/next.config.ts:35` rewrites block: `{ source: "/verticals", destination: \`${gatewayUrl}/api/v1/verticals\` }` and `{ source: "/verticals/:path*", destination: \`${gatewayUrl}/api/v1/verticals/:path*\` }`.
2. **Migrate the caller** to the `/api/v1/verticals` path. Update `web/lib/api/workflows.ts` JSDoc + the call site in `web/app/(app)/workflows/page.tsx`. Cleaner long-term but only safe AFTER the sibling gateway brief mounts the endpoint.

Default to option 1 for back-compat. If the sibling gateway brief is already MERGED at claim time, option 2 is preferred.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`web/next.config.ts`]

## Files you may touch

- `web/next.config.ts` — add the rewrite entry in the `rewrites()` async block (lines ~33-37). Keep the existing `/api/v1/:path*` rule.
- `web/tests/e2e/verticals-discovery-cors.spec.ts` (NEW, optional) — Playwright spec that fetches `/verticals` from the dev server and asserts a non-CORS response (status 200 with JSON, or 404 with JSON envelope — anything that proves the request reached the gateway).
- `web/lib/api/workflows.ts:7-13` — if going with option 2, update the JSDoc comment block to reference `/api/v1/verticals` everywhere.

## Tests

- Add a hermetic test in `web/tests/unit/next-config-rewrites.test.ts` (or extend existing if present) that exercises the rewrites array and asserts a `/verticals` entry pointing at the gateway. ~20 LOC.
- Optional Playwright e2e per above.

## Definition of done

- `web/next.config.ts` exports a rewrite that proxies `/verticals[/...]` to `${gatewayUrl}/api/v1/verticals[/...]`.
- `pnpm test --filter web` green; existing rewrite tests still pass.
- PR body includes: a `curl -i http://localhost:3000/verticals` showing the proxied response (run against a local Next dev + gateway).
- PR body cites the round-49 evaluator finding + this brief.

Notes: if running `pnpm dev` alongside the gateway, validate the round trip end-to-end. If the gateway side returns 404 because the sibling brief hasn't merged yet, that's still a PASS for THIS brief — it proves the rewrite reaches the gateway (the bug is "request never gets there because Next.js 404s first").
