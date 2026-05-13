# Mission load-bearing wire — Prometheus exporter wired to default_registry

**Owner**: claude. **Depends on**: existing `argo-telemetry` crate (verify `default_registry()` exists via `rg -l 'default_registry\|prometheus' crates/argo-telemetry/src/`), existing gateway boot/serve plumbing.
**Authored against**: origin/main `bd7fd450` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prometheus-exporter-default-registry`.

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED**: `/metrics` is already wired in `crates/argo-cli/src/handlers/gateway/boot.rs:221` (early-dispatch `metrics::is_metrics_request` → `metrics::http_response` serving custom Prometheus-text exposition: `argo_sessions_total`, `argo_audit_chain_ok`, `argo_spend_usd_micros_total`, etc., via store SQL). Adding `/metrics` to `routes_dispatch_get_admin.rs` is dead code — boot.rs short-circuits before `routes::dispatch_route` runs. ALSO `default_registry()` does NOT exist in `crates/argo-telemetry/src/` (only `TelemetrySink` trait + Noop/InMemory/Tracing sinks). Either rescope to (a) convert the existing handler to use the `prometheus` crate registry while preserving store-derived metrics, OR (b) add a new path `/openmetrics` alongside existing `/metrics`. Either path requires expanding may-touch to include `boot.rs`, `metrics.rs`, `metrics_text.rs`, `metrics_actions.rs` — all currently forbidden by the brief. Conductor-level rewrite required.

> SOURCE: MISSION_BACKLOG.md Section 1 #32 ("Wire Prometheus exporter to `default_registry()`"). **Tier M (observability).** Today metrics are emitted (via tracing events + `argo-telemetry::default_registry` Counter / Histogram primitives) but there is no scrape endpoint — operators can't point Prometheus / Grafana / Datadog at SKYCOT. This brief lands the OpenMetrics scrape endpoint on the gateway so operators get one drop-in target.

## Scope

NEW gateway route `GET /metrics` returning OpenMetrics-text-format (Prometheus-compatible). Wire the `prometheus` crate exporter to `argo-telemetry::default_registry()` so every existing metric (and every future one) is automatically exposed.

NEW route handler `crates/argo-cli/src/handlers/gateway/routes_metrics.rs` (≤ 180 LOC):

```rust
pub fn render() -> ArgoResult<String>;          // calls TextEncoder::encode on default registry
pub fn dispatch_metrics_route() -> (u16, String, &'static str); // status + body + content_type
```

Add `/metrics` to the public-GET allowlist in `routes_auth.rs::is_public_get` (mirroring `/health` / `/livez` / `/readyz` precedent — Prometheus scrapers don't carry bearer tokens; operators should put a reverse-proxy in front if they want auth).

NEW `[gateway.metrics]` config block (`crates/argo-config/src/sections.rs`):
- `enabled: bool = true` — disable per-deployment when operators don't run Prometheus
- `protected: bool = false` — when true, REQUIRE bearer token (returns 401 instead of OpenMetrics). Default off because the typical deployment pattern is reverse-proxy auth.

Bridge layer `crates/argo-telemetry/src/prometheus_bridge.rs` (≤ 200 LOC):
- Wraps the existing `default_registry()` (whatever its current internal type) so that the `prometheus` crate's `Registry` can scrape from it.
- If `argo-telemetry` already wraps `prometheus::Registry` internally, this bridge is trivial; if it wraps a different counter/histogram primitive, this layer registers a `prometheus::Registry` parallel and the same metric names show up in both. Verify the existing primitive shape first via `rg 'pub fn counter\|pub fn histogram\|pub struct Registry' crates/argo-telemetry/src/`.

Mandatory metrics to surface (these likely already exist as tracing-event fields — adapt to prometheus counters/histograms):
- `gateway_request_total{method,path,status}` (counter)
- `gateway_request_duration_seconds{method,path}` (histogram with 8-bucket exponential 0.005s..5s)
- `gateway_rate_limit_throttle_total{scope}` (counter; scope=ip|tenant|route)
- `provider_call_total{provider,model,status}` (counter)
- `provider_breaker_state{provider}` (gauge; 0=closed/1=half-open/2=open)
- `provider_cost_micros_total{provider,model}` (counter; sum of micros, divided by 1e6 in dashboards)
- `audit_chain_tail_seq` (gauge; latest audit row sequence number)
- `store_migration_chain_version` (gauge)

This is NOT in scope: per-tenant cardinality on every metric (would blow the cardinality budget — `tenant_id` label only on `provider_cost_micros_total` and `gateway_rate_limit_throttle_total`); push-gateway support (only pull/scrape); custom histogram buckets per metric (one shared bucket set for v1); OpenTelemetry-Metrics export (separate brief); per-tracing-span metrics extraction (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_metrics.rs` (NEW, ≤ 180 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_metrics_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests (output-shape conformance, metric-presence assertions, content-type assertion, empty-registry safe, large-registry chunked, protected mode 401)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_admin.rs` — narrowly scoped: register `/metrics` dispatch (≤ 8 LOC delta)
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — narrowly scoped: add `/metrics` to `is_public_get` (when `protected = false`) (≤ 5 LOC delta)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: register `pub mod routes_metrics;`
- `crates/argo-telemetry/src/prometheus_bridge.rs` (NEW, ≤ 200 LOC)
- `crates/argo-telemetry/src/prometheus_bridge_tests.rs` (NEW, sibling, ≤ 150 LOC) — 5 unit tests
- `crates/argo-telemetry/src/lib.rs` — narrowly scoped: register module + `pub fn render_default_registry() -> ArgoResult<String>`
- `crates/argo-telemetry/Cargo.toml` — narrowly scoped: add `prometheus` workspace dep
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[gateway.metrics]` section
- `crates/argo-config/src/defaults.rs` — narrowly scoped: ADD default values
- `crates/argo-cli/tests/gateway_metrics_smoke.rs` (NEW, ≤ 150 LOC) — boot gateway, hit `/health` 3 times, hit `/metrics`, assert OpenMetrics text format + `gateway_request_total` counter ≥ 3

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema change
- `crates/argo-provider/**` — provider-side metric emitters already exist; this brief only wires the registry to the scrape endpoint
- Any frontend / web crate

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/metrics` returns 200 with `Content-Type: text/plain; version=0.0.4; charset=utf-8` (OpenMetrics)
- [ ] Output contains ALL 8 mandatory metrics in OpenMetrics text format
- [ ] `[gateway.metrics].enabled = false` returns 404 on `/metrics` (route is not registered)
- [ ] `[gateway.metrics].protected = true` returns 401 without bearer; 200 with valid bearer
- [ ] `prometheus` crate added to `argo-telemetry/Cargo.toml`; no other Cargo.toml diffs needed
- [ ] No tenant_id cardinality on the request_total counter (cardinality budget protection)
- [ ] 6 unit tests on routes_metrics + 5 on prometheus_bridge + 1 integration smoke
- [ ] `cargo clippy -p argo-telemetry -p argo-config -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-telemetry -p argo-config -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-prometheus-exporter-default-registry`
- PR title: `telemetry: wire prometheus exporter to default_registry + GET /metrics (mission-prometheus-exporter-default-registry)`

## Notes for the implementer

- First read `crates/argo-telemetry/src/` to understand the existing primitive shape (`default_registry()`, counter / histogram / gauge constructors). If it already wraps `prometheus::Registry`, the bridge is trivial (just `Encoder::encode(&registry.gather(), &mut buf)`). If it wraps a different primitive, the bridge needs to register a parallel `prometheus::Registry` and proxy registrations.
- The 8 mandatory metrics MUST be wired now; future metrics inherit the path. Verify each by looking at existing emit sites — most are already structured `tracing::warn!` events with metric-tag fields (e.g. `metric = "gateway_rate_limit_throttle_count"`); convert those to `Counter::inc()` calls in the appropriate crate. The conversion is a follow-up brief PER caller-site if the wiring is non-trivial; this brief delivers the bridge + scrape endpoint + the wiring for the easy metrics.
- OpenMetrics text format: use `prometheus::TextEncoder::encode(&metric_families, &mut buf)`. Set the response content-type to exactly `text/plain; version=0.0.4; charset=utf-8`.
- Mirror the `/health` route shape for `/metrics`: dispatched in `routes_dispatch_get_admin.rs`, allowlisted in `routes_auth.rs`, lives in its own sibling file under `handlers/gateway/`.
- `[gateway.metrics].protected = true` is the secure-default for production deployments without a reverse-proxy. Document this in module docs.
- Cardinality budget: the `path` label MUST be normalized — `/sessions/<uuid>/messages` should be labeled `path="/sessions/{id}/messages"`, not the literal UUID. Without this, every session creates a new label combination and the metrics blow up. The brief acceptance test should explicitly check that `/sessions/<uuid>` is normalized.
- Don't lock in the OpenTelemetry exporter format — that's a separate brief. Prometheus text-format is sufficient for v1 because every major observability vendor accepts it as a scrape target.
