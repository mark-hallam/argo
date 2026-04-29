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
    assert_eq!(h[0].0, metrics::JUDGE_DURATION_MS);
    assert!((h[0].1 - 200.0).abs() < 1e-9);
}
