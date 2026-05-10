//! Bridge from the `tracing` ecosystem to the OpenTelemetry tracer.
//!
//! Built only when the `otlp` feature is enabled. Returns a
//! [`tracing_subscriber::Layer`] that captures every `#[instrument]`
//! span (and every other `tracing` span) and forwards it to the
//! OpenTelemetry tracer registered by
//! [`crate::otlp_exporter::init`]. Span attributes recorded via
//! `#[instrument(fields(...))]` propagate as OTLP span attributes.
//!
//! Usage:
//!
//! ```ignore
//! use tracing_subscriber::prelude::*;
//!
//! let _guard = argo_telemetry::otlp_exporter::init().expect("otlp init");
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .with(argo_telemetry::otlp_bridge::otlp_layer())
//!     .init();
//! ```
//!
//! Order matters: [`crate::otlp_exporter::init`] MUST be called
//! before [`otlp_layer`] so the bridge picks up the real exporting
//! tracer rather than a no-op fallback. If init has not run, the
//! bridge attaches to an orphan SDK tracer that drops every span
//! silently — useful for tests that exercise the layer plumbing
//! without a collector, but a misconfiguration in production.

use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::Tracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::otlp_exporter::{sdk_tracer, TRACER_NAME};

/// Build a `tracing-opentelemetry` layer that forwards every
/// `tracing` span to the OpenTelemetry tracer registered by
/// [`crate::otlp_exporter::init`].
///
/// The returned layer is generic over any subscriber `S` that
/// implements `Subscriber + for<'a> LookupSpan<'a>`, which covers
/// `tracing_subscriber::Registry` and any `Layered` stack built on
/// top of it.
///
/// If [`crate::otlp_exporter::init`] has not yet been called, the
/// bridge attaches to an orphan, non-exporting SDK tracer so the
/// layer is still safe to install — but spans will not reach a
/// collector. Call `init()` first in production.
#[must_use]
pub fn otlp_layer<S>() -> OpenTelemetryLayer<S, Tracer>
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    let tracer = sdk_tracer().unwrap_or_else(orphan_tracer);
    OpenTelemetryLayer::new(tracer)
}

/// Type-erased boxed form of [`otlp_layer`]. Convenient when callers
/// want to assemble a heterogeneous subscriber whose layer types
/// differ across feature flags without naming the long
/// `OpenTelemetryLayer<_, _>` type.
#[must_use]
pub fn otlp_layer_boxed<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup> + Send + Sync + 'static,
{
    Box::new(otlp_layer::<S>())
}

/// SDK tracer detached from the global provider. Used as a safe
/// fallback when [`crate::otlp_exporter::init`] hasn't been called;
/// the tracer is real (so `OpenTelemetryLayer::new` accepts it) but
/// has no exporter, so spans are dropped silently.
fn orphan_tracer() -> Tracer {
    let provider = opentelemetry_sdk::trace::TracerProvider::builder().build();
    provider.tracer(TRACER_NAME)
}

#[cfg(test)]
#[path = "otlp_bridge_tests.rs"]
mod tests;
