# fix-spa-client-use-same-origin-gateway-rewrite

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-client-use-same-origin-gateway-rewrite`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F01 (P0) + F02 (P0 cascade).

🔴 **P0 — The SPA's typed `apiGet` / `apiPost` wrapper prepends the absolute gateway URL `http://127.0.0.1:7400` to every `/api/v1/*` request, which the browser blocks with `TypeError: Failed to fetch` because the gateway does not emit `Access-Control-Allow-Origin` for the page origin. Net effect: every in-app surface that depends on live data falls back to mock-offline; surfaces without a mock fallback (notably `/audit`) stick in a permanent loading state. Scene 6 of the v1.0 demo runbook drills into `/audit` and lands on an empty skeleton. Next.js already exposes a same-origin rewrite at `/api/v1/*` that proxies to the gateway and works fine — the SPA just isn't using it from the client.**

## Root cause

`web/lib/api/client.ts:80-81` defines `gatewayBase()` to return `NEXT_PUBLIC_GATEWAY_URL` (default `http://127.0.0.1:7400`) **unconditionally** — same value for both server-side render (where absolute URL is necessary because the Next.js process is the requester) and client-side browser requests (where the browser becomes the requester and crosses origins). Line 179 then prepends `gatewayBase()` to every API call. Browser-side: `fetch("http://127.0.0.1:7400/api/v1/audit")` is a cross-origin request; the gateway returns the response without an `Access-Control-Allow-Origin` header matching the page origin, so the browser rejects it.

The Next.js rewrite at `web/next.config.ts:30-32` is `{ source: "/api/v1/:path*", destination: "${gatewayUrl}/api/v1/:path*" }` — already shipped and working. Same-origin `fetch("/api/v1/audit")` from the browser returns 200 + the real audit rows.

The one-line fix: `gatewayBase()` returns `""` (empty string → same-origin) when called in a browser context, and the absolute URL only in SSR / route-handler contexts. A `typeof window !== "undefined"` guard does it.

This is the single largest cascade defect surfaced by the T0119 audit:
- `/audit` stuck in loading (F02) — root cause F01
- `/workers` deck falls through to SAMPLE_TEAM with the visible mock-offline pill (scene-1-2 F1) — same root cause
- `/skills` shows the SSR `0 skills` flash before mock-fallback paints 6 (scene-4-5 F6) — same root cause
- Every in-app surface effectively in mock-offline mode (whole-site TL;DR) — same root cause

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F01 [P0] — every cross-origin gateway call CORS-fails in the browser
- `docs/skycot/ux-audits/2026-05-27-0119/whole-site-mvp-claims-coverage.md` F02 [P0] — `/audit` page permanently stuck in `audit-loading` skeleton
- `docs/skycot/ux-audits/2026-05-27-0119/scene-1-and-2-claim-1-and-2.md` F1 [P0] (root cause beneath the visible mock-offline chip — fixing the absolute URL removes the trigger for the fallback path)
- `docs/skycot/ux-audits/2026-05-27-0119/scene-4-and-5-claim-4-and-5.md` F6 [P1] (SSR flash from `0 skills · 0 shown` to `6 skills · 6 shown` — same root cause; with same-origin rewrite the gateway responds on the SSR pass too)

## Fix recipe

1. Edit `web/lib/api/client.ts` `gatewayBase()` to branch on execution context:
   ```ts
   function gatewayBase(): string {
     if (typeof window !== "undefined") return "";
     return process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:7400";
   }
   ```
   - Server-side render and Next.js route handlers continue to use the absolute URL (they cross process boundaries; the Next process is the requester, not the browser).
   - Browser-side fetches now resolve to same-origin `/api/v1/*` which Next.js rewrites onto the gateway.
2. Add a vitest unit test that asserts `gatewayBase()` returns `""` when `window` is defined and the absolute URL otherwise.
3. Verify in DevTools: open `/audit` in Chrome → Network panel shows a request to `/api/v1/audit` (same-origin, 200 OK) — not to `http://127.0.0.1:7400/api/v1/audit`.
4. Sanity-check the SSR path is unchanged by hitting `curl http://127.0.0.1:3000/audit` and confirming the SSR'd HTML matches today's behavior (no regression in mock-offline fallback when gateway truly down).

## Acceptance criteria

- [ ] `fetch('/api/v1/audit')` from the browser console returns 200 + audit rows.
- [ ] `fetch('http://127.0.0.1:7400/api/v1/audit')` from the browser still fails — but the SPA no longer calls it.
- [ ] `/audit` page renders the audit feed (real rows when gateway has them, mock-offline fallback otherwise — but no permanent loading skeleton).
- [ ] `/workers` and `/skills` paint with live data when gateway has it, mock fallback only when the gateway truly 404s — no CORS-induced fallback.
- [ ] Vitest `web/lib/api/client.test.ts` covers both branches.
- [ ] No regression in SSR: `curl http://127.0.0.1:3000/audit` returns the same HTML shape as before.

## May-touch

- `web/lib/api/client.ts:80-81,179` (~5 LOC change)
- `web/lib/api/client.test.ts` (NEW or extended — ~30 LOC test)

## Out of scope

- Adding CORS headers to the gateway (alternative fix; the Next rewrite is already shipped and free — don't double-solve).
- The mock-offline fallback path itself (`web/lib/api/team.ts`, `web/lib/api/audit.ts` etc) — leave intact as a fallback for when the gateway truly is down.
- Removing `NEXT_PUBLIC_GATEWAY_URL` (still needed for SSR + route-handler contexts).

## Effort

XS — a single conditional in one function, plus a unit test. ~30 minutes including local verification. P0 because the cascade from this one bug wipes ~25 points off the whole-site orientation score and breaks Scene 6 of the v1.0 demo runbook outright.
