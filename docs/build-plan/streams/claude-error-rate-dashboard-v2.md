# claude-error-rate-dashboard-v2 — error-rate dashboard backed by audit_record kind=http.request

**Owner**: claude. **Depends on**: shipped `routes_request_audit.rs` emitting `AuditKind::HttpRequest`.
**Authored against**: origin/main `94370ca5` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-error-rate-dashboard-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized after audit-3 INTENT-DRIFT finding).** V1 (`claude-error-rate-dashboard`) was DEEP-DRIFT — assumed a non-existent `gateway_request_log` table. The dropped precursor (`claude-gateway-request-log-schema`) would have built a parallel logging surface. V2 queries the **already-shipped** `audit_record` table filtered to `kind = 'http.request'`.

## Scope

The error-rate dashboard is a TUI / CLI / gateway-route surface that aggregates `status_code` over a time window from `audit_record WHERE kind = 'http.request'`. The data is already being emitted (`routes_request_audit.rs:95` writes the row with method, path, tenant_id, status, duration_ms, principal_id in `payload_json`).

Ship:

1. **NEW `argo-store::audit_record_query` extension** (or a new module) with `pub async fn error_rate_in_window(store, start_ts, end_ts) -> StoreResult<ErrorRateSnapshot>` returning total count + error count + per-status-code histogram. Queries `audit_record WHERE kind='http.request' AND received_at BETWEEN ? AND ?` + parses `payload_json::status_code`.

2. **NEW `argo errors dashboard` CLI** that takes `--window <duration>` (default 1h) + optional `--tenant <id>`/`--route <pattern>` filters. Renders a table.

3. **NEW `GET /api/v1/dashboards/error-rate` gateway route** for the browser/SPA surface.

4. **Composability**: the existing `handlers/alerts.rs::AlertKind::ErrorRate` rule predicate already monitors error-rate. This brief is the **read-side dashboard**; the alerts.rs surface is the **alerting predicate**. They share the same source data (audit_record). No parallel system.

**Not in scope**: indexing audit_record for query performance (separate brief if SLOs warrant); breakdowns beyond status_code; pre-aggregating into a separate dashboard table.

## Files you may touch

- `<NEW: crates/argo-store/src/audit_record_query.rs>` (≤200 LOC) — extension queries for HttpRequest rows.
- `<NEW: crates/argo-cli/src/handlers/errors_dashboard.rs>` (≤150 LOC) — CLI + gateway route handler.
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: ONE `errors:` subcommand. ≤10 modified lines.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` — narrowly: route `/api/v1/dashboards/error-rate`. ≤10 modified lines.

## Files you must NOT touch

- `routes_request_audit.rs` — emission surface is locked.
- `argo-audit::AuditSink` / `audit_record` schema — locked.
- `handlers/alerts.rs::AlertKind::ErrorRate` — that's the alerting predicate; this brief is the read-side dashboard. They share data but don't share code.

## Acceptance criteria

- [ ] `error_rate_in_window` returns `(total, errors, per_status_histogram)`.
- [ ] `argo errors dashboard --window 1h` prints a table.
- [ ] `GET /api/v1/dashboards/error-rate?window=1h` returns JSON envelope.
- [ ] No new SQL table created.
- [ ] 5 unit tests against an in-memory store with seeded `audit_record` rows.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-error-rate-dashboard-v2`
- PR title: `cli+gateway: error-rate dashboard over audit_record (claude-error-rate-dashboard-v2)`

## Notes for the implementer

- This brief explicitly does NOT create a parallel logging table. Use audit_record. Per CLAUDE.md "subtract before adding".
- If you find audit_record's payload_json shape is hard to query at scale (e.g. `SELECT json_extract(payload_json, '$.status_code')` is slow), that's a **separate brief** — `mission-audit-record-indexed-views` or similar — to either add indexes or build a materialized view. Don't fix it in this brief.
