# web-perf-cache-control-marketing — fix `Cache-Control: no-store` on public marketing pages

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-perf-cache-control-marketing`.
**Priority**: P3 — wave-2 perf: /landing serves `Cache-Control: no-store, must-revalidate` (verified via curl -I). Fails Lighthouse "efficient cache policy"; impacts SEO + CDN edge caching.
**Sizing**: ~50-100 LOC (next.config.ts cache headers per public path + verification).

## Why this brief exists

From the wave-2 perf audit:

> /landing ships `Cache-Control: no-store, must-revalidate`
> (verified via `curl -I`) — fails "efficient cache policy".
> Likely Next.js dev-mode default leaking into our header config.

In dev mode this is expected (HMR needs fresh fetches). In production
+ behind a CDN, marketing pages should be `s-maxage=300, stale-while-revalidate=600`
or similar so the edge caches them.

## Goals

1. Set `Cache-Control: public, s-maxage=300, stale-while-revalidate=600` on marketing routes (/landing, /pricing, /customers, /compare, /changelog, /docs, /privacy, /terms, /security).
2. Authed routes stay `no-store` (correct).
3. API routes stay `no-store` (correct).

## Steps

1. Extend `web/next.config.ts` `async headers()` to add a marketing-paths matcher.
2. Add Cache-Control directives matching the production CDN strategy.
3. Verify in dev that the headers fire on `curl -I http://localhost:3000/landing` only when `process.env.NODE_ENV === "production"`.
4. Document the cache strategy in a comment.

## may-touch

- `web/next.config.ts` (extend the `headers()` block)
- Optional: `web/middleware.ts` (only if header injection from middleware is preferred over next.config)

## must-NOT-touch

- Authed routes (must stay `no-store`)
- /api routes (must stay `no-store`)
- Existing security headers (CSP/HSTS etc. — keep as-is)

## Acceptance criteria

- [ ] `curl -I` against /landing in production-mode returns `Cache-Control: public, s-maxage=300, stale-while-revalidate=600`.
- [ ] `curl -I` against /landing in dev-mode still returns no-store (so HMR works).
- [ ] `curl -I` against / (authed) returns no-store.
- [ ] Lighthouse "efficient cache policy" passes on /landing.
- [ ] `pnpm build` succeeds.

## References

- Audit: `docs/skycot/audit/perf/2026-05-20-wave-2-perf.md`
- Next.js headers config: `next.config.ts headers()` API
