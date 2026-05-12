# Mission load-bearing wire — SIEM integration (Splunk / ELK / Datadog)

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #227. Enterprise tenants want every audit-chain row + every gateway request-audit row + every security event shipped to their SIEM in real time. Today: only operators with custom log-shipping setups see it.

## Scope

NEW `argo-siem` crate exporting `SiemSink` trait + 3 concrete sinks:
1. **`SplunkHecSink`** — HTTP Event Collector at `https://<host>/services/collector` with `Authorization: Splunk <token>`. JSON line per event.
2. **`ElasticSink`** — Elasticsearch `_bulk` endpoint at `https://<host>:9200/_bulk`. Index name templatable.
3. **`DatadogLogsSink`** — `https://http-intake.logs.datadoghq.com/api/v2/logs` with `DD-API-KEY` header.

Wiring: a periodic dispatcher (mirror `sweepers.rs` pattern) tails `audit_record` from a cursor, batches by sink-specific size limit, ships with retry-on-5xx + exponential backoff. Cursor persists to a new `siem_dispatch_cursor` row in `argo-store` so a crash doesn't re-ship.

Config: `[siem] enabled = true; backend = "splunk"|"elastic"|"datadog"; endpoint = "..."; token = "..."` in `argo.toml`.

This is NOT in scope: custom field mapping (each sink uses a canonical JSON shape — operators map via SIEM-side); failed-events DLQ (errors drop after exponential-backoff exhaustion + audit-row written); per-event filtering (ship all-or-nothing in v1).

## Files you may touch

- `crates/argo-siem/` (NEW crate, ≤ 1200 LOC total)
  - `Cargo.toml`, `src/lib.rs`, `src/sink.rs` (trait), `src/splunk.rs`, `src/elastic.rs`, `src/datadog.rs`, `src/dispatcher.rs` (tokio loop), `src/{splunk,elastic,datadog,dispatcher}_tests.rs`
- `crates/argo-store/migrations/NEXT_SLOT__siem_cursor.sql` (NEW, verify slot at claim time — likely 71+ per header pin)
- `crates/argo-cli/src/handlers/gateway/boot.rs` — spawn dispatcher under existing sweeper conditional
- `argo-config/src/sections.rs` — `[siem]` section parse
- `docs/skycot/RUNBOOK.md` — "SIEM integration" subsection

## Files you must NOT touch

- The audit-chain hash logic; existing audit-write paths
- Other connector crates (these are LOG sinks, not connectors)

## Acceptance criteria

- [ ] NEW `argo-siem` crate compiles standalone
- [ ] All 3 sinks ship to their respective endpoints in unit tests (wiremock)
- [ ] Cursor advances after successful batch; reverts on 5xx; retries with exp backoff
- [ ] Crash + restart resumes from persisted cursor without duplicates
- [ ] `[siem] enabled = false` (default) — no dispatcher spawned, no overhead
- [ ] Credentials redacted in Debug + logs
- [ ] Integration test: gated by `ARGO_SIEM_E2E=1`; ships a real event to a local mock HEC
- [ ] `cargo clippy -p argo-siem --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-siem-integration`
- PR title: `siem: Splunk/Elastic/Datadog dispatcher (mission-siem-integration)`

## Notes

- Splunk HEC, Elastic `_bulk`, and Datadog v2 are the 3 lowest-friction sinks; one of them covers ~90% of enterprise SIEMs. Operators with niche SIEMs (Sumo, ArcSight, etc.) can write a custom sink against the trait.
- Cursor design: `(last_seq, last_shipped_at)`. Resume reads from `last_seq + 1`. Idempotent via the audit row's `id` field — even if a batch ships twice (network blip), the SIEM dedupes on `id` if configured.
- The dispatcher is opt-in (default `enabled = false`). Default-on would surprise existing operators with their first connector bill.
