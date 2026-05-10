//! OpenTelemetry Protocol (OTLP) exporter wiring for argo-telemetry.
//!
//! Built only when the `otlp` feature is enabled. Calling [`init`]
//! once on process start registers global OpenTelemetry tracer and
//! meter providers backed by the OTLP/HTTP exporters from
//! `opentelemetry-otlp`. From that point on, counter and histogram
//! observations emitted via `TracingTelemetry` are routed to the
//! OTLP meter, and spans are mirrored as OpenTelemetry spans on the
//! global tracer (in addition to continuing to emit on the
//! `tracing` side, so existing `tracing-subscriber` consumers keep
//! working unchanged).
//!
//! Operator setup: see `docs/skycot/RUNBOOK.md` (Telemetry section).
//!
//! Environment variables:
//! - `OTEL_EXPORTER_OTLP_ENDPOINT` — collector base URL (default
//!   `http://localhost:4318`, the OTLP/HTTP standard port).
//! - `OTEL_SERVICE_NAME` — service.name resource attribute (default
//!   `argo`).

use opentelemetry::{global, trace::Tracer, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{MetricExporter, Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime,
    trace::{Tracer as SdkTracer, TracerProvider},
    Resource,
};
use parking_lot::Mutex;
use std::{env, fmt};

/// Tracer name used for both the manual mirror in
/// [`crate::tracing_sink`] and the bridge layer in
/// [`crate::otlp_bridge`]. Sharing the name keeps both export paths
/// in the same instrumentation scope so collectors don't see two
/// separate tracer entries for the same binary.
pub(crate) const TRACER_NAME: &str = "argo";

/// Stash of the SDK [`SdkTracer`] built by the most recent [`init`]
/// call. The bridge layer reads it via [`sdk_tracer`] to construct
/// `OpenTelemetryLayer`, which requires a real SDK tracer (not the
/// global `BoxedTracer`) because `tracing-opentelemetry`'s
/// `PreSampledTracer` impl is gated on the SDK type.
static SDK_TRACER: Mutex<Option<SdkTracer>> = Mutex::new(None);

/// Hand back the SDK tracer registered by the most recent [`init`]
/// call. Returns `None` if [`init`] has not yet been called; callers
/// in that case typically fall back to a no-op layer.
#[must_use]
pub fn sdk_tracer() -> Option<SdkTracer> {
    SDK_TRACER.lock().clone()
}

const DEFAULT_ENDPOINT: &str = "http://localhost:4318";
const DEFAULT_SERVICE_NAME: &str = "argo";

/// Initialize global OpenTelemetry tracer + meter providers backed
/// by the OTLP/HTTP exporters. Reads the endpoint and service name
/// from `OTEL_EXPORTER_OTLP_ENDPOINT` and `OTEL_SERVICE_NAME`.
///
/// Returns a [`ShutdownGuard`] that flushes pending exports on drop.
/// Safe to call once per process; calling more than once will
/// install fresh providers, replacing whatever was registered.
pub fn init() -> Result<ShutdownGuard, OtlpInitError> {
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
    let service_name =
        env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| DEFAULT_SERVICE_NAME.to_string());

    let resource = Resource::new(vec![KeyValue::new("service.name", service_name)]);

    let traces_endpoint = format!("{}/v1/traces", endpoint.trim_end_matches('/'));
    let span_exporter = SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(&traces_endpoint)
        .build()
        .map_err(|e| OtlpInitError(format!("span exporter: {e}")))?;
    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(span_exporter, runtime::Tokio)
        .with_resource(resource.clone())
        .build();
    // Stash the SDK tracer so `crate::otlp_bridge::otlp_layer()` can
    // build an `OpenTelemetryLayer` against it. We must capture it
    // BEFORE moving the provider into the global registry because
    // `global::tracer(...)` returns the boxed wrapper and we need
    // the concrete SDK type for the bridge.
    let sdk_tracer = tracer_provider.tracer(TRACER_NAME);
    *SDK_TRACER.lock() = Some(sdk_tracer);
    global::set_tracer_provider(tracer_provider.clone());

    let metrics_endpoint = format!("{}/v1/metrics", endpoint.trim_end_matches('/'));
    let metric_exporter = MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(&metrics_endpoint)
        .build()
        .map_err(|e| OtlpInitError(format!("metric exporter: {e}")))?;
    let reader = PeriodicReader::builder(metric_exporter, runtime::Tokio).build();
    let meter_provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();
    global::set_meter_provider(meter_provider.clone());

    Ok(ShutdownGuard {
        tracer_provider,
        meter_provider,
    })
}

/// Open an OpenTelemetry span on the global tracer with the given
/// name and attributes. Crate-private — only `TracingTelemetry`
/// calls into this. Returns a `BoxedSpan` whose lifetime is owned
/// by the caller; the span is closed automatically on drop.
pub(crate) fn open_span(
    name: &'static str,
    attributes: &[(&str, &str)],
) -> opentelemetry::global::BoxedSpan {
    let tracer = global::tracer("argo");
    let mut builder = tracer.span_builder(name);
    if !attributes.is_empty() {
        let attrs: Vec<KeyValue> = attributes
            .iter()
            .map(|(k, v)| KeyValue::new((*k).to_string(), (*v).to_string()))
            .collect();
        builder = builder.with_attributes(attrs);
    }
    builder.start(&tracer)
}

/// Record a counter observation on the global meter.
pub(crate) fn record_counter(name: &'static str, by: u64, labels: &[(&str, &str)]) {
    let meter = global::meter("argo");
    let counter = meter.u64_counter(name).build();
    let kvs: Vec<KeyValue> = labels
        .iter()
        .map(|(k, v)| KeyValue::new((*k).to_string(), (*v).to_string()))
        .collect();
    counter.add(by, &kvs);
}

/// Record a histogram observation on the global meter.
pub(crate) fn record_histogram(name: &'static str, value: f64, labels: &[(&str, &str)]) {
    let meter = global::meter("argo");
    let histogram = meter.f64_histogram(name).build();
    let kvs: Vec<KeyValue> = labels
        .iter()
        .map(|(k, v)| KeyValue::new((*k).to_string(), (*v).to_string()))
        .collect();
    histogram.record(value, &kvs);
}

/// Owns the global tracer + meter providers; flushes pending
/// exports on drop via `shutdown` calls. Hold this for the
/// lifetime of the process and let the destructor run on shutdown.
pub struct ShutdownGuard {
    tracer_provider: TracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        let _ = self.tracer_provider.shutdown();
        let _ = self.meter_provider.shutdown();
    }
}

/// Failure to initialize the OTLP exporter — wraps the underlying
/// `opentelemetry-otlp` builder error as a string so we don't leak
/// the SDK error type into our public API.
#[derive(Debug)]
pub struct OtlpInitError(String);

impl fmt::Display for OtlpInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to initialize OTLP exporter: {}", self.0)
    }
}

impl std::error::Error for OtlpInitError {}
