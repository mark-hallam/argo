//! Admin-flavour telemetry route wiring. Pulled out of [`super::boot`]
//! because boot.rs sits at the 500-LOC ceiling; this helper keeps the
//! ceiling intact while still routing the new `/metrics/cardinality`
//! endpoint through the accept loop.
//!
//! Also hosts [`apply_cardinality_caps`], the boot-time hook that
//! pushes the operator's `[telemetry.prometheus.cardinality_cap]`
//! config into the process-global tracker. Both are pure plumbing —
//! the heavy lifting lives in `argo_telemetry::cardinality`.

use argo_config::TelemetryPrometheusCardinalityCapConfig;
use argo_telemetry::{cardinality::Caps, prometheus_registry};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::routes_metrics_cardinality;

/// Try to serve the request as `/metrics/cardinality`. Returns `true`
/// when handled (response written, stream shut down); `false` so the
/// accept loop can fall through to the next dispatcher.
pub(super) async fn try_serve(req: &str, stream: &mut TcpStream) -> bool {
    if !routes_metrics_cardinality::is_metrics_cardinality_request(req) {
        return false;
    }
    let bytes = routes_metrics_cardinality::http_response();
    let _ = stream.write_all(&bytes).await;
    let _ = stream.shutdown().await;
    true
}

/// Hydrate the cardinality tracker from operator config. Called from
/// boot.rs after `ArgoConfig::load`. Idempotent — safe to call more
/// than once across reloads.
pub(super) fn apply_cardinality_caps(cfg: &TelemetryPrometheusCardinalityCapConfig) {
    let mut caps = Caps::defaults()
        .with_histogram_default(cfg.histogram_default)
        .with_counter_default(cfg.counter_gauge_default);
    for (metric, cap) in &cfg.per_metric {
        caps = caps.with_override(metric.clone(), *cap);
    }
    prometheus_registry::set_cardinality_caps(caps);
}
