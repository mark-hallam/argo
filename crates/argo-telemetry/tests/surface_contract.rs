use argo_telemetry::cardinality::{
    Caps, CardinalityDecision, CardinalityTracker, FamilyKind, DEFAULT_COUNTER_GAUGE_CAP,
    DEFAULT_HISTOGRAM_CAP,
};
use argo_telemetry::error_reporting::{
    capture_error_message, error_reporting_layer, init_error_reporting, ErrorReportingConfig,
    ErrorReportingInitError, ErrorReportingOptions,
};
use argo_telemetry::http_target::{HttpLogTarget, HttpLogTargetLayer, INTERNAL_TARGET};
use argo_telemetry::metrics_prompt_cache::{self, PromptCacheStats};
use argo_telemetry::pii_scrubber::{pseudonymize_tenant, scrub_pii};
use argo_telemetry::span_context::{enter_context, ContextInjectorLayer, LogContext};
use argo_telemetry::{
    cardinality, error_reporting, http_target, metrics, metrics_prompt_cache as prompt_cache,
    pii_scrubber, prometheus, prometheus_registry, span_context, spans, InMemoryTelemetry,
    NoopTelemetry, RecordedCounter, RecordedHistogram, RecordedSpan, SpanGuard, TelemetrySink,
    TracingTelemetry,
};
use std::any::type_name;

#[test]
fn public_surface_exports_expected_modules_types_traits_and_constants() {
    fn assert_sink<T: TelemetrySink + Send + Sync>() {}

    assert_sink::<NoopTelemetry>();
    assert_sink::<InMemoryTelemetry>();
    assert_sink::<TracingTelemetry>();

    let _guard_factory: fn() -> SpanGuard = SpanGuard::empty;
    let _span = RecordedSpan {
        name: spans::AGENT_TURN,
        attributes: vec![("tenant_id".to_string(), "tenant-a".to_string())],
    };
    let _counter = RecordedCounter {
        name: metrics::ROUTING_DECISIONS_TOTAL,
        by: 1,
        labels: vec![("tier".to_string(), "simple".to_string())],
    };
    let _histogram = RecordedHistogram {
        name: metrics::JUDGE_DURATION_MS.to_string(),
        value: 42.0,
        labels: vec![("verdict".to_string(), "ok".to_string())],
    };

    assert_eq!(DEFAULT_HISTOGRAM_CAP, 1000);
    assert_eq!(DEFAULT_COUNTER_GAUGE_CAP, 5000);
    let _caps = Caps::defaults()
        .with_counter_default(10)
        .with_histogram_default(5)
        .with_override("contract_metric_total", 3);
    assert_eq!(
        CardinalityTracker::new(Caps::defaults()).observe_kind(
            "contract_metric_total",
            &["same-family"],
            FamilyKind::CounterGauge,
        ),
        CardinalityDecision::Allow
    );

    assert_eq!(INTERNAL_TARGET, "argo_telemetry::http_target");
    assert!(type_name::<HttpLogTarget>().contains("HttpLogTarget"));
    assert!(type_name::<HttpLogTargetLayer>().contains("HttpLogTargetLayer"));

    let context = LogContext {
        tenant_id: Some("tenant-a".to_string()),
        worker_id: Some("worker-a".to_string()),
        session_id: Some("session-a".to_string()),
    };
    let _entered = enter_context(context.clone());
    let _context_layer = ContextInjectorLayer::new("anon");
    assert_eq!(context.tenant_id.as_deref(), Some("tenant-a"));

    let options = ErrorReportingOptions {
        mode: "disabled".to_string(),
        dsn: String::new(),
        otlp_endpoint: String::new(),
        pseudonym_salt: "contract-salt".to_string(),
    };
    assert_eq!(options.mode(), "disabled");
    init_error_reporting(&options).expect("disabled error reporting should initialize");
    capture_error_message(&options, "alice@example.com failed", Some("tenant-a"));
    let _layer = error_reporting_layer(options.clone());
    assert_eq!(
        ErrorReportingInitError::FeatureDisabled("otlp").to_string(),
        "error reporting feature 'otlp' is not enabled"
    );

    metrics_prompt_cache::record("surface-contract", "model-a", 10, 2, 3);
    let cache_rows: Vec<PromptCacheStats> = metrics_prompt_cache::snapshot();
    assert!(cache_rows
        .iter()
        .any(|row| row.provider == "surface-contract" && row.model == "model-a"));

    let _prometheus_registry = prometheus_registry::default_registry();
    let _encoder = prometheus::TextEncoder::new();
    assert!(prometheus_registry::GATEWAY_REQUEST_DURATION_BUCKETS
        .windows(2)
        .all(|w| w[0] < w[1]));
    assert!(prometheus_registry::ACTION_DISPATCH_DURATION_BUCKETS
        .windows(2)
        .all(|w| w[0] < w[1]));
    assert!(prometheus_registry::PROVIDER_CALL_DURATION_BUCKETS
        .windows(2)
        .all(|w| w[0] < w[1]));

    let _scrub_fn: fn(&str) -> String = pii_scrubber::scrub_pii;
    let _pseudonym_fn: fn(&str, &str) -> String = pii_scrubber::pseudonymize_tenant;
    let _registry_fn: fn() -> std::sync::Arc<prometheus::Registry> =
        prometheus_registry::default_registry;

    let module_type_names = [
        type_name::<cardinality::CardinalityTracker>(),
        type_name::<error_reporting::ErrorReportingOptions>(),
        type_name::<http_target::HttpLogTarget>(),
        type_name::<prompt_cache::PromptCacheStats>(),
        type_name::<span_context::LogContext>(),
    ];
    assert!(module_type_names
        .iter()
        .all(|name| name.starts_with("argo_telemetry::")));
}

