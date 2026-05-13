# claude-gateway-request-log-schema — schema + insert for `gateway_request_log` table

**Owner**: claude. **Depends on**: nothing (precursor).
**Authored against**: origin/main `b77b7474` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-gateway-request-log-schema`.

## Scope

Precursor brief for 2 DEEP-DRIFT briefs (`claude-error-rate-dashboard`, `claude-availability-sla-tracking`) that both cite a non-existent `gateway_request_log` SQL table as their source. Today the gateway emits audit rows for some request outcomes (auth failures, rate-limit denials) via `argo-audit::AuditSink`, but there is no comprehensive per-request log that captures request_id + method + path + status + latency + tenant. The 2 downstream briefs depend on this surface.

Ship the schema + the per-request insert hook + a thin query API:

1. **Migration** (next free slot — current free is 73+; verify with `ls crates/argo-store/migrations/`): `<slot>__gateway_request_log.sql`:
   ```sql
   CREATE TABLE gateway_request_log (
       request_id TEXT NOT NULL PRIMARY KEY,
       received_at INTEGER NOT NULL,
       tenant_id TEXT,
       method TEXT NOT NULL,
       path TEXT NOT NULL,
       status_code INTEGER NOT NULL,
       latency_micros INTEGER NOT NULL,
       source_ip TEXT,
       user_agent TEXT,
       error_kind TEXT
   ) STRICT;
   CREATE INDEX gateway_request_log_received_at_idx ON gateway_request_log(received_at);
   CREATE INDEX gateway_request_log_tenant_idx ON gateway_request_log(tenant_id);
   CREATE INDEX gateway_request_log_path_status_idx ON gateway_request_log(path, status_code);
   ```

2. **Insert hook**: `argo-store::gateway_request_log` module with `pub async fn insert(store: &SessionStore, row: GatewayRequestLogRow) -> StoreResult<()>`. Pure async insert; the gateway's per-request middleware will call this fire-and-forget after the response is sent (low-priority best-effort, must NOT block the response path).

3. **Query API**: a small set of read-only helpers (~3 functions) that the 2 downstream briefs need:
   - `pub async fn count_in_window(store: &SessionStore, start_ts: i64, end_ts: i64) -> StoreResult<u64>` — for SLA tracking
   - `pub async fn count_errors_in_window(store: &SessionStore, start_ts: i64, end_ts: i64) -> StoreResult<u64>` — for error-rate dashboard
   - `pub async fn p95_latency_in_window(store: &SessionStore, start_ts: i64, end_ts: i64) -> StoreResult<u64>` — for SLA tracking; SQLite has no PERCENTILE_CONT so use `ORDER BY latency_micros DESC LIMIT <5% of count> OFFSET <5% of count - 1>` or compute via subquery

4. **Wire in gateway boot**: NO wire-up in this brief — that's `mission-gateway-request-log-emit` (a follow-up brief, not authored here). This brief ships only the schema + store helpers; emission is deferred.

**Not in scope**: emitting the rows (separate brief); retention/archival (separate brief — log will grow unbounded until archived); per-tenant access controls (separate brief); building the dashboards or SLA tracker.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `<NEW: crates/argo-store/migrations/<slot>__gateway_request_log.sql>` — table + 3 indexes.
- `crates/argo-store/src/migrations/registry.rs` (post-#1550 split) — narrowly scoped: add ONE `Migration { version: <slot>, up: include_str!("../../migrations/<slot>__gateway_request_log.sql"), down: ... }` literal to the registry. Also need a `<slot>__gateway_request_log_down.sql` if the registry pattern requires both up + down files (check by reading the existing `Migration` literal shape).
- `<NEW: crates/argo-store/src/gateway_request_log.rs>` (≤200 LOC) — `GatewayRequestLogRow` struct + `insert` + `count_in_window` + `count_errors_in_window` + `p95_latency_in_window`. Module-local tests covering: round-trip insert+read, window-filter correctness, p95 computation matches a known fixture.
- `crates/argo-store/src/lib.rs` — narrowly scoped: add ONE `pub mod gateway_request_log;` line + `pub use gateway_request_log::{GatewayRequestLogRow, insert, count_in_window, count_errors_in_window, p95_latency_in_window};`. ≤5 modified lines.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/**` — emission wire-up is a follow-up brief.
- Any audit-sink code — this is a separate per-request log, complementary to (not replacing) audit rows.
- `argo-store::SessionStore` struct — this brief uses the existing pool via `&SessionStore`; no struct changes needed.

## Acceptance criteria

- [ ] Migration file exists; slot ≥73 (verified clean at claim time).
- [ ] Schema includes: request_id (PK), received_at, tenant_id, method, path, status_code, latency_micros, source_ip, user_agent, error_kind.
- [ ] 3 indexes: (received_at), (tenant_id), (path, status_code).
- [ ] `insert(store, row)` round-trips correctly (insert + select round-trip test).
- [ ] `count_in_window` + `count_errors_in_window` + `p95_latency_in_window` all have unit-test fixtures.
- [ ] Migration registered in `registry.rs` with correct slot, additive only (no ALTER on existing tables).
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-store gateway_request_log` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-gateway-request-log-schema`
- PR title: `store: gateway_request_log table + query helpers (claude-gateway-request-log-schema)`

## Notes for the implementer

- The SQLite p95 computation is the trickiest part. Two acceptable approaches: (a) sort-and-offset (`SELECT latency_micros FROM gateway_request_log WHERE received_at BETWEEN ? AND ? ORDER BY latency_micros DESC LIMIT 1 OFFSET CAST((SELECT COUNT(*) FROM ...) * 0.05 AS INTEGER) - 1`), (b) approximate via `NTILE(20)` window function. Pick (a) for v1 — exact, simpler.
- The schema is **additive only** per the additive-migrations invariant (CLAUDE.md). New table; no ALTER.
- `request_id` should be a UUID-v7 minted at request entry (the gateway's middleware will provide it on emit-wire-up; this brief just declares the column).
- `error_kind` is nullable; populated only for non-2xx responses (e.g. `"rate_limit"`, `"auth_denied"`, `"timeout"`).
- Unblocks 2 DEEP-DRIFT briefs once landed: `claude-error-rate-dashboard` + `claude-availability-sla-tracking`.
- Retention is deliberately out-of-scope; the table will grow until a future `mission-gateway-request-log-retention` brief lands. Operators should be aware.
