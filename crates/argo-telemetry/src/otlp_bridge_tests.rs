//! Sibling tests for `otlp_bridge.rs`. Included via `#[path]` from
//! `otlp_bridge.rs::tests` to keep the bridge module under the file
//! ceiling without splitting the public surface across crates.
//!
//! These tests run only when the `otlp` feature is enabled (the
//! whole bridge module is feature-gated). They exercise the pure
//! Rust surface — layer construction, attaching a layer to a
//! registry, and span-attribute propagation through the
//! `tracing-opentelemetry` adapter — without speaking to a real
//! collector. The end-to-end bytes-on-the-wire test lives in
//! `tests/otlp_bridge_smoke.rs`.

use super::*;
use tracing_subscriber::{prelude::*, Registry};

/// `otlp_layer()` returns a usable layer when the `otlp` feature is
/// on. The compiler enforces this — if the layer type changed under
/// a feature, this test would fail to build.
#[test]
fn otlp_layer_constructs_when_feature_on() {
    let _layer = otlp_layer::<Registry>();
}

/// The layer attaches to a fresh `Registry` without panicking and
/// emitting a span through that subscriber stack runs cleanly. We
/// don't assert export — the global tracer is a no-op until
/// `otlp_exporter::init()` is called — but we DO assert that the
/// bridge plumbing is wired correctly enough that the
/// `Registry → OpenTelemetryLayer` pipeline accepts a span without
/// blowing up.
#[test]
fn otlp_layer_attaches_to_subscriber() {
    let subscriber = Registry::default().with(otlp_layer::<Registry>());
    tracing::subscriber::with_default(subscriber, || {
        let _span = tracing::info_span!("bridge.test", custom_field = "hello").entered();
        tracing::info!("a log line inside the span");
    });
}

/// Span attributes recorded via `#[instrument(fields(...))]` must
/// reach the bridge layer. We can't observe the OTLP byte stream
/// from a unit test (that's the integration test's job), but we CAN
/// confirm that recording attributes via `tracing::Span::record` and
/// emitting nested spans through the bridge layer doesn't panic and
/// produces a parented span tree. If the bridge mis-handled fields
/// (e.g. failed to install a `WithContext` extension on the parent
/// span) this would surface as a runtime panic from
/// `tracing-opentelemetry`.
#[test]
fn span_fields_propagate_through_bridge() {
    let subscriber = Registry::default().with(otlp_layer::<Registry>());
    tracing::subscriber::with_default(subscriber, || {
        let outer = tracing::info_span!(
            "outer.span",
            session_id = tracing::field::Empty,
            tier = "smoke"
        );
        outer.record("session_id", "sess-1234");
        let _outer_entered = outer.enter();
        let _inner =
            tracing::info_span!("inner.span", parent_session = "sess-1234").entered();
        tracing::info!(event = "child", "nested event");
    });
}