#[test]
fn mission_taxonomy_names_stay_visible_to_gateway_and_workers() {
    assert_eq!(
        spans::all(),
        vec![
            spans::AGENT_CONTEXT_BUILD,
            spans::AGENT_PERSIST,
            spans::AGENT_PROVIDER_REQUEST,
            spans::AGENT_PROVIDER_STREAM,
            spans::AGENT_TOOLS_EXECUTE,
            spans::AGENT_TOOLS_PLAN,
            spans::AGENT_TURN,
            spans::FORGE_RUN,
            spans::GATEWAY_REQUEST_DISPATCH,
            spans::JUDGE_ADJUDICATE,
            spans::MARKETPLACE_PULL,
            spans::MARKETPLACE_PUSH,
            spans::ROUTING_CLASSIFY,
            spans::SESSION_CONTEXT,
            spans::SESSION_REPLAY,
            spans::WORKER_JOB_DISPATCH,
            spans::WORKER_STEP,
        ]
    );
    assert_eq!(
        metrics::all(),
        vec![
            metrics::FORGE_RUNS_TOTAL,
            metrics::FORGE_SPEND_USD_MICROS_TOTAL,
            metrics::JUDGE_DURATION_MS,
            metrics::JUDGE_VERDICTS_TOTAL,
            metrics::LOCK_VIOLATIONS_TOTAL,
            metrics::ROUTING_CLASSIFY_DURATION_MS,
            metrics::ROUTING_DECISIONS_TOTAL,
            metrics::SPEND_CAP_BLOCKS_TOTAL,
        ]
    );

    prometheus_registry::ensure_production_metrics_registered();
    let text = prometheus_registry::encode_text();
    for production_metric in [
        "action_dispatch_total",
        "action_dispatch_duration_seconds",
        "provider_call_duration_seconds",
        "tenant_active_count",
        "tenant_signup_total",
        "audit_rows_written_total",
        "audit_chain_verify_total",
        "workflow_cron_tick_total",
        "argo_metric_cardinality_capped_total",
    ] {
        assert!(
            text.contains(production_metric),
            "missing production metric {production_metric} in openmetrics output:\n{text}"
        );
    }
}

