//! `tracing`-backed telemetry sink — emits spans, counters, and
//! histogram observations as `tracing` events. Combined with
//! `tracing-subscriber` at the binary edge this gives stdout /
//! file / OTLP exporters without each subsystem touching a global.
//!
//! Span semantics: each `span()` call emits a `tracing::info_span!`
//! and the returned [`SpanGuard`] owns the entered span until the
//! caller drops it. Holding the entered span across the caller's
//! scope is what gives nested `span()` calls correct parent/child
//! relationships. With the `otlp` feature enabled, the guard
//! additionally owns an OpenTelemetry span that mirrors the same
//! attributes and is closed on drop.
//!
//! Counter and histogram observations emit `tracing::event!` rows
//! at INFO level with a structured `metric` field and the labels
//! flattened into the event. With the `otlp` feature on they go
//! through the OpenTelemetry meter instead, which delivers them to
//! the OTLP exporter configured by [`crate::otlp_exporter::init`].
use crate::sink::{SpanGuard, TelemetrySink};

/// Tracing-backed sink. Cheap (`Copy`) — share by value.
#[derive(Debug, Default, Clone, Copy)]
pub struct TracingTelemetry;

impl TelemetrySink for TracingTelemetry {
    fn span(&self, name: &'static str, attributes: &[(&str, &str)]) -> SpanGuard {
        // Flatten attributes into a single string so the span name
        // remains a static `&'static str` (tracing requires that).
        let attrs = attributes
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        let entered = tracing::info_span!("argo", span = name, attrs = %attrs).entered();

        #[cfg(feature = "otlp")]
        {
            let otlp_span = crate::otlp_exporter::open_span(name, attributes);
            SpanGuard::from_tracing_and_otlp(entered, otlp_span)
        }

        #[cfg(not(feature = "otlp"))]
        SpanGuard::from_tracing(entered)
    }

    fn counter(&self, name: &'static str, by: u64, labels: &[(&str, &str)]) {
        #[cfg(feature = "otlp")]
        crate::otlp_exporter::record_counter(name, by, labels);

        #[cfg(not(feature = "otlp"))]
        {
            let labels_str = labels
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ");
            tracing::info!(metric = "counter", name = name, by, labels = %labels_str);
        }
    }

    fn observe(&self, name: &'static str, value: f64, labels: &[(&str, &str)]) {
        #[cfg(feature = "otlp")]
        crate::otlp_exporter::record_histogram(name, value, labels);

        #[cfg(not(feature = "otlp"))]
        {
            let labels_str = labels
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ");
            tracing::info!(metric = "histogram", name = name, value, labels = %labels_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{metrics, spans};

    #[test]
    fn emits_span_counter_histogram_without_panic() {
        // The point of these tests is "doesn't panic"; assertions
        // on stdout would couple to the global tracing subscriber,
        // which is set per-binary. The InMemoryTelemetry sink is
        // the right fixture for routing-correctness tests; this
        // sink is exercised via integration tests in argo-cli.
        let s = TracingTelemetry;
        let _g = s.span(spans::AGENT_TURN, &[("session", "abc")]);
        s.counter(metrics::ROUTING_DECISIONS_TOTAL, 1, &[("tier", "simple")]);
        s.observe(metrics::JUDGE_DURATION_MS, 42.5, &[("verdict", "success")]);
    }

    #[test]
    fn empty_attributes_and_labels_are_safe() {
        let s = TracingTelemetry;
        let _g = s.span(spans::AGENT_PERSIST, &[]);
        s.counter(metrics::FORGE_RUNS_TOTAL, 1, &[]);
        s.observe(metrics::ROUTING_CLASSIFY_DURATION_MS, 0.0, &[]);
    }
}
