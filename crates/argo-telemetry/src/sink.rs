//! `TelemetrySink` trait + a no-op default + an in-memory recorder
//! used in tests / fixtures.
use parking_lot::Mutex;
use std::sync::Arc;

/// One recorded span (entered + closed scope, attributes flattened).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedSpan {
    /// Span name (one of `argo_telemetry::spans::*`).
    pub name: &'static str,
    /// Attributes attached at start.
    pub attributes: Vec<(String, String)>,
}

/// One recorded counter increment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCounter {
    /// Metric name (one of `argo_telemetry::metrics::*`).
    pub name: &'static str,
    /// Increment.
    pub by: u64,
    /// Label set.
    pub labels: Vec<(String, String)>,
}

/// Telemetry seam.
///
/// `span()` returns an opaque guard the caller drops when the
/// instrumented scope ends. The no-op sink returns `()`; the
/// in-memory sink records the span on enter.
pub trait TelemetrySink: Send + Sync {
    /// Open a span. The returned guard is dropped when the caller
    /// leaves the instrumented scope. Implementations may emit
    /// `tracing::info_span!` events under the hood.
    fn span(&self, name: &'static str, attributes: &[(&str, &str)]) -> SpanGuard;
    /// Increment a counter.
    fn counter(&self, name: &'static str, by: u64, labels: &[(&str, &str)]);
    /// Record a histogram observation (ms / bytes / etc.).
    fn observe(&self, name: &'static str, value: f64, labels: &[(&str, &str)]);
}

/// Opaque scope guard returned by `TelemetrySink::span`.
///
/// The default sink (`NoopTelemetry`) and the in-memory recorder
/// return an empty guard ([`SpanGuard::empty`]). The
/// `tracing`-backed sink ([`crate::TracingTelemetry`]) builds a
/// guard that owns an entered `tracing` span; the entered scope
/// is held until the guard is dropped, which is what makes nested
/// spans inherit the correct parent/child relationship. With the
/// `otlp` feature enabled, the guard additionally owns an
/// OpenTelemetry span that is closed on drop.
pub struct SpanGuard {
    _tracing: Option<tracing::span::EnteredSpan>,
    #[cfg(feature = "otlp")]
    _otlp: Option<opentelemetry::global::BoxedSpan>,
}

impl SpanGuard {
    /// Build an empty guard. Used by sinks that do not own an
    /// entered span (`NoopTelemetry`, `InMemoryTelemetry`).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            _tracing: None,
            #[cfg(feature = "otlp")]
            _otlp: None,
        }
    }

    /// Build a guard that owns an entered `tracing` span. Crate-private
    /// because the `tracing` types are an implementation detail.
    /// Used by `TracingTelemetry` when the `otlp` feature is off.
    #[cfg(not(feature = "otlp"))]
    pub(crate) fn from_tracing(entered: tracing::span::EnteredSpan) -> Self {
        Self { _tracing: Some(entered) }
    }

    /// Build a guard that owns both an entered `tracing` span and a
    /// matching OpenTelemetry span. Crate-private; only constructed
    /// when the `otlp` feature is enabled.
    #[cfg(feature = "otlp")]
    pub(crate) fn from_tracing_and_otlp(
        entered: tracing::span::EnteredSpan,
        otlp: opentelemetry::global::BoxedSpan,
    ) -> Self {
        Self {
            _tracing: Some(entered),
            _otlp: Some(otlp),
        }
    }
}

/// No-op sink — drops every event. Default for tests + the §6
/// walking skeleton.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopTelemetry;

impl TelemetrySink for NoopTelemetry {
    fn span(&self, name: &'static str, attributes: &[(&str, &str)]) -> SpanGuard {
        let _ = (name, attributes);
        SpanGuard::empty()
    }
    fn counter(&self, name: &'static str, by: u64, labels: &[(&str, &str)]) {
        let _ = (name, by, labels);
    }
    fn observe(&self, name: &'static str, value: f64, labels: &[(&str, &str)]) {
        let _ = (name, value, labels);
    }
}

/// One recorded histogram observation.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedHistogram {
    /// Metric name.
    pub name: String,
    /// Observed value.
    pub value: f64,
    /// Label set.
    pub labels: Vec<(String, String)>,
}

/// Test-only recording sink. Tests assert that the agent loop emits
/// the expected spans and counters.
#[derive(Debug, Default, Clone)]
pub struct InMemoryTelemetry {
    spans: Arc<Mutex<Vec<RecordedSpan>>>,
    counters: Arc<Mutex<Vec<RecordedCounter>>>,
    histograms: Arc<Mutex<Vec<RecordedHistogram>>>,
}

impl InMemoryTelemetry {
    /// New empty recorder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all recorded spans, in arrival order.
    #[must_use]
    pub fn spans(&self) -> Vec<RecordedSpan> {
        self.spans.lock().clone()
    }

    /// Snapshot of all recorded counter increments, in arrival order.
    #[must_use]
    pub fn counters(&self) -> Vec<RecordedCounter> {
        self.counters.lock().clone()
    }

    /// Snapshot of all recorded histogram observations.
    #[must_use]
    pub fn histograms(&self) -> Vec<RecordedHistogram> {
        self.histograms.lock().clone()
    }
}

impl TelemetrySink for InMemoryTelemetry {
    fn span(&self, name: &'static str, attributes: &[(&str, &str)]) -> SpanGuard {
        self.spans.lock().push(RecordedSpan {
            name,
            attributes: attributes
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        });
        SpanGuard::empty()
    }
    fn counter(&self, name: &'static str, by: u64, labels: &[(&str, &str)]) {
        self.counters.lock().push(RecordedCounter {
            name,
            by,
            labels: labels
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        });
    }
    fn observe(&self, name: &'static str, value: f64, labels: &[(&str, &str)]) {
        self.histograms.lock().push(RecordedHistogram {
            name: name.to_string(),
            value,
            labels: labels
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        });
    }
}
