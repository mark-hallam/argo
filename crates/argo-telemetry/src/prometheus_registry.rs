//! Process-global Prometheus registry + register helpers.
//!
//! `default_registry()` returns the process-wide `Arc<prometheus::Registry>`
//! used by the gateway's `/openmetrics` parallel route
//! (`mission-prometheus-exporter-openmetrics-route`). Crates that want a
//! new counter / histogram / gauge call one of the `register_*`
//! convenience surfaces below — the helpers cache by metric name so a
//! second call with the same name returns the already-registered handle
//! instead of panicking (Prometheus' default registry rejects duplicate
//! names with `AlreadyReg`).
//!
//! The existing custom `/metrics` handler in
//! `crates/argo-cli/src/handlers/gateway/metrics.rs` is **not** wired
//! through this registry — it serves 18 store-derived metrics by direct
//! SQL and is preserved unchanged. Auto-wiring through this module is
//! for the 8 brief-specified gateway/provider/runtime metrics plus any
//! future metric.
//!
//! # Example
//!
//! ```
//! use argo_telemetry::prometheus_registry::{default_registry, register_counter};
//! use argo_telemetry::prometheus::{Encoder, TextEncoder};
//! let c = register_counter(
//!     "doc_example_counter_total",
//!     "Example counter from the rustdoc",
//!     &[],
//! );
//! c.with_label_values(&[]).inc();
//! let mut buf = Vec::new();
//! TextEncoder::new()
//!     .encode(&default_registry().gather(), &mut buf)
//!     .unwrap();
//! assert!(String::from_utf8_lossy(&buf).contains("doc_example_counter_total"));
//! ```

use parking_lot::Mutex;
use prometheus::core::{AtomicF64, AtomicI64, AtomicU64, GenericCounter, GenericGauge};
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// Histogram buckets for `gateway_request_duration_seconds`. 10
/// boundary values → 11 buckets per the prometheus convention
/// (`+Inf` is implicit). Matches the brief's exponential ladder.
pub const GATEWAY_REQUEST_DURATION_BUCKETS: &[f64] =
    &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0];

static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();

#[derive(Default)]
struct MetricCache {
    counters: HashMap<String, IntCounterVec>,
    histograms: HashMap<String, HistogramVec>,
    gauges: HashMap<String, IntGaugeVec>,
    float_counters: HashMap<String, prometheus::CounterVec>,
    float_gauges: HashMap<String, prometheus::GaugeVec>,
}

static CACHE: OnceLock<Mutex<MetricCache>> = OnceLock::new();

fn cache() -> &'static Mutex<MetricCache> {
    CACHE.get_or_init(|| Mutex::new(MetricCache::default()))
}

/// The process-global Prometheus registry.
///
/// First call initialises a fresh `Registry`; subsequent calls return
/// the same `Arc` clone. Cheap to call repeatedly.
#[must_use]
pub fn default_registry() -> Arc<Registry> {
    Arc::clone(REGISTRY.get_or_init(|| Arc::new(Registry::new())))
}

/// Register (or look up) an integer counter vector keyed by `name`.
///
/// `label_names` are the variable label dimensions; pass an empty slice
/// for a label-free counter. Repeated calls with the same `name` return
/// the previously-registered handle (no panic on duplicate
/// registration).
///
/// # Panics
///
/// Panics if Prometheus rejects the metric (e.g. invalid name or
/// disagreeing label set on a re-registration with different
/// dimensions). Panics here surface a programmer bug at boot, not at
/// runtime under load — the metric set is closed at compile time.
#[must_use]
pub fn register_counter(name: &str, help: &str, label_names: &[&str]) -> IntCounterVec {
    let mut cache = cache().lock();
    if let Some(existing) = cache.counters.get(name) {
        return existing.clone();
    }
    let opts = Opts::new(name, help);
    let counter = IntCounterVec::new(opts, label_names)
        .unwrap_or_else(|e| panic!("counter {name} construction failed: {e}"));
    register_or_existing(&default_registry(), &counter, name);
    cache.counters.insert(name.to_string(), counter.clone());
    counter
}

/// Register (or look up) a float counter vector keyed by `name`.
///
/// Float-flavoured siblings exist because spend / cost counters carry
/// fractional values (USD-micros divided by 1e6, for example) and the
/// integer counter API rejects non-integer increments.
#[must_use]
pub fn register_counter_f64(
    name: &str,
    help: &str,
    label_names: &[&str],
) -> prometheus::CounterVec {
    let mut cache = cache().lock();
    if let Some(existing) = cache.float_counters.get(name) {
        return existing.clone();
    }
    let opts = Opts::new(name, help);
    let counter = prometheus::CounterVec::new(opts, label_names)
        .unwrap_or_else(|e| panic!("float counter {name} construction failed: {e}"));
    register_or_existing(&default_registry(), &counter, name);
    cache
        .float_counters
        .insert(name.to_string(), counter.clone());
    counter
}

