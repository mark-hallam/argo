//! Declarative-KPI tests for [`crate::manifest`].
//!
//! Six unit tests covering the contract carried by
//! `mission-vertical-kpi-declarative`:
//!   1. `[[kpi]]` parses end-to-end via [`super::parse`].
//!   2. Empty `kpis` array → legacy [`SuccessMetricsSection`]
//!      fields materialize via [`super::VerticalManifest::all_kpis`].
//!   3. When both `[[kpi]]` AND legacy fields are populated,
//!      declarative entries win (no double-counting).
//!   4. `comparator = "gte"` round-trips through serde.
//!   5. `comparator = "lte"` round-trips through serde.
//!   6. All three `gate` values (`release`, `warn`, `inform`)
//!      round-trip.
use super::*;

const HEADER: &str = r#"schema_version = "1"

[vertical]
id = "demo"
name = "Demo"
version = "0.1.0"
"#;

fn parse_or_panic(src: &str) -> VerticalManifest {
    parse(src).unwrap_or_else(|e| panic!("expected parse OK; got: {e}\n--- src ---\n{src}"))
}

#[test]
fn parses_kpi_block() {
    let src = format!(
        r#"{HEADER}
[[kpi]]
name        = "deflection_rate"
kind        = "percentage"
threshold   = 80.0
comparator  = "gte"
gate        = "release"
description = "Pct of conversations resolved without a human handoff."

[[kpi]]
name       = "first_response_p95_seconds"
kind       = "duration_ms"
threshold  = 30000.0
comparator = "lte"
gate       = "warn"
"#
    );
    let m = parse_or_panic(&src);
    assert_eq!(m.kpis.len(), 2);
    let first = &m.kpis[0];
    assert_eq!(first.name, "deflection_rate");
    assert_eq!(first.kind, KpiKind::Percentage);
    assert_eq!(first.comparator, KpiComparator::Gte);
    assert_eq!(first.gate, KpiGate::Release);
    assert!(first.description.is_some());
    let second = &m.kpis[1];
    assert_eq!(second.kind, KpiKind::DurationMs);
    assert_eq!(second.comparator, KpiComparator::Lte);
    assert_eq!(second.gate, KpiGate::Warn);
    assert!(second.description.is_none());
    assert_eq!(m.all_kpis().len(), 2);
}

#[test]
fn empty_kpis_falls_back_to_legacy_success_metrics() {
    let src = format!(
        r#"{HEADER}
[success_metrics]
deflection_target_pct       = 80
median_response_seconds     = 30
csat_threshold              = 4.5
required_audit_chain_intact = true
"#
    );
    let m = parse_or_panic(&src);
    assert!(m.kpis.is_empty(), "no [[kpi]] entries");
    let materialized = m.all_kpis();
    assert_eq!(materialized.len(), 4, "all 4 legacy fields materialize");

    let by_name: std::collections::BTreeMap<&str, &KpiDefinition> = materialized
        .iter()
        .map(|k| (k.name.as_str(), k))
        .collect();

    let defl = by_name.get("deflection_target_pct").expect("deflection materialized");
    assert_eq!(defl.kind, KpiKind::Percentage);
    assert_eq!(defl.comparator, KpiComparator::Gte);
    assert_eq!(defl.gate, KpiGate::Release);
    assert!((defl.threshold - 80.0).abs() < f64::EPSILON);

    let resp = by_name
        .get("median_response_seconds")
        .expect("median_response materialized");
    assert_eq!(resp.kind, KpiKind::DurationMs);
    assert_eq!(resp.comparator, KpiComparator::Lte);
    assert!(
        (resp.threshold - 30_000.0).abs() < f64::EPSILON,
        "seconds converted to ms in the KpiDefinition (kind=DurationMs)"
    );

    let csat = by_name.get("csat_threshold").expect("csat materialized");
    assert_eq!(csat.kind, KpiKind::Score);
    assert_eq!(csat.comparator, KpiComparator::Gte);

    let audit = by_name
        .get("required_audit_chain_intact")
        .expect("audit materialized");
    assert_eq!(audit.kind, KpiKind::Count);
    assert_eq!(audit.comparator, KpiComparator::Eq);
    assert!((audit.threshold - 1.0).abs() < f64::EPSILON);
}

#[test]
fn both_kpis_and_legacy_present_declarative_wins() {
    // Operators migrating a legacy field into a [[kpi]] entry
    // should NOT see the legacy field re-emerge in all_kpis() —
    // otherwise the same metric is double-counted by every
    // downstream consumer.
    let src = format!(
        r#"{HEADER}
[[kpi]]
name       = "custom_handoff_rate"
kind       = "ratio"
threshold  = 0.15
comparator = "lte"
gate       = "release"

[success_metrics]
deflection_target_pct = 75
csat_threshold        = 4.2
"#
    );
    let m = parse_or_panic(&src);
    assert_eq!(m.kpis.len(), 1, "one declarative entry");
    assert!(m.success_metrics.is_some(), "legacy block also parses");

    let unified = m.all_kpis();
    assert_eq!(
        unified.len(),
        1,
        "declarative wins; legacy fields suppressed when [[kpi]] is non-empty"
    );
    assert_eq!(unified[0].name, "custom_handoff_rate");
    assert_eq!(unified[0].kind, KpiKind::Ratio);
}

#[test]
fn comparator_gte_round_trips() {
    let src = format!(
        r#"{HEADER}
[[kpi]]
name       = "uptime_pct"
kind       = "percentage"
threshold  = 99.9
comparator = "gte"
gate       = "release"
"#
    );
    let m = parse_or_panic(&src);
    assert_eq!(m.kpis[0].comparator, KpiComparator::Gte);
    // serialize → parse → identical comparator preserved
    let toml = toml::to_string(&m).expect("serialize");
    let round = parse(&toml).expect("re-parse");
    assert_eq!(round.kpis[0].comparator, KpiComparator::Gte);
}

#[test]
fn comparator_lte_round_trips() {
    let src = format!(
        r#"{HEADER}
[[kpi]]
name       = "p99_latency_ms"
kind       = "duration_ms"
threshold  = 1500.0
comparator = "lte"
gate       = "warn"
"#
    );
    let m = parse_or_panic(&src);
    assert_eq!(m.kpis[0].comparator, KpiComparator::Lte);
    let toml = toml::to_string(&m).expect("serialize");
    let round = parse(&toml).expect("re-parse");
    assert_eq!(round.kpis[0].comparator, KpiComparator::Lte);
}

#[test]
fn release_warn_inform_gates_all_parse() {
    let src = format!(
        r#"{HEADER}
[[kpi]]
name       = "release_metric"
kind       = "count"
threshold  = 1.0
comparator = "eq"
gate       = "release"

[[kpi]]
name       = "warn_metric"
kind       = "count"
threshold  = 1.0
comparator = "eq"
gate       = "warn"

[[kpi]]
name       = "inform_metric"
kind       = "count"
threshold  = 1.0
comparator = "eq"
gate       = "inform"
"#
    );
    let m = parse_or_panic(&src);
    assert_eq!(m.kpis[0].gate, KpiGate::Release);
    assert_eq!(m.kpis[1].gate, KpiGate::Warn);
    assert_eq!(m.kpis[2].gate, KpiGate::Inform);
}
