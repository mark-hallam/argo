//! `GET /openmetrics` — Prometheus-text exposition over the
//! process-global `argo_telemetry::prometheus_registry::default_registry()`.
//!
//! Parallel to the existing `/metrics` handler in `metrics.rs`. The two
//! endpoints carry **different** metric sets:
//!
//! - `/metrics` — 18 store-derived gauges/counters (session/message
//!   counts, audit chain integrity, etc.) computed by direct SQL.
//!   Hand-rolled exposition. Operators currently scraping this keep
//!   working without migration.
//! - `/openmetrics` — the 8 brief-specified metrics
//!   (`mission-prometheus-exporter-openmetrics-route`) plus any future
//!   metric a crate registers via `register_counter` / `register_gauge`
//!   / `register_histogram`. Auto-wired: register once, appears in the
//!   scrape automatically.
//!
//! Lives in a sibling module rather than collapsing onto `metrics.rs`
//! because (a) the brief deliberately preserves the existing handler
//! (892 LOC of code paths operators are already scraping) and (b) keeps
//! the new exposition surface to a single ≤150 LOC file under the 500-
//! LOC per-file ceiling.

use std::sync::OnceLock;

use argo_telemetry::prometheus_registry;

/// Cheap path-prefix check. Mirrors `super::metrics::is_metrics_request`
/// so the accept loop in `boot.rs` can dispatch `/openmetrics` outside
/// of the JSON response builder.
pub(super) fn is_openmetrics_request(req: &str) -> bool {
    req.starts_with("GET /openmetrics ") || req.starts_with("GET /openmetrics?")
}

/// Build the full HTTP response bytes for `/openmetrics`. Returned as
/// bytes so the caller can write straight into the `TcpStream` without
/// going through the JSON response builder.
#[must_use]
pub(super) fn http_response() -> Vec<u8> {
    let body = prometheus_registry::encode_text();
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
         Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    resp.into_bytes()
}

/// Lazy-init the brief-specified counters/gauges/histograms exactly
/// once. Called from `boot.rs` at gateway start so the metric families
/// appear in the very first `/openmetrics` scrape (even before the
/// first request fires the per-call emission sites).
///
/// Prometheus elides families with zero children from the text
/// exposition, so this function materialises a zero-value placeholder
/// child per family with sentinel labels (`init`). Operators see the
/// family on the very first scrape (value 0); real emission sites then
/// add real-label children alongside. Without the placeholders, an
/// idle gateway would scrape empty.
///
/// Idempotent — `register_*` is duplicate-safe; this `OnceLock` just
/// keeps the init cost off the request-dispatch path.
pub(super) fn ensure_metrics_registered() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        // `inc_by(0)` is a no-op that materialises the child series so
        // the family renders in the very first encoded scrape.
        brief_request_total()
            .with_label_values(&["init", "init", "init"])
            .inc_by(0);
        let _ = brief_request_duration().with_label_values(&["init", "init"]);
        brief_rate_limit_throttle_total()
            .with_label_values(&["init", ""])
            .inc_by(0);
        brief_provider_call_total()
            .with_label_values(&["init", "init", "init"])
            .inc_by(0);
        brief_provider_breaker_state()
            .with_label_values(&["init"])
            .set(0);
        brief_provider_cost_micros_total()
            .with_label_values(&["init", "init", "init"])
            .inc_by(0.0);
        brief_audit_chain_tail_seq().set(0);
        brief_store_migration_chain_version().set(0);
    });
}

/// `gateway_request_total{method,path,status}` — incremented in the
/// request-dispatch path. Cardinality budget: `path` is the rate-limit
/// `classify_route` template so UUIDs are masked to `:id` and the path
/// space stays bounded (~30 routes × ~5 methods × ~10 status codes ≈
/// 1500 series at full saturation).
pub(super) fn brief_request_total() -> argo_telemetry::prometheus::IntCounterVec {
    prometheus_registry::register_counter(
        "gateway_request_total",
        "Total gateway HTTP requests served, labelled by method, route template, and status",
        &["method", "path", "status"],
    )
}

/// `gateway_request_duration_seconds{method,path}` — histogram
/// observation in the request-dispatch path.
pub(super) fn brief_request_duration() -> argo_telemetry::prometheus::HistogramVec {
    prometheus_registry::register_histogram(
        "gateway_request_duration_seconds",
        "Gateway HTTP request duration in seconds, labelled by method and route template",
        prometheus_registry::GATEWAY_REQUEST_DURATION_BUCKETS,
        &["method", "path"],
    )
}