/// Register (or look up) a histogram vector keyed by `name`.
///
/// `buckets` is the boundary list (NOT including `+Inf`, which Prometheus
/// adds itself).
#[must_use]
pub fn register_histogram(
    name: &str,
    help: &str,
    buckets: &[f64],
    label_names: &[&str],
) -> HistogramVec {
    let mut cache = cache().lock();
    if let Some(existing) = cache.histograms.get(name) {
        return existing.clone();
    }
    let opts = HistogramOpts::new(name, help).buckets(buckets.to_vec());
    let hist = HistogramVec::new(opts, label_names)
        .unwrap_or_else(|e| panic!("histogram {name} construction failed: {e}"));
    register_or_existing(&default_registry(), &hist, name);
    cache.histograms.insert(name.to_string(), hist.clone());
    hist
}

/// Register (or look up) an integer gauge vector keyed by `name`.
#[must_use]
pub fn register_gauge(name: &str, help: &str, label_names: &[&str]) -> IntGaugeVec {
    let mut cache = cache().lock();
    if let Some(existing) = cache.gauges.get(name) {
        return existing.clone();
    }
    let opts = Opts::new(name, help);
    let gauge = IntGaugeVec::new(opts, label_names)
        .unwrap_or_else(|e| panic!("gauge {name} construction failed: {e}"));
    register_or_existing(&default_registry(), &gauge, name);
    cache.gauges.insert(name.to_string(), gauge.clone());
    gauge
}

/// Register (or look up) a float gauge vector keyed by `name`.
#[must_use]
pub fn register_gauge_f64(name: &str, help: &str, label_names: &[&str]) -> prometheus::GaugeVec {
    let mut cache = cache().lock();
    if let Some(existing) = cache.float_gauges.get(name) {
        return existing.clone();
    }
    let opts = Opts::new(name, help);
    let gauge = prometheus::GaugeVec::new(opts, label_names)
        .unwrap_or_else(|e| panic!("float gauge {name} construction failed: {e}"));
    register_or_existing(&default_registry(), &gauge, name);
    cache.float_gauges.insert(name.to_string(), gauge.clone());
    gauge
}

/// Encode the current registry to Prometheus text-exposition format
/// (`text/plain; version=0.0.4; charset=utf-8`).
///
/// Used by the gateway's `/openmetrics` route. Returns the body bytes;
/// the caller frames the HTTP response. Errors during encoding are
/// surfaced as a short comment in-band so a scrape never fails outright.
#[must_use]
pub fn encode_text() -> String {
    let registry = default_registry();
    let metric_families = registry.gather();
    let mut buf = Vec::with_capacity(4096);
    let encoder = prometheus::TextEncoder::new();
    match encoder.encode(&metric_families, &mut buf) {
        Ok(()) => String::from_utf8(buf)
            .unwrap_or_else(|e| format!("# error: prometheus text not utf-8: {e}\n")),
        Err(e) => format!("# error: prometheus encode failed: {e}\n"),
    }
}

/// Strict-cast helper for the few sites that need a *single*
/// scalar counter (rather than a vector). Returns the no-label child
/// of an empty-label vector; useful for boot-time gauges like
/// `store_migration_chain_version` that never carry a dimension.
///
/// `IntCounterVec` is backed by `AtomicU64` in `prometheus 0.13` — the
/// concrete underlying atom type leaks through `GenericCounter`'s
/// type parameter, so this helper mirrors the upstream choice rather
/// than coercing into a different atom.
#[must_use]
pub fn register_scalar_counter(name: &str, help: &str) -> GenericCounter<AtomicU64> {
    register_counter(name, help, &[]).with_label_values(&[])
}

/// Strict-cast helper for the few sites that need a single scalar gauge.
#[must_use]
pub fn register_scalar_gauge(name: &str, help: &str) -> GenericGauge<AtomicI64> {
    register_gauge(name, help, &[]).with_label_values(&[])
}

/// Strict-cast helper for a single scalar float gauge (used by
/// `provider_breaker_state` when no dimension applies to a metric).
#[must_use]
pub fn register_scalar_gauge_f64(name: &str, help: &str) -> GenericGauge<AtomicF64> {
    register_gauge_f64(name, help, &[]).with_label_values(&[])
}

fn register_or_existing<C: prometheus::core::Collector + Clone + 'static>(
    registry: &Registry,
    collector: &C,
    name: &str,
) {
    match registry.register(Box::new(collector.clone())) {
        Ok(()) => (),
        // `AlreadyReg` is benign in tests / repeated calls; any other
        // error is a programmer bug worth shouting about at boot.
        Err(prometheus::Error::AlreadyReg) => (),
        Err(e) => panic!("prometheus register {name} failed: {e}"),
    }
}

/// Histogram buckets for action-dispatch latency. Coarser than the
/// gateway-request ladder because action dispatch (email send, slack
/// post, voice-call setup) crosses external services and dwells in
/// the seconds range, not milliseconds.
pub const ACTION_DISPATCH_DURATION_BUCKETS: &[f64] =
    &[0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0];

