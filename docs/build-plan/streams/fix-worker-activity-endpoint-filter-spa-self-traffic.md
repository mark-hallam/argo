# fix-worker-activity-endpoint-filter-spa-self-traffic

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-activity-endpoint-filter-spa-self-traffic`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-1-and-2-claim-1-and-2.md` F8 [P1].

🟠 **P1 — `GET /api/v1/workers/<id>/activity` returns rows tagged `verb=worker.kpi_rolled` (recurring 5-min cron telemetry) and `verb=http.request` (SPA self-pings from `/workers`, `/audit`). Zero rows show an actual agent turn. Scene 2's "click into Mariah's activity, see what she's been doing" narration shows the camera "worker.kpi_rolled · http.request · http.request" — not work the worker did. This is the per-worker analogue of the global audit feed self-traffic finding, but the existing brief `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` only addresses the GLOBAL `/api/v1/audit` endpoint, NOT the per-worker activity endpoint.**

## Root cause

The gateway audits every HTTP request to itself, then also fires periodic cron-style KPI roll-ups per worker. Both kinds get tagged into the per-worker activity stream:

```bash
curl -sS http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/activity \
  | jq '.items[].verb' | sort -u
# → "http.request"
# → "worker.kpi_rolled"
```

Neither `http.request` nor `worker.kpi_rolled` is an agent turn. They're system-internal events that should either be filtered or visually bucketed.

The per-worker activity endpoint lives at `crates/argo-gateway/.../routes_workers_activity*.rs` (or sibling under `crates/argo-cli/src/handlers/gateway/`). The SPA-side reader is `web/lib/api/worker-activity.ts` which renders rows literally onto the Activity tab of the worker-detail page (`web/app/(app)/workers/[workerId]/activity/page.tsx`).

The existing brief `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` covers the GLOBAL `/api/v1/audit` endpoint with a default-filter pattern (exclude `verb in {http.request, worker.kpi_rolled, audit.detail.viewed, audit.replay}` unless `?include_meta=1`). The same pattern needs to be applied to the per-worker endpoint — but the route is different and the filter set is slightly different (worker-scoped `kpi_rolled` is the dominant noise here, not `audit.detail.viewed`).

Scene 2's runbook beat: "the operator clicks Activity on Mariah's profile, watches the row stream populate." Today the row stream is 100% system telemetry. Function-side defect — the camera reads the verb strings.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F8 [P1] — Worker Activity tab gateway endpoint returns SPA self-traffic (http.request / worker.kpi_rolled), not real agent turns

## Fix recipe

Two paths; (a) is the demo-gate fix, (b) is the post-MVP polish.

### Path 1 — Filter at the per-worker list endpoint (preferred for demo)

1. In the per-worker activity handler (`crates/argo-gateway/src/.../routes_workers_activity.rs` — verify exact path with `rg "fn workers_activity" crates/`), add a default filter:
   - Exclude rows where `verb IN ('http.request', 'worker.kpi_rolled', 'audit.detail.viewed', 'audit.replay', 'audit.list.viewed')` UNLESS the query string carries `?include_meta=1`.
2. Mirror the same constant-list as `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request` to avoid drift — define `METADATA_VERBS: &[&str]` in `crates/argo-audit/src/lib.rs` (or a sibling module) and import both list endpoints from it.

### Path 2 — Bucket meta-rows in the SPA (defence-in-depth + post-MVP)

3. `web/lib/api/worker-activity.ts` adds a `kind: "agent_turn" | "system_meta"` discriminant derived from `verb`.
4. `web/app/(app)/workers/[workerId]/activity/page.tsx` renders agent-turn rows in the main timeline and collapses system-meta rows under a "System events" expandable section (`<details>` block, default collapsed).

### Test

5. Vitest pin at `web/lib/api/worker-activity.test.ts`: feed a 10-row fixture with 7 system-meta + 3 agent-turn verbs → assert main render shows 3 rows, expandable section shows 7.
6. Backend test in `crates/argo-gateway/.../tests/`: insert a row with `verb=worker.kpi_rolled`, call `/api/v1/workers/<id>/activity` (no query) → assert response excludes the row; call with `?include_meta=1` → assert response includes it.

## Acceptance criteria

- [ ] `GET /api/v1/workers/<id>/activity` returns ZERO rows tagged `verb=http.request` or `verb=worker.kpi_rolled` by default.
- [ ] `GET /api/v1/workers/<id>/activity?include_meta=1` returns all rows including system-meta.
- [ ] The SPA Activity tab on `/workers/<id>/activity` shows 0 rows (empty state) for a worker with only system-meta events, NOT a list of "http.request · http.request · …".
- [ ] Vitest + cargo tests pin both behaviors.
- [ ] The constant-list `METADATA_VERBS` lives in one place (no copy-paste between global audit list and per-worker activity).

## May-touch

- `crates/argo-gateway/src/.../routes_workers_activity.rs` (filter logic — verify exact path)
- `crates/argo-audit/src/lib.rs` or sibling (shared `METADATA_VERBS` constant)
- `web/lib/api/worker-activity.ts` (defence-in-depth, optional)
- `web/app/(app)/workers/[workerId]/activity/page.tsx` (bucket render, optional)
- Cargo tests for the gateway route
- Vitest for the SPA path (if Path 2 lands)

## Out of scope

- Global `/api/v1/audit` feed self-traffic filter (covered by `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`).
- Fabricated provenance on placeholder action ids (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`).
- Hash-chain anchor surfacing (covered by `fix-audit-feed-include-self-hash-in-envelope`).
- Demo seed running real agent turns (covered by `fix-demo-seed-emit-worker-hired-audit-rows`).

## Effort

~80-150 LOC across gateway route + shared constant + tests (Path 1 only); +50 LOC for SPA bucket render (Path 2). ~2-3 hours. P1 because Scene 2 narrates clicking into worker activity; the camera reads the verb strings directly.
