# mission-prometheus-exporter-openmetrics-route — parallel `/openmetrics` route + `default_registry()`

**Owner**: claude. **Depends on**: existing `argo-telemetry::TelemetrySink` (shipped); existing custom `/metrics` handler (shipped).
**Authored against**: origin/main `39b92666` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prometheus-exporter-openmetrics-route`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized Cluster D #6).** V1 (`mission-prometheus-exporter-default-registry`) assumed `default_registry()` existed in argo-telemetry (it doesn't) and would have replaced the existing 892-LOC custom `/metrics` handler (which already exposes 18 store-derived metrics operators are using). V2 keeps existing `/metrics` untouched, adds a **parallel `/openmetrics` route** for the 8 brief-specified HTTP-flow / provider / runtime metrics, and adds `default_registry()` to argo-telemetry so future metrics auto-wire.

## Scope

**Today**: `/metrics` at `boot.rs:220-221` serves 18 store-derived metrics via 892 LOC of hand-coded SQL. Working well for what it covers. But the 8 operationally-critical brief-specified metrics (gateway-request histogram, provider breaker state, per-provider cost labels, migration version gauge) are missing.

**This brief**:

1. **Add `default_registry()` to argo-telemetry** — a process-global `Arc<prometheus::Registry>` accessible by any crate that wants to emit a metric. NEW `crates/argo-telemetry/src/prometheus_registry.rs` with `pub fn default_registry() -> Arc<Registry>` + a `register_counter` / `register_histogram` / `register_gauge` convenience surface.

2. **Add `/openmetrics` parallel route** — NEW `crates/argo-cli/src/handlers/gateway/routes_openmetrics.rs` that calls `prometheus::TextEncoder::encode` over `default_registry()`. Wired in `boot.rs` after the existing `/metrics` short-circuit. Public-GET allowlist.

3. **Wire the 8 brief-specified metrics** at their emission sites:
   - `gateway_request_total{method,path,status}` — increment in the request-dispatch path (after route resolution, before response write).
   - `gateway_request_duration_seconds{method,path}` — histogram observation in same site.
   - `gateway_rate_limit_throttle_total{scope}` — increment in rate-limit middleware (scope = ip|tenant|route).
   - `provider_call_total{provider,model,status}` — increment in `argo-provider::transport` dispatch.
   - `provider_breaker_state{provider}` — gauge updated in `breaker_observability.rs` on state transition.
   - `provider_cost_micros_total{provider,model}` — increment after a billed call resolves.
   - `audit_chain_tail_seq` — gauge updated by audit sink on append.
   - `store_migration_chain_version` — gauge set at boot from `EXPECTED_SCHEMA_VERSION`.

4. **Existing `/metrics` UNCHANGED** — operators relying on the 18 store-derived metrics continue working without migration.

5. **Operator runbook update** in `docs/skycot/runbook/observability.md` explaining the two endpoints + when to scrape which.

**Not in scope**: deprecating the existing custom `/metrics` (operator migration cost too high; keep both indefinitely or until a v2.x cleanup brief); per-tenant cardinality on every metric (cardinality budget — only `tenant_id` label on `provider_cost_micros_total` + `gateway_rate_limit_throttle_total{scope=tenant}`); push-gateway support; OpenTelemetry Metrics export.

## Files you may touch

- `<NEW: crates/argo-telemetry/src/prometheus_registry.rs>` (≤200 LOC) — `default_registry()` + register helpers + a small caching layer so multiple registrations of the same metric name don't panic.
- `<NEW: crates/argo-telemetry/src/prometheus_registry_tests.rs>` (≤200 LOC) — 5 unit tests: default-registry singleton, register-counter, register-histogram, double-registration-returns-existing, encode-text-output.
- `crates/argo-telemetry/src/lib.rs` — narrowly: `pub mod prometheus_registry;` + re-exports.
- `crates/argo-telemetry/Cargo.toml` — narrowly: add `prometheus = "0.13"` dep.
- `<NEW: crates/argo-cli/src/handlers/gateway/routes_openmetrics.rs>` (≤150 LOC) — encoder handler + content-type + public-GET allowlist.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly: ≤10 modified lines to wire `/openmetrics` route alongside `/metrics`.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: `pub(super) mod routes_openmetrics;`.
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs::is_public_get` — narrowly: add `/openmetrics`. ≤3 modified lines.
- The 8 emission sites for the brief-required metrics — each ≤10-15 LOC of metric.inc()/observe()/set() calls in their natural locations:
  - `crates/argo-cli/src/handlers/gateway/boot.rs` request-dispatch path
  - `crates/argo-cli/src/handlers/gateway/rate_limit*.rs`
  - `crates/argo-provider/src/transport.rs` + `breaker_observability.rs`
  - `crates/argo-audit/src/sink.rs` (tail_seq gauge)
- `docs/skycot/runbook/observability.md` — narrowly: add a short section on `/openmetrics` vs `/metrics`. ≤40 modified lines.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/metrics.rs`, `metrics_json.rs`, `metrics_text.rs`, `metrics_actions.rs` — existing custom handler is locked. The brief deliberately preserves it.
- `argo-telemetry::TelemetrySink` trait — orthogonal abstraction; this brief adds a sibling, doesn't replace.

## Acceptance criteria

- [ ] `argo_telemetry::default_registry() -> Arc<Registry>` returns the process-global singleton.
- [ ] Double-registration of the same metric name returns the existing handle (doesn't panic).
- [ ] `/openmetrics` route serves valid Prometheus-text exposition; `/metrics` continues to serve 18 metrics unchanged.
- [ ] All 8 brief-specified metrics emit at their natural sites + appear in `/openmetrics` output during a smoke run.
- [ ] Per-metric label cardinality stays within budget: `tenant_id` label only on the 2 explicit metrics; `provider`/`model` labels on the 3 provider metrics.
- [ ] Public-GET allowlist updated for `/openmetrics`.
- [ ] 5 unit tests + 1 integration smoke (full gateway boot + scrape both endpoints + assert both return 200).
- [ ] Operator runbook updated.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-prometheus-exporter-openmetrics-route`
- PR title: `telemetry: default_registry() + /openmetrics route alongside /metrics (mission-prometheus-exporter-openmetrics-route)`

## Notes for the implementer

- `prometheus` crate (0.13.x) is the standard; mature; small dep tree.
- Histogram buckets for `gateway_request_duration_seconds`: 8 exponential `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]` (10 boundaries gives 8 buckets per Prom convention — round up if your crate's API differs).
- `provider_breaker_state` gauge values: 0=closed, 1=half-open, 2=open. Documented in module-level doc.
- The 18 existing metrics in `/metrics` are not re-emitted via `default_registry()` — keep them in the existing handler. Auto-wiring is for FUTURE metrics + the 8 brief-specified ones.
- This is the foundation. Future briefs that want metrics just `default_registry().register_counter(...)` once and the metric appears in `/openmetrics` automatically.
