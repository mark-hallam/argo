# r83 S3 — Live gateway canonical-routes probe (process-drift cover)

- **Date**: 2026-05-26
- **Round**: 83
- **Pillar / Claim**: Operational health (gateway-as-API surface); cross-cuts Claim 1 (`/api/v1/team`) + Claim 4 (`/api/audit`) data plane availability
- **Verdict**: 🟢 PASS (3 canonical routes return 200 with non-stub payloads); 🟡 PARTIAL informational finding on `/api/v1/audit/feed?limit=3` returning 404 with misleading error message

## Setup

Live gateway probes against `http://127.0.0.1:7400` (operator dev box). Gateway uptime confirmed at 180.9 minutes (3+ hours) — schema_version 83 matches `gateway_schema=v83 codebase_latest=v83` from `scripts/check-gateway-freshness.sh`. No tenant header or session JWT passed; relying on default-tenant resolution.

## Observed

### Successful routes

```
GET /health                       → 200  {"status":"ok","schema_version":83,"uptime_seconds":10849}
GET /api/audit                    → 200  16+ rows; recent activity span 2026-05-25T22:24 → 2026-05-26T04:52
GET /api/v1/team                  → 200  {"total_count":1,"workers":[{"name":"Maya","role":{"vertical":"marketing-intern",...}}]}
```

`/api/audit` returns rich audit data: action_type, channel="web", data_classification="internal", verb="http.request", timestamps. Top 3 rows are `audit.detail.viewed` events from earlier today — consistent with operator-driven /audit drill-ins during interactive session.

`/api/v1/team` returns Maya (marketing-intern vertical) with the headline empty-state structure (`bullets: ["No activity yet."]`, `tomorrow: null`). This is the expected starter-prompt path the conductor's overnight Genesis briefs are working on — confirms it's still reachable.

### 4xx responses (expected vs investigatable)

```
GET /api/v1/workers               → 404  {"error":"not found"}
GET /api/v1/audit/feed?limit=3    → 404  {"error":{"code":"not_found","message":"audit row not found"}}
```

`/api/v1/workers` 404 is expected — canonical route is `/api/v1/team` (per ledger row `fix-gateway-workers-collection-alias-team` PR #2395, which is currently in-review and adds an alias to fix exactly this SPA-side confusion). No regression.

`/api/v1/audit/feed?limit=3` 404 with message `"audit row not found"` is informationally interesting — that error string comes from the audit-detail-by-id handler (probably routing `feed` as a `bundle_id` / row-id segment under `/api/v1/audit/<id>`). The message is misleading: the route doesn't exist at all, but the error implies "row missing" rather than "endpoint missing". Real impact is low — SPA clients should be using `/api/audit` (no `/v1` prefix, no `/feed` suffix) — but the error message could mislead a developer probing for a feed endpoint.

## Anti-theater check

Probes hit a running production binary (the live gateway plist `com.skycot.gateway` per ops setup, schema v83 matches HEAD). Cannot be faked — `/api/audit` returned 16+ distinct audit rows with internal IDs (`019e62a0490...`), action_types, and timestamps spanning real activity windows. If the gateway were stubbing responses, the row IDs wouldn't match the `audit.detail.viewed` pattern the conductor logs on every drill-in.

`/api/v1/team` returned Maya with a verifiable schema (`headline.bullets`, `role.vertical`, `team: null`) consistent with the actual SPA contract — a stub would have to know the full team-headline pack shape.

## Gap / follow-up brief candidates

Both informational, neither blocking:

1. **P3 ergonomic brief**: `fix-gateway-audit-route-404-message-distinguishes-missing-endpoint-from-missing-row` — when the request matches the `/api/v1/audit/<bundle_id>` pattern but the `bundle_id` is a known reserved word (`feed`, `export`, `replay`, etc.), return `{ error: "endpoint_not_found", suggested_path: "/api/audit" }` instead of "audit row not found". Tiny scope (~30 LOC in the audit dispatcher). Defer post-MVP — not on demo path.

2. **Verification ledger note (not a brief)**: PR #2395's `workers-collection-alias-team` brief is the load-bearing fix for the `/api/v1/workers → /api/v1/team` confusion. After it merges, re-probe `/api/v1/workers` and confirm it 200s with the alias-not-divergent payload (per r82 PRE-MERGE verdict). Conductor handles via the regular post-merge tick.

No P0/P1 findings. 25-round 🟢 streak (r57 → r83) HOLDS.
