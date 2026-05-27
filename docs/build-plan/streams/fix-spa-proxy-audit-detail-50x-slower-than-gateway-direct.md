# fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/scene-6-claim-6-trust-and-approvals.md` F2 [P0]; corroborated by `scene-1-and-2-claim-1-and-2.md` F9 (compile-time blow-out).

🔴 **P0 — Gateway-direct `GET /api/v1/audit/<id>` returns 200 in 0.22s. Same request through SPA proxy (`http://localhost:3000/api/v1/audit/<id>`) takes 11.22s. SSR (Server-Side Render — the HTML the Next.js server returns before the React client takes over) of `/audit/action/<id>` hangs >60s. In demo recording, the operator clicks an audit row and waits 10+ seconds for the detail to populate; that dead-air is camera-visible.**

## Root cause

`web/next.config.ts:30-32` has a one-line rewrite `/api/v1/:path*` → gateway. Under turbopack dev mode, the rewrite for `/api/v1/audit/<id>` is 50× slower than gateway-direct, and the SSR of `/audit/action/<id>` times out at 60s. `/audit` (list view) returns 200 in 3.8s — so the latency isn't generic turbopack cold-compile, it's specific to the detail route.

Two hypotheses, both worth investigating:

1. **Synchronous .next chunk regeneration loop on each `/api/v1/audit/<id>` rewrite.** Turbopack may be re-bundling on every rewrite because the API route filename pattern triggers HMR (Hot Module Replacement — Next.js dev-mode auto-reload) invalidation. Verify by warming the route once then timing a second call.
2. **Server component triggering an extra fetch.** `web/app/(app)/audit/action/[actionId]/page.tsx` is `"use client"` but a parent layout may have a server-side data hook that fetches and waits. The 13-minute first-paint observed in scene-1-2 F9 corroborates: 203s compile + 784s first response suggests a layer doing an extra round-trip while compiling.

Compounding: the rewrite is dev-only pain (Next.js production builds don't compile per-request). But the v1.0 demo MP4 is recorded against `pnpm dev`, so this IS a recording blocker.

Evidence:
- `curl -m 10 -o /dev/null -w "%{time_total}\n" 'http://127.0.0.1:7400/api/v1/audit/019e6666b1dd7ef3971b47289dc1a76e'` → ~0.22s
- `curl -m 20 -o /dev/null -w "%{time_total}\n" 'http://localhost:3000/api/v1/audit/019e6666b1dd7ef3971b47289dc1a76e'` → 11.22s
- `curl -m 60 'http://localhost:3000/audit/action/019e6666b1dd7ef3971b47289dc1a76e'` → times out at 60s
- preview_logs: `Compiling /audit/action/[actionId] ... ✓ Compiled in 203s`, `GET /audit/action/... 200 in 784813ms`

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F2 [P0] — SPA proxy for `/api/v1/audit/<id>` is 50× slower than gateway-direct; SSR of `/audit/action/<id>` times out
- scene-1-and-2-claim-1-and-2 F9 [P1] — Cold-start dev compile time for `/audit/action/[actionId]` is 203s; first request takes 784s

## Fix recipe

1. **Confirm `/audit/action/[actionId]/page.tsx` is genuinely client-only.** If it's `"use client"`, walk up the layout tree: `web/app/(app)/audit/action/layout.tsx`, `web/app/(app)/audit/layout.tsx`, `web/app/(app)/layout.tsx`. Any of these doing a server-side `await fetch(...)` will block SSR.
2. **If a parent layout has a blocking fetch**, move it into a client-side `useQuery` in the page component, or guard it with `dynamic = 'force-dynamic'` + `revalidate = 0` so Next.js doesn't try to prerender during compile.
3. **Profile turbopack rewrite cost.** Add timing instrumentation to `web/middleware.ts` (or wherever requests are matched) around the `/api/v1/audit/*` rewrite. If the rewrite alone takes >1s on warm cache, escalate to a Next.js bug filing — the dev-server team will need a repro.
4. **Add pre-warm step to demo-record runbook.** Update `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist to include `curl http://localhost:3000/audit/action/<known-id>` before pressing record, so the camera doesn't catch the cold-compile blank state.
5. **(Optional) Skip the rewrite for audit-detail by using a direct gateway URL** from the SPA — set a build-time constant `NEXT_PUBLIC_GATEWAY_DIRECT_URL=http://127.0.0.1:7400` and have `apiGet("/api/v1/audit/<id>")` route directly when in dev. Sidesteps the rewrite entirely. Document the CORS implications.

## Acceptance criteria

- [ ] Warm-load `/audit/action/<id>` from `pnpm dev` returns within 3s.
- [ ] SPA proxy `/api/v1/audit/<id>` latency on warm cache ≤ gateway-direct latency + 200ms.
- [ ] Pre-record runbook updated with `/audit/action/<id>` pre-warm step.
- [ ] No production build regression (verify with `pnpm build` + `pnpm start` against a known audit row).

## May-touch

- `web/app/(app)/audit/action/[actionId]/page.tsx` (verify pure client-side)
- `web/app/(app)/audit/action/layout.tsx` + `web/app/(app)/audit/layout.tsx` + `web/app/(app)/layout.tsx` (audit for server-side fetches)
- `web/next.config.ts` (rewrite review)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-warm checklist)
- `web/lib/api/audit-detail.ts` (optional direct-gateway URL)

## Out of scope

- Audit row content fabrication (covered by `fix-activity-row-audit-detail-synthesized-fixture-theatre` + `fix-audit-detail-render-empty-decision-chain-and-provenance-fallback`)
- Audit-feed self-traffic filter (covered by `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`)
- Turbopack panic in dev logs (Next.js bug, not app issue)

## Effort

M — diagnostic-heavy. ~2-4 hours to find the blocking-fetch layer, plus 30 min to add pre-warm step to runbook. P0 because the v1.0 demo MP4 records `/audit/action/<id>` cold and would catch 10+ seconds of dead air.
