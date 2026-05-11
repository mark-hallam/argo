//! Unit tests for `[budget.ceiling]` parsing.
use super::*;

#[test]
fn all_uncapped_parses_clean() {
    let toml = "";
    let s: BudgetCeilingSection = toml::from_str(toml).unwrap();
    assert!(s.per_session_usd.is_none());
    assert!(s.per_day_usd.is_none());
    assert!(s.per_month_usd.is_none());
    let m = s.to_micros().unwrap();
    assert!(m.per_session_usd_micros.is_none());
    assert!(m.per_day_usd_micros.is_none());
    assert!(m.per_month_usd_micros.is_none());
}

#[test]
fn all_three_set_converts_to_micros_round_trip() {
    let toml = "per_session_usd = 5.0\nper_day_usd = 100.0\nper_month_usd = 2000.0\n";
    let s: BudgetCeilingSection = toml::from_str(toml).unwrap();
    let m = s.to_micros().unwrap();
    assert_eq!(m.per_session_usd_micros, Some(5_000_000));
    assert_eq!(m.per_day_usd_micros, Some(100_000_000));
    assert_eq!(m.per_month_usd_micros, Some(2_000_000_000));
}

#[test]
fn negative_value_rejects_with_schema_error() {
    let s = BudgetCeilingSection {
        per_day_usd: Some(-1.0),
        ..Default::default()
    };
    let err = s.to_micros().expect_err("negative is invalid");
    match err {
        ArgoError::Schema(msg) => {
            assert!(msg.contains("per_day_usd"), "names the offending field");
            assert!(msg.contains("non-negative"), "explains the rule");
        }
        other => panic!("expected Schema error, got {other:?}"),
    }
}

#[test]
fn explicit_zero_is_permitted_as_kill_switch() {
    // 0.00 is an explicit, opinionated choice — keep the surface
    // honest by allowing it. Operators that intend "uncapped"
    // omit the line.
    let s = BudgetCeilingSection {
        per_session_usd: Some(0.0),
        per_day_usd: None,
        per_month_usd: None,
    };
    let m = s.to_micros().unwrap();
    assert_eq!(m.per_session_usd_micros, Some(0));
    assert!(m.per_day_usd_micros.is_none());
}

#[test]
fn mixed_uncapped_and_set_round_trips() {
    let toml = "per_day_usd = 50.0\n";
    let s: BudgetCeilingSection = toml::from_str(toml).unwrap();
    let m = s.to_micros().unwrap();
    assert!(m.per_session_usd_micros.is_none());
    assert_eq!(m.per_day_usd_micros, Some(50_000_000));
    assert!(m.per_month_usd_micros.is_none());
}

#[test]
fn non_finite_value_rejects() {
    let s = BudgetCeilingSection {
        per_month_usd: Some(f64::NAN),
        ..Default::default()
    };
    assert!(matches!(s.to_micros(), Err(ArgoError::Schema(_))));
    let s = BudgetCeilingSection {
        per_month_usd: Some(f64::INFINITY),
        ..Default::default()
    };
    assert!(matches!(s.to_micros(), Err(ArgoError::Schema(_))));
}
