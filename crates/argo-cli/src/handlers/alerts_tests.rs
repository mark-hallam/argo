//! Unit tests for `handlers::alerts` — config parsing, the pure
//! evaluator, and hysteresis. Wired in from `handlers/mod.rs` under
//! `#[cfg(test)]`.

use std::collections::{HashMap, HashSet};

use super::alerts::{
    evaluate_once, load_alerts_config, AlertKind, AlertRule, AlertSnapshot, AlertsConfig,
};

fn make_snapshot() -> AlertSnapshot {
    AlertSnapshot {
        now_micros: 1_700_000_000_000_000,
        spend_usd_by_rule: HashMap::new(),
        error_pct_by_rule: HashMap::new(),
        oldest_pending_approval_age_secs: 0,
    }
}

#[test]
fn parses_minimal_config() {
    let toml_src = r#"
[[rule]]
name = "spend-day"
kind = "spend_total"
threshold_usd = 50.0
window = "24h"
"#;
    let cfg: AlertsConfig = toml::from_str(toml_src).expect("parse minimal cfg");
    assert_eq!(cfg.rules.len(), 1);
    let rule = &cfg.rules[0];
    assert_eq!(rule.name, "spend-day");
    assert!(rule.channels.is_empty());
    match &rule.kind {
        AlertKind::SpendTotal {
            threshold_usd,
            window,
        } => {
            assert!((threshold_usd - 50.0).abs() < f64::EPSILON);
            assert_eq!(window, "24h");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn parses_full_multi_rule_config() {
    let toml_src = r#"
[[rule]]
name = "spend-day"
kind = "spend_total"
threshold_usd = 50.0
window = "24h"
channels = ["file:.skycot/alerts.log", "event"]

[[rule]]
name = "error-rate-actions"
kind = "error_rate"
route = "/v1/actions/dispatch"
threshold_pct = 5.0
window = "10m"
channels = ["file:.skycot/alerts.log"]

[[rule]]
name = "approvals-stuck"
kind = "approval_timeout"
older_than = "30m"
channels = ["event"]
"#;
    let cfg: AlertsConfig = toml::from_str(toml_src).expect("parse full cfg");
    assert_eq!(cfg.rules.len(), 3);
    assert_eq!(cfg.rules[0].channels.len(), 2);
    assert!(matches!(cfg.rules[0].kind, AlertKind::SpendTotal { .. }));
    match &cfg.rules[1].kind {
        AlertKind::ErrorRate {
            route,
            threshold_pct,
            window,
        } => {
            assert_eq!(route, "/v1/actions/dispatch");
            assert!((threshold_pct - 5.0).abs() < f64::EPSILON);
            assert_eq!(window, "10m");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
    match &cfg.rules[2].kind {
        AlertKind::ApprovalTimeout { older_than } => assert_eq!(older_than, "30m"),
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn missing_config_file_returns_empty_cfg() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("does-not-exist.toml");
    let cfg = load_alerts_config(&path).expect("absent → Ok(empty)");
    assert!(cfg.rules.is_empty());
}

fn spend_rule(name: &str, threshold_usd: f64) -> AlertRule {
    AlertRule {
        name: name.to_string(),
        kind: AlertKind::SpendTotal {
            threshold_usd,
            window: "24h".to_string(),
        },
        channels: Vec::new(),
    }
}

#[test]
fn spend_rule_fires_at_threshold() {
    let cfg = AlertsConfig {
        rules: vec![spend_rule("spend-day", 50.0)],
    };
    let mut snap = make_snapshot();
    snap.spend_usd_by_rule
        .insert("spend-day".to_string(), 50.5);
    let mut fired: HashSet<String> = HashSet::new();
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert_eq!(firings.len(), 1);
    let f = &firings[0];
    assert_eq!(f.rule, "spend-day");
    assert_eq!(f.kind, "spend_total");
    assert!(f.observed >= f.threshold);
    assert!(fired.contains("spend-day"));
}

#[test]
fn spend_rule_does_not_re_fire_until_metric_falls_below_threshold() {
    let cfg = AlertsConfig {
        rules: vec![spend_rule("spend-day", 50.0)],
    };
    let mut fired: HashSet<String> = HashSet::new();

    // Tick 1: above threshold → fires.
    let mut snap = make_snapshot();
    snap.spend_usd_by_rule
        .insert("spend-day".to_string(), 60.0);
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert_eq!(firings.len(), 1, "first crossing should fire");

    // Tick 2: still above → suppressed by hysteresis.
    snap.spend_usd_by_rule
        .insert("spend-day".to_string(), 70.0);
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert!(firings.is_empty(), "still-above tick must not re-fire");

    // Tick 3: drops below → re-arms.
    snap.spend_usd_by_rule
        .insert("spend-day".to_string(), 10.0);
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert!(firings.is_empty(), "below-threshold tick is silent");
    assert!(!fired.contains("spend-day"), "rule should have re-armed");

    // Tick 4: above again → fires once more.
    snap.spend_usd_by_rule
        .insert("spend-day".to_string(), 80.0);
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert_eq!(firings.len(), 1, "re-armed rule fires on next crossing");
}

#[test]
fn error_rate_rule_fires() {
    let cfg = AlertsConfig {
        rules: vec![AlertRule {
            name: "err-actions".to_string(),
            kind: AlertKind::ErrorRate {
                route: "/v1/actions/dispatch".to_string(),
                threshold_pct: 5.0,
                window: "10m".to_string(),
            },
            channels: Vec::new(),
        }],
    };
    let mut snap = make_snapshot();
    snap.error_pct_by_rule
        .insert("err-actions".to_string(), 7.5);
    let mut fired: HashSet<String> = HashSet::new();
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert_eq!(firings.len(), 1);
    assert_eq!(firings[0].rule, "err-actions");
    assert_eq!(firings[0].kind, "error_rate");
    assert!((firings[0].observed - 7.5).abs() < f64::EPSILON);
    assert!((firings[0].threshold - 5.0).abs() < f64::EPSILON);
}

#[test]
fn approval_timeout_rule_fires() {
    let cfg = AlertsConfig {
        rules: vec![AlertRule {
            name: "approvals-stuck".to_string(),
            kind: AlertKind::ApprovalTimeout {
                older_than: "30m".to_string(),
            },
            channels: Vec::new(),
        }],
    };
    let mut snap = make_snapshot();
    snap.oldest_pending_approval_age_secs = 45 * 60;
    let mut fired: HashSet<String> = HashSet::new();
    let firings = evaluate_once(&cfg, &snap, &mut fired);
    assert_eq!(firings.len(), 1);
    assert_eq!(firings[0].rule, "approvals-stuck");
    assert_eq!(firings[0].kind, "approval_timeout");
    assert!((firings[0].threshold - (30.0 * 60.0)).abs() < f64::EPSILON);
    assert!(firings[0].observed >= firings[0].threshold);
}