#[test]
fn noop_and_in_memory_sinks_keep_their_contracts() {
    let noop = NoopTelemetry;
    let _guard = noop.span(spans::AGENT_TURN, &[("tenant_id", "tenant-a")]);
    noop.counter(metrics::ROUTING_DECISIONS_TOTAL, 2, &[("tier", "simple")]);
    noop.observe(metrics::JUDGE_DURATION_MS, 14.5, &[("verdict", "ok")]);

    let recorder = InMemoryTelemetry::new();
    {
        let _guard = recorder.span(
            spans::WORKER_STEP,
            &[("tenant_id", "tenant-a"), ("worker_id", "worker-a")],
        );
    }
    recorder.counter(
        metrics::ROUTING_DECISIONS_TOTAL,
        7,
        &[("tier", "simple"), ("provider", "anthropic")],
    );
    recorder.observe(
        metrics::JUDGE_DURATION_MS,
        22.25,
        &[("verdict", "accepted")],
    );

    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, argo_telemetry::spans::WORKER_STEP);
    assert_eq!(
        spans[0].attributes,
        vec![
            ("tenant_id".to_string(), "tenant-a".to_string()),
            ("worker_id".to_string(), "worker-a".to_string()),
        ]
    );

    let counters = recorder.counters();
    assert_eq!(counters.len(), 1);
    assert_eq!(counters[0].name, metrics::ROUTING_DECISIONS_TOTAL);
    assert_eq!(counters[0].by, 7);
    assert_eq!(
        counters[0].labels,
        vec![
            ("tier".to_string(), "simple".to_string()),
            ("provider".to_string(), "anthropic".to_string()),
        ]
    );

    let histograms = recorder.histograms();
    assert_eq!(histograms.len(), 1);
    assert_eq!(histograms[0].name, metrics::JUDGE_DURATION_MS);
    assert!((histograms[0].value - 22.25).abs() < f64::EPSILON);
    assert_eq!(
        histograms[0].labels,
        vec![("verdict".to_string(), "accepted".to_string())]
    );
}

#[test]
fn cardinality_cap_drops_overflow_within_one_metric_family() {
    let tracker = CardinalityTracker::new(
        Caps::defaults().with_override("surface_contract_overflow_total", 8),
    );
    let mut drops = 0_u64;
    for i in 0..250 {
        let label = format!("tenant-{i}");
        if tracker.observe("surface_contract_overflow_total", &[label.as_str()])
            == CardinalityDecision::Drop
        {
            drops += 1;
        }
    }

    assert!(drops > 0, "expected overflow drops for one metric family");
    let row = tracker
        .snapshot()
        .into_iter()
        .find(|row| row.metric == "surface_contract_overflow_total")
        .expect("overflow family should be present");
    assert_eq!(row.cap, 8);
    assert_eq!(row.capped_total, drops);
}

#[test]
fn pii_scrubber_redacts_public_error_reporting_shapes() {
    let input = "contact alice@example.com with sk-123456789012345678901234 or +1 415 555 1212";
    let scrubbed = scrub_pii(input);

    assert!(!scrubbed.contains("alice@example.com"));
    assert!(!scrubbed.contains("sk-123456789012345678901234"));
    assert!(!scrubbed.contains("+1 415 555 1212"));
    assert!(scrubbed.contains("<redacted:email>"));
    assert!(scrubbed.contains("<redacted:provider-key>"));
    assert!(scrubbed.contains("<redacted:phone>"));

    let pseudonym_a = pseudonymize_tenant("tenant-raw", "stable-salt");
    let pseudonym_b = pseudonymize_tenant("tenant-raw", "stable-salt");
    assert_eq!(pseudonym_a, pseudonym_b);
    assert!(pseudonym_a.starts_with("tenant-"));
    assert!(!pseudonym_a.contains("tenant-raw"));
}