/// Histogram buckets for provider-call latency. Providers vary widely;
/// the upper bound (120s) matches the gateway streaming-deadline policy.
pub const PROVIDER_CALL_DURATION_BUCKETS: &[f64] =
    &[0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0];

/// `action_dispatch_total{kind, status}` — counter incremented on every
/// channel-dispatcher action attempt. `status` is `ok|error`. `kind` is
/// the dispatcher kind id (email, slack, voice, push, …).
#[must_use]
pub fn action_dispatch_total() -> IntCounterVec {
    register_counter(
        "action_dispatch_total",
        "Action-dispatch attempts, labelled by kind and outcome (ok|error)",
        &["kind", "status"],
    )
}

/// `action_dispatch_duration_seconds{kind}` — histogram of dispatch
/// latency, observed on every action attempt (success or failure).
#[must_use]
pub fn action_dispatch_duration() -> HistogramVec {
    register_histogram(
        "action_dispatch_duration_seconds",
        "Action-dispatch latency in seconds, labelled by kind",
        ACTION_DISPATCH_DURATION_BUCKETS,
        &["kind"],
    )
}

/// `provider_call_duration_seconds{provider, model}` — provider-call
/// latency histogram. Sibling to the existing `provider_call_total`
/// counter exposed by `routes_openmetrics`.
#[must_use]
pub fn provider_call_duration() -> HistogramVec {
    register_histogram(
        "provider_call_duration_seconds",
        "Provider transport latency in seconds, labelled by provider and model",
        PROVIDER_CALL_DURATION_BUCKETS,
        &["provider", "model"],
    )
}

/// `tenant_active_count` — gauge of tenants with at least one
/// session in the last 5 minutes. Operator-set at scrape time from a
/// store query.
#[must_use]
pub fn tenant_active_count() -> GenericGauge<AtomicI64> {
    register_scalar_gauge(
        "tenant_active_count",
        "Tenants with at least one active session in the rolling 5-minute window",
    )
}

/// `tenant_signup_total` — counter incremented on every new tenant
/// signup. The `signup-rate-spike` Alertmanager rule uses
/// `rate(tenant_signup_total[5m])` against a 1-hour baseline.
#[must_use]
pub fn tenant_signup_total() -> GenericCounter<AtomicU64> {
    register_scalar_counter(
        "tenant_signup_total",
        "Total tenant signups since gateway boot",
    )
}

/// `audit_rows_written_total` — counter incremented after every
/// successful audit-record append. Feeds `audit-rows-per-second`
/// dashboard panel via `rate(audit_rows_written_total[1m])`.
#[must_use]
pub fn audit_rows_written_total() -> GenericCounter<AtomicU64> {
    register_scalar_counter(
        "audit_rows_written_total",
        "Audit-record rows written since gateway boot",
    )
}

/// `audit_chain_verify_total{result}` — counter incremented on every
/// chain-verify run. `result` is `ok|broken`. The verify-pass-rate
/// dashboard panel uses `rate(audit_chain_verify_total{result="ok"}[5m])
/// / rate(audit_chain_verify_total[5m])`.
#[must_use]
pub fn audit_chain_verify_total() -> IntCounterVec {
    register_counter(
        "audit_chain_verify_total",
        "Audit-chain verify-all runs, labelled by result (ok|broken)",
        &["result"],
    )
}

/// `workflow_cron_tick_total{name, status}` — counter incremented on
/// every cron-driven workflow tick. `status` is `ok|error`. The
/// cron-failure-rate dashboard panel uses
/// `rate(workflow_cron_tick_total{status="error"}[5m])`.
#[must_use]
pub fn workflow_cron_tick_total() -> IntCounterVec {
    register_counter(
        "workflow_cron_tick_total",
        "Cron-driven workflow ticks, labelled by workflow name and outcome (ok|error)",
        &["name", "status"],
    )
}

/// Materialise zero-valued init children for the production-metric
/// families so the first scrape after gateway boot already lists every
/// expected `# HELP` / `# TYPE` line. Prometheus elides empty families
/// from the text exposition, so without this step an idle gateway
/// scrapes empty and the dashboards/alerts see no metric until the
/// first emission.
///
/// Idempotent — the `OnceLock` keeps the cost off the per-scrape path
/// and the underlying `register_*` helpers are duplicate-safe anyway.
pub fn ensure_production_metrics_registered() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        action_dispatch_total()
            .with_label_values(&["init", "init"])
            .inc_by(0);
        let _ = action_dispatch_duration().with_label_values(&["init"]);
        let _ = provider_call_duration().with_label_values(&["init", "init"]);
        tenant_active_count().set(0);
        tenant_signup_total().inc_by(0);
        audit_rows_written_total().inc_by(0);
        audit_chain_verify_total()
            .with_label_values(&["init"])
            .inc_by(0);
        workflow_cron_tick_total()
            .with_label_values(&["init", "init"])
            .inc_by(0);
    });
}

#[cfg(test)]
#[path = "prometheus_registry_tests.rs"]
mod tests;
