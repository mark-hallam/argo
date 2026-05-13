use super::{
    BreakerSection, ProvidersConfig, DEFAULT_FAILURE_THRESHOLD, DEFAULT_HALF_OPEN_PROBE_INTERVAL_MS,
};
use std::time::Duration;

#[test]
fn parse_default_when_no_provider_block_is_present() {
    // An argo.toml body with unrelated sections only — no
    // `[provider.*]` tables. Lookup of any id resolves to the
    // default entry and BreakerSection::default() values.
    let body = r#"
[core]
profile = "default"
"#;
    let cfg = ProvidersConfig::parse_from_argo_toml(body).expect("parse ok");
    assert!(cfg.providers.is_empty());
    let entry = cfg.entry_or_default("anthropic");
    assert!(entry.breaker.is_none());
    let breaker = entry.breaker_or_default();
    assert_eq!(
        breaker.resolved_failure_threshold(),
        DEFAULT_FAILURE_THRESHOLD
    );
    assert_eq!(
        breaker.resolved_half_open_probe_interval(),
        Duration::from_millis(DEFAULT_HALF_OPEN_PROBE_INTERVAL_MS)
    );
}

#[test]
fn parse_custom_overrides_thresholds_per_provider() {
    let body = r#"
[provider.anthropic.breaker]
failure_threshold = 3
half_open_probe_interval_ms = 5000

[provider.openai.breaker]
failure_threshold = 10
"#;
    let cfg = ProvidersConfig::parse_from_argo_toml(body).expect("parse ok");
    let anthropic = cfg.entry_or_default("anthropic");
    let anth_b = anthropic.breaker_or_default();
    assert_eq!(anth_b.resolved_failure_threshold(), 3);
    assert_eq!(
        anth_b.resolved_half_open_probe_interval(),
        Duration::from_millis(5000)
    );

    // Partial override: only failure_threshold set; the interval
    // falls back to the default.
    let openai = cfg.entry_or_default("openai");
    let oa_b = openai.breaker_or_default();
    assert_eq!(oa_b.resolved_failure_threshold(), 10);
    assert_eq!(
        oa_b.resolved_half_open_probe_interval(),
        Duration::from_millis(DEFAULT_HALF_OPEN_PROBE_INTERVAL_MS)
    );

    // Unrelated provider id → fresh default.
    let bedrock = cfg.entry_or_default("bedrock");
    assert!(bedrock.breaker.is_none());
}

#[test]
fn parse_rejects_zero_failure_threshold() {
    // Zero would either spin (never trip) or trip on every call —
    // both useless. Hard-reject so operators get a parse error
    // instead of a silent footgun.
    let body = r#"
[provider.anthropic.breaker]
failure_threshold = 0
"#;
    let err = ProvidersConfig::parse_from_argo_toml(body).expect_err("must reject zero");
    let msg = format!("{err}");
    assert!(
        msg.contains("failure_threshold"),
        "error must name the field, got: {msg}"
    );
    assert!(
        msg.contains("anthropic"),
        "error must name the provider, got: {msg}"
    );
}

#[test]
fn parse_rejects_zero_probe_interval() {
    let body = r#"
[provider.openai.breaker]
half_open_probe_interval_ms = 0
"#;
    let err = ProvidersConfig::parse_from_argo_toml(body).expect_err("must reject zero");
    let msg = format!("{err}");
    assert!(
        msg.contains("half_open_probe_interval_ms"),
        "error must name the field, got: {msg}"
    );
}

#[test]
fn breaker_section_defaults_are_none() {
    let b = BreakerSection::default();
    assert!(b.failure_threshold.is_none());
    assert!(b.half_open_probe_interval_ms.is_none());
}