/// `gateway_rate_limit_throttle_total{scope,tenant_id?}` — incremented
/// in the per-IP and per-tenant limiter paths. `scope` is one of
/// `ip|tenant|route`. Cardinality budget: `tenant_id` only carries a
/// real value for `scope=tenant`; the IP and route scopes use a stable
/// empty-string sentinel to keep label-set arity constant (Prometheus
/// requires a fixed label set per metric family).
pub(super) fn brief_rate_limit_throttle_total() -> argo_telemetry::prometheus::IntCounterVec {
    prometheus_registry::register_counter(
        "gateway_rate_limit_throttle_total",
        "Gateway requests rejected by the rate limiter, labelled by scope (ip|tenant|route)",
        &["scope", "tenant_id"],
    )
}

/// `provider_call_total{provider,model,status}` — incremented in
/// `argo-provider::transport` dispatch.
pub(super) fn brief_provider_call_total() -> argo_telemetry::prometheus::IntCounterVec {
    prometheus_registry::register_counter(
        "provider_call_total",
        "Provider transport calls, labelled by provider id, model, and outcome (ok|error)",
        &["provider", "model", "status"],
    )
}

/// `provider_breaker_state{provider}` — gauge updated on state
/// transitions in `breaker_observability.rs`. Values: 0=closed,
/// 1=half-open, 2=open.
pub(super) fn brief_provider_breaker_state() -> argo_telemetry::prometheus::IntGaugeVec {
    prometheus_registry::register_gauge(
        "provider_breaker_state",
        "Circuit-breaker state per provider (0=closed, 1=half-open, 2=open)",
        &["provider"],
    )
}

/// `provider_cost_micros_total{provider,model,tenant_id}` — float
/// counter (cost is a fractional concept, even though we accumulate
/// USD-micros as integers most places). Tenant id is carried here per
/// the brief's explicit cardinality budget exception.
pub(super) fn brief_provider_cost_micros_total() -> argo_telemetry::prometheus::CounterVec {
    prometheus_registry::register_counter_f64(
        "provider_cost_micros_total",
        "Total provider spend in USD-micros, labelled by provider, model, and tenant",
        &["provider", "model", "tenant_id"],
    )
}

/// `audit_chain_tail_seq` — gauge holding the latest `seq` written to
/// the audit chain. Updated by the audit sink after a successful
/// append (best-effort).
pub(super) fn brief_audit_chain_tail_seq() -> argo_telemetry::prometheus::core::GenericGauge<argo_telemetry::prometheus::core::AtomicI64>
{
    prometheus_registry::register_scalar_gauge(
        "audit_chain_tail_seq",
        "Latest sequence number observed by the audit sink after a successful append",
    )
}

/// `store_migration_chain_version` — gauge set at boot from
/// `argo_store::EXPECTED_SCHEMA_VERSION`.
pub(super) fn brief_store_migration_chain_version()
-> argo_telemetry::prometheus::core::GenericGauge<argo_telemetry::prometheus::core::AtomicI64> {
    prometheus_registry::register_scalar_gauge(
        "store_migration_chain_version",
        "Expected store schema migration chain version (set once at boot)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_openmetrics_request_detects_path_prefix() {
        assert!(is_openmetrics_request("GET /openmetrics HTTP/1.1\r\n"));
        assert!(is_openmetrics_request("GET /openmetrics?foo=bar HTTP/1.1\r\n"));
        assert!(!is_openmetrics_request("GET /openmetricsfoo HTTP/1.1\r\n"));
        assert!(!is_openmetrics_request("GET /metrics HTTP/1.1\r\n"));
        assert!(!is_openmetrics_request("POST /openmetrics HTTP/1.1\r\n"));
    }

    #[test]
    fn http_response_has_prometheus_content_type() {
        ensure_metrics_registered();
        let bytes = http_response();
        let s = std::str::from_utf8(&bytes).expect("response is utf-8");
        assert!(s.starts_with("HTTP/1.1 200 OK"));
        assert!(s.contains("Content-Type: text/plain; version=0.0.4"));
    }

    #[test]
    fn ensure_metrics_registered_makes_all_eight_appear() {
        ensure_metrics_registered();
        let text = argo_telemetry::prometheus_registry::encode_text();
        for name in [
            "gateway_request_total",
            "gateway_request_duration_seconds",
            "gateway_rate_limit_throttle_total",
            "provider_call_total",
            "provider_breaker_state",
            "provider_cost_micros_total",
            "audit_chain_tail_seq",
            "store_migration_chain_version",
        ] {
            assert!(
                text.contains(name),
                "metric {name} should appear in encoded /openmetrics output:\n{text}"
            );
        }
    }
}
