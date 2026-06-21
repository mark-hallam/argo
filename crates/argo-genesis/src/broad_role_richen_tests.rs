//! Unit tests for the broad-role richening helpers.

use super::*;

#[test]
fn whitelist_keeps_only_wired_connectors() {
    let raw = vec![
        "Gmail".to_string(),         // case-insensitive → "gmail"
        "  slack ".to_string(),      // trimmed → "slack"
        "MyCustomCRM".to_string(),   // not wired → dropped
        "gmail".to_string(),         // duplicate → collapsed
        "salesforce".to_string(),    // wired
    ];
    let kept = whitelist_access(&raw);
    assert_eq!(kept, vec!["gmail", "slack", "salesforce"]);
}

#[test]
fn invented_access_is_dropped() {
    let raw = vec!["RandomTool".to_string(), "internal_db".to_string()];
    assert!(
        whitelist_access(&raw).is_empty(),
        "an invented integration must never survive the whitelist",
    );
}

#[test]
fn canonical_connector_matches_case_insensitively() {
    assert_eq!(canonical_connector("HubSpot"), Some("hubspot"));
    assert_eq!(canonical_connector(" zendesk "), Some("zendesk"));
    assert_eq!(canonical_connector("nope"), None);
}

#[test]
fn cron_shape_accepts_5_and_6_fields() {
    assert!(is_cron_shaped("0 9 * * 1-5"));
    assert!(is_cron_shaped("0 0 9 * * 1-5")); // 6-field
    assert!(is_cron_shaped("*/15 * * * *"));
}

#[test]
fn cron_shape_rejects_free_text() {
    assert!(!is_cron_shaped("every weekday at 9am"));
    assert!(!is_cron_shaped("daily"));
    assert!(!is_cron_shaped("")); // empty
    assert!(!is_cron_shaped("0 9 * *")); // 4 fields
}

#[test]
fn cadence_from_cron_drops_non_cron() {
    assert_eq!(cadence_from_cron(" 0 9 * * 1-5 "), Some("0 9 * * 1-5".to_string()));
    assert_eq!(cadence_from_cron("weekday mornings"), None);
}

#[test]
fn typed_kpi_parses_full_object() {
    let v = serde_json::json!({
        "name": "on-time dispatch rate",
        "kind": "percentage",
        "target": 95.0,
        "comparator": "gte"
    });
    let c = parse_success_criterion(&v).expect("well-formed criterion parses");
    assert_eq!(c.name, "on-time dispatch rate");
    assert_eq!(c.kind, KpiKind::Percentage);
    assert_eq!(c.comparator, KpiComparator::Gte);
    assert!((c.target - 95.0).abs() < f64::EPSILON);
}

#[test]
fn typed_kpi_defaults_unknown_kind_and_comparator() {
    let v = serde_json::json!({ "name": "kept goal" });
    let c = parse_success_criterion(&v).expect("name-only criterion still parses");
    assert_eq!(c.kind, KpiKind::Count, "missing kind defaults to Count");
    assert_eq!(c.comparator, KpiComparator::Gte, "missing comparator defaults to Gte");
    assert!((c.target - 1.0).abs() < f64::EPSILON, "missing target floors at 1.0");
}

#[test]
fn typed_kpi_drops_nameless() {
    let v = serde_json::json!({ "kind": "count", "target": 3 });
    assert!(parse_success_criterion(&v).is_none(), "a nameless KPI is dropped");
}

#[test]
fn typed_kpi_maps_lte_and_eq() {
    let lte = serde_json::json!({ "name": "errors", "kind": "count", "target": 2, "comparator": "lte" });
    assert_eq!(parse_success_criterion(&lte).unwrap().comparator, KpiComparator::Lte);
    let eq = serde_json::json!({ "name": "exact", "comparator": "eq" });
    assert_eq!(parse_success_criterion(&eq).unwrap().comparator, KpiComparator::Eq);
}
