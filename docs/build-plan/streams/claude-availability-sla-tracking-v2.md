# claude-availability-sla-tracking-v2 — availability/SLA tracking from audit_record kind=http.request

**Owner**: claude. **Depends on**: shipped `routes_request_audit.rs` emitting `AuditKind::HttpRequest`; `claude-error-rate-dashboard-v2` MERGED (shares the `audit_record_query` extension).
**Authored against**: origin/main `94370ca5` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-availability-sla-tracking-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized after audit-3 INTENT-DRIFT finding).** V1 (`claude-availability-sla-tracking`) was DEEP-DRIFT — assumed a non-existent `gateway_request_log` table. V2 queries `audit_record WHERE kind='http.request'`.

## Scope

Availability/SLA tracking: percent of `audit_record kind='http.request'` rows in a time window whose `status_code < 500`, plus p50/p95/p99 latency over the same window.

Ship:

1. **Extend `argo-store::audit_record_query`** (from sibling brief `claude-error-rate-dashboard-v2`) with:
   - `pub async fn availability_in_window(store, start_ts, end_ts) -> StoreResult<AvailabilitySnapshot>` returning `(total, success_count, success_pct)`.
   - `pub async fn latency_percentiles_in_window(store, start_ts, end_ts) -> StoreResult<LatencyPercentiles>` returning `(p50, p95, p99)` in microseconds. SQLite has no PERCENTILE_CONT — use `ORDER BY duration_ms DESC LIMIT 1 OFFSET ...` approach.

2. **NEW `argo sla report` CLI** that takes `--window <duration>` (default 24h) + optional `--tenant <id>` filter. Renders availability % + latency percentiles.

3. **NEW `GET /api/v1/dashboards/sla` gateway route** for the browser/SPA surface.

**Not in scope**: SLO breach alerting (compose with `handlers/alerts.rs::AlertKind::ErrorRate` or a new `LatencyP99` rule — separate brief); historical retention (data is bounded by audit_record retention policy); per-route SLA breakdown (v1 is gateway-wide).

## Files you may touch

- `crates/argo-store/src/audit_record_query.rs` (extended via sibling brief; THIS brief adds `availability_in_window` + `latency_percentiles_in_window`). ≤120 modified lines.
- `<NEW: crates/argo-cli/src/handlers/sla_report.rs>` (≤150 LOC).
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: ONE `sla:` subcommand. ≤10 modified lines.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` — narrowly: route `/api/v1/dashboards/sla`. ≤10 modified lines.

## Files you must NOT touch

- `routes_request_audit.rs` — emission locked.
- `argo-audit::AuditSink` — locked.
- The other dashboard (`claude-error-rate-dashboard-v2`) — they share `audit_record_query` but ship independently.

## Acceptance criteria

- [ ] `availability_in_window` correctly returns `success_pct` (rows with status_code < 500).
- [ ] `latency_percentiles_in_window` returns sensible p50/p95/p99 for a seeded window.
- [ ] `argo sla report --window 24h` prints a table.
- [ ] `GET /api/v1/dashboards/sla?window=24h` returns JSON envelope.
- [ ] No new SQL table created.
- [ ] 5 unit tests with a seeded fixture (mix of 200/500/timeout statuses, varying latencies).
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-availability-sla-tracking-v2`
- PR title: `cli+gateway: SLA tracking over audit_record (claude-availability-sla-tracking-v2)`

## Notes for the implementer

- SQLite p99 idiom: `SELECT duration_ms FROM audit_record WHERE kind='http.request' AND received_at BETWEEN ? AND ? ORDER BY duration_ms DESC LIMIT 1 OFFSET CAST((total * 0.01) AS INTEGER) - 1`.
- If pre-aggregation is needed for performance, that's a separate brief.
- Like the sibling error-rate brief: explicitly NO new logging table.
