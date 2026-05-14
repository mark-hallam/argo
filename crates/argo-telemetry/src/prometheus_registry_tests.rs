//! Unit tests for `prometheus_registry` — singleton, duplicate-safety,
//! encode-text-output. Each test uses a unique metric name so they
//! don't fight over the process-global registry when run together.

use super::*;

#[test]
fn default_registry_returns_singleton_across_calls() {
    let a = default_registry();
    let b = default_registry();
    assert!(Arc::ptr_eq(&a, &b), "default_registry must hand back the same Arc");
}

#[test]
fn register_counter_increments_into_registry() {
    let counter = register_counter(
        "test_register_counter_total",
        "Test counter for prometheus_registry_tests",
        &["label_a"],
    );
    counter.with_label_values(&["x"]).inc();
    counter.with_label_values(&["x"]).inc();
    let text = encode_text();
    assert!(
        text.contains("test_register_counter_total"),
        "encoded text should contain the counter family name:\n{text}"
    );
    assert!(
        text.contains(r#"label_a="x""#),
        "encoded text should carry the label dim:\n{text}"
    );
}

#[test]
fn register_histogram_records_observations() {
    let hist = register_histogram(
        "test_register_histogram_seconds",
        "Test histogram for prometheus_registry_tests",
        &[0.1, 0.5, 1.0],
        &["route"],
    );
    hist.with_label_values(&["/foo"]).observe(0.05);
    hist.with_label_values(&["/foo"]).observe(0.7);
    let text = encode_text();
    assert!(
        text.contains("test_register_histogram_seconds_bucket"),
        "histogram should emit bucket lines:\n{text}"
    );
    assert!(
        text.contains("test_register_histogram_seconds_sum"),
        "histogram should emit a sum line:\n{text}"
    );
}

#[test]
fn double_registration_returns_existing_handle() {
    let a = register_counter(
        "test_double_reg_counter_total",
        "First registration",
        &["k"],
    );
    let b = register_counter(
        "test_double_reg_counter_total",
        "Second registration (ignored — first wins)",
        &["k"],
    );
    a.with_label_values(&["only"]).inc();
    b.with_label_values(&["only"]).inc();
    // Both handles point at the same underlying collector — the
    // second `inc()` is visible through the first.
    assert_eq!(a.with_label_values(&["only"]).get(), 2);
}

#[test]
fn encode_text_produces_prometheus_exposition() {
    // Gauges without any `with_label_values` child render no exposition
    // at all — prometheus elides empty families. Materialise one child
    // so the test asserts the HELP / TYPE lines actually surface.
    let gauge = register_gauge(
        "test_encode_text_gauge",
        "Test gauge for encode_text_produces_prometheus_exposition",
        &["k"],
    );
    gauge.with_label_values(&["v"]).set(7);
    let text = encode_text();
    assert!(
        text.contains("# HELP test_encode_text_gauge"),
        "exposition should carry HELP lines:\n{text}"
    );
    assert!(
        text.contains("# TYPE test_encode_text_gauge gauge"),
        "exposition should carry TYPE lines:\n{text}"
    );
}

#[test]
fn register_scalar_gauge_set_visible_in_encoded_output() {
    let gauge = register_scalar_gauge(
        "test_scalar_gauge_set",
        "Test scalar gauge",
    );
    gauge.set(42);
    let text = encode_text();
    assert!(
        text.contains("test_scalar_gauge_set 42"),
        "scalar gauge value should appear in encoded output:\n{text}"
    );
}
