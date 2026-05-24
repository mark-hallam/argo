# Round 49 · Gateway `/api/v1/verticals` discovery surface — confirm round 48 sibling-gap

- **Date**: 2026-05-24 (UTC) · Melbourne 2026-05-25
- **Round**: 49
- **Pillar / Claim**: Claim 3 (Platform builds workers) — vertical discovery is load-bearing for `/onboarding` step-4 + `/workflows` catalog · Claim 6 (Multi-tenant) — vertical list is tenant-public so CORS surface matters
- **Verdict**: 🟡 **PARTIAL** — round 48's `mission-gateway-add-api-v1-verticals-cors-mounted-discovery` brief candidate is **architecturally CONFIRMED** by direct gateway probe. SPA root `/verticals` returns the Next.js page (HTML), not JSON — this is the actual root-cause of the `apiGet("/verticals")` failure mode #2247 just routed around.
- **Severity**: P2 (web client has a working workaround in #2247; the gateway gap is the "right" fix)

## Setup

Live gateway probes against the running argo gateway at `:7400` (uptime ~17.9h, schema v80):

```bash
curl -sf http://127.0.0.1:7400/health                                   # → {"status":"ok","version":"0.0.1","schema_version":80,...}
curl -s  http://127.0.0.1:7400/api/v1/verticals                         # body check
curl -s  -X OPTIONS http://127.0.0.1:7400/api/v1/verticals -H "Origin: http://localhost:3000" -i
curl -s  -X OPTIONS http://127.0.0.1:7400/verticals -H "Origin: http://localhost:3000" -i
curl -s  http://127.0.0.1:7400/verticals -i
curl -s  http://127.0.0.1:7400/api/v1/verticals/sales-sdr/workflows
```

## Observed

| Path | Method | Status | Body / headers |
|---|---|---|---|
| `/api/v1/verticals` | GET | 200 | `{"error":"not found"}` — **gateway uniform-error envelope; the collection route is NOT mounted** |
| `/api/v1/verticals` | OPTIONS | 204 | full CORS preflight headers present (`Access-Control-Allow-Origin: http://localhost:3000`, `Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS`, `Allow-Headers: Content-Type, Authorization, X-Argo-Principal, X-Idempotency-Key, X-Argo-Tenant-Id, X-CSRF-Token`, `Allow-Credentials: true`, `Max-Age: 86400`, `Vary: Origin`) — proves the `/api/v1/*` CORS handler matches but the underlying GET route is missing |
| `/verticals` | GET | 200 | 24395-byte HTML (Next.js SPA page bundle) — this is the SPA route, not a gateway API |
| `/api/v1/verticals/sales-sdr/workflows` | GET | 200 | `{"error":{"code":"unauthorized","message":"X-Argo-Principal header is required"}}` — route IS mounted, requires auth |

## Anti-theater check

The PR #2247 web-side fix (verified r48) walks `SAMPLE_WORKFLOW_SUMMARIES` to derive a vertical-id list, then per-vertical fans-out against the per-vertical workflows route — bypassing the missing collection route. That works for the workflows catalog. But:

1. The `web/lib/api/verticals.ts:202` `fetchVerticals()` call (used by `app/(app)/verticals/page.tsx` via `use-verticals.ts:21`) **still hits the same dead `apiGet("/verticals")` path** that #2247 just routed around for `/workflows`. R48 surfaced this as a sibling-gap; r49 confirms by live probe that there is no `/api/v1/verticals` collection-route, only the per-vertical descendant.
2. The 204 OPTIONS preflight on `/api/v1/verticals` is *misleadingly green* — it proves the CORS handler matches, NOT that a real GET handler exists. R48's reported `OPTIONS /verticals → 404` failure mode applied to the SPA root-path (different layer); on the API path the preflight returns 204 but the GET 200-with-error envelope is still a dead-end.
3. The static `SAMPLE_WORKFLOW_SUMMARIES` becomes a *de-facto vertical registry* on the web side. That couples the SPA catalog to a hardcoded list and silently drifts whenever a new vertical ships to the gateway. Single source of truth would be a gateway-side `/api/v1/verticals` collection-route returning ids + display metadata.

This isn't theater on #2247's side — the brief was correctly scoped web-only / workflows-only per its may-touch. The architecture gap is **pre-existing** on `origin/main` and surfaced by the anti-theater verify, not introduced by the fix.

## Gap / follow-up brief candidates (carried from r48, now backed by live-probe evidence)

**(a) `fix-web-verticals-route-cors-preflight-symmetric-fix` (P1 if `/verticals` is in v1.0 demo MP4 surface, else P2; ~50-100 LOC web)**
- Apply #2247's SAMPLE-derived seed-fallback recipe to `fetchVerticals()`.
- Symmetric vitest pin mirroring `never hits /verticals` from #2247's test pack.
- Demo-blocker shape only IF the operator's demo MP4 runbook walks `/verticals` (recommend operator confirm before P0 promote).

**(b) `mission-gateway-add-api-v1-verticals-cors-mounted-discovery` (P2; ~30-60 LOC Rust)**
- Mount `GET /api/v1/verticals` returning `[{id, display_name, tags, ...}]` from `argo-verticals` registry.
- Tenant-public route (no `X-Argo-Principal` required) so the SPA can call pre-signin.
- Vitest gateway integration test + curl probe in `docs/skycot/operator/`.
- This is the architecturally clean root-cause fix; (a) is the band-aid that lets the SPA ship without the gateway change.

Note: (a) and (b) are NOT redundant — (a) gives near-term demo-MP4 safety while the gateway team owns (b). Conductor decides whether to ship both or just (b) given the v1.0 ship-gate pressure.

**Carry forward unchanged from r47/r48 (re-surface):**
- (c) `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (P1, BLOCKED on missing stream file) — author the stream file next conductor tick.
- (d) `ops-evaluator-skill-md-update-from-runbook` (P2 operator-actionable on non-repo file) — **14th-consecutive round** unactioned, still operator-only.
- (e) `mission-agent-runner-wire-process-registry-per-session` (P1, F1 sibling of #2117 triad, open since r41).
