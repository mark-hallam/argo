use super::*;

#[test]
fn span_taxonomy_has_thirteen_names() {
    // §18.1: 6 from v0.1 + 7 v0.2 additions = 13 total.
    assert_eq!(spans::all().len(), 13);
}

#[test]
fn metric_taxonomy_has_eight_names() {
    // §18.2 declares exactly 8 metrics.
    assert_eq!(metrics::all().len(), 8);
}

#[test]
fn span_names_are_unique_and_use_dot_namespacing() {
    let all = spans::all();
    let mut sorted = all.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, all);
    for s in all {
        assert!(s.contains('.'), "span name {s} should be dot-namespaced");
    }
}

#[test]
fn metric_names_are_unique_snake_case() {
    let all = metrics::all();
    let mut sorted = all.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, all);
    for m in all {
        assert!(m
            .chars()
            .all(|c| c == '_' || c.is_ascii_lowercase() || c.is_ascii_digit()));
    }
}

#[test]
fn noop_sink_drops_events_silently() {
    let s = NoopTelemetry;
    let _g = s.span(spans::AGENT_TURN, &[("session", "abc")]);
    s.counter(metrics::ROUTING_DECISIONS_TOTAL, 1, &[("tier", "simple")]);
    s.observe(metrics::ROUTING_CLASSIFY_DURATION_MS, 42.0, &[]);
}

#[test]
fn in_memory_sink_records_spans_with_attributes() {
    let s = InMemoryTelemetry::new();
    {
        let _g = s.span(spans::AGENT_TURN, &[("session", "abc"), ("turn", "1")]);
    }
    let recorded = s.spans();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].name, spans::AGENT_TURN);
    assert_eq!(
        recorded[0].attributes,
        vec![
            ("session".to_string(), "abc".to_string()),
            ("turn".to_string(), "1".to_string()),
        ]
    );
}

#[test]
fn in_memory_sink_records_counters() {
    let s = InMemoryTelemetry::new();
    s.counter(metrics::ROUTING_DECISIONS_TOTAL, 1, &[("tier", "simple")]);
    s.counter(metrics::ROUTING_DECISIONS_TOTAL, 1, &[("tier", "complex")]);
    let c = s.counters();
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].name, metrics::ROUTING_DECISIONS_TOTAL);
    assert_eq!(c[0].labels[0].1, "simple");
    assert_eq!(c[1].labels[0].1, "complex");
}

#[test]
fn in_memory_sink_records_histograms() {
    let s = InMemoryTelemetry::new();
    s.observe(metrics::JUDGE_DURATION_MS, 200.0, &[("verdict", "success")]);
    let h = s.histograms();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].name, metrics::JUDGE_DURATION_MS);
    assert!((h[0].value - 200.0).abs() < 1e-9);
}

#[test]
fn nested_spans_change_current_span() {
    // Pre-mission: TracingTelemetry::span entered the span and dropped
    // the entered guard on the same line, so by the time a child
    // `s.span(...)` ran, the parent was no longer current — child
    // spans had no parent. With the typed `SpanGuard` that owns an
    // `EnteredSpan`, the parent stays entered for the caller's
    // scope, so `tracing::Span::current()` walks the entered stack
    // and a nested `s.span(...)` inherits the right parent.
    use tracing::Span;
    use tracing_subscriber::{registry::Registry, util::SubscriberInitExt};
    let _guard = Registry::default().set_default();

    let s = TracingTelemetry;
    let initial = Span::current().id();

    let outer = s.span(spans::AGENT_TURN, &[]);
    let outer_id = Span::current().id();
    assert!(outer_id.is_some(), "outer should be entered as the current span");
    assert_ne!(outer_id, initial, "current span changes after entering outer");

    let inner = s.span(spans::AGENT_PROVIDER_REQUEST, &[]);
    let inner_id = Span::current().id();
    assert!(inner_id.is_some(), "inner should be entered as the current span");
    assert_ne!(inner_id, outer_id, "inner span has its own id");

    drop(inner);
    assert_eq!(
        Span::current().id(),
        outer_id,
        "after inner drop, outer is current again"
    );

    drop(outer);
    assert_eq!(
        Span::current().id(),
        initial,
        "after outer drop, current returns to initial"
    );
}

#[cfg(feature = "otlp")]
#[test]
fn otlp_record_path_does_not_panic_without_init() {
    // Without `otlp_exporter::init()` the global meter and tracer
    // are the no-op defaults from `opentelemetry::global`. The
    // record functions must still be safe to call — this guards
    // against a regression where the OTLP code path assumes a
    // provider is registered.
    crate::otlp_exporter::record_counter(
        metrics::ROUTING_DECISIONS_TOTAL,
        1,
        &[("tier", "simple")],
    );
    crate::otlp_exporter::record_histogram(
        metrics::JUDGE_DURATION_MS,
        42.0,
        &[("verdict", "ok")],
    );
    let span = crate::otlp_exporter::open_span(spans::AGENT_TURN, &[("session", "abc")]);
    drop(span);
}
