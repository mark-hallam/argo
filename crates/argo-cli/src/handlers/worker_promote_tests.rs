//! Unit tests for `worker_promote` — the promotion eligibility
//! engine. Focused on the pure-function gap math + criteria lookup;
//! end-to-end DB persistence + audit-row writes are covered by the
//! `tests/worker_promote_smoke.rs` integration test.

use super::*;

fn metrics(sessions: u64, granted: u64, revoked: u64, tenure_days: i64) -> PromotionMetrics {
    let denom = granted + revoked;
    #[allow(clippy::cast_precision_loss)]
    let success_rate = if denom > 0 {
        Some(granted as f64 / denom as f64)
    } else {
        None
    };
    PromotionMetrics {
        sessions,
        granted,
        revoked,
        success_rate,
        tenure_days,
    }
}

#[test]
fn entry_to_junior_criteria_match_brief() {
    let c = criteria_for(WorkerLevel::Junior).unwrap();
    assert_eq!(c.sessions, 25);
    assert!((c.success_rate - 0.80).abs() < f64::EPSILON);
    assert_eq!(c.tenure_days, 14);
    assert!(!c.requires_force);
}

#[test]
fn junior_to_senior_criteria_match_brief() {
    let c = criteria_for(WorkerLevel::Senior).unwrap();
    assert_eq!(c.sessions, 100);
    assert!((c.success_rate - 0.90).abs() < f64::EPSILON);
    assert_eq!(c.tenure_days, 60);
    assert!(!c.requires_force);
}

#[test]
fn senior_to_lead_requires_force() {
    let c = criteria_for(WorkerLevel::Lead).unwrap();
    assert_eq!(c.sessions, 500);
    assert!((c.success_rate - 0.92).abs() < f64::EPSILON);
    assert_eq!(c.tenure_days, 180);
    assert!(c.requires_force);
}

#[test]
fn entry_has_no_inbound_criteria() {
    assert!(criteria_for(WorkerLevel::Entry).is_none());
}

#[test]
fn gap_marks_all_met_when_thresholds_cleared() {
    let c = criteria_for(WorkerLevel::Junior).unwrap();
    let m = metrics(30, 9, 1, 20);
    let gap = evaluate_gap(&m, &c);
    assert!(gap.all_met);
    assert!(gap.sessions_short <= 0);
    assert!(gap.tenure_days_short <= 0);
    assert!(gap.success_rate_short <= 0.0);
}

#[test]
fn gap_reports_each_axis_shortfall_separately() {
    let c = criteria_for(WorkerLevel::Junior).unwrap();
    let m = metrics(2, 1, 4, 10);
    let gap = evaluate_gap(&m, &c);
    assert!(!gap.all_met);
    assert_eq!(gap.sessions_short, 23);
    assert_eq!(gap.tenure_days_short, 4);
    assert!(gap.success_rate_short > 0.0);
}

#[test]
fn gap_treats_no_decisions_as_zero_success_rate() {
    let c = criteria_for(WorkerLevel::Junior).unwrap();
    let m = metrics(40, 0, 0, 30);
    let gap = evaluate_gap(&m, &c);
    assert!(!gap.all_met);
    assert!((gap.success_rate_short - c.success_rate).abs() < f64::EPSILON);
    assert!(gap.sessions_short <= 0);
    assert!(gap.tenure_days_short <= 0);
}

#[test]
fn format_gap_renders_brief_example_phrasing() {
    let gap = PromotionGap {
        sessions_short: 23,
        success_rate_short: 0.0,
        tenure_days_short: 4,
        all_met: false,
    };
    let s = format_gap(&gap);
    assert!(s.contains("23 sessions short"), "{s}");
    assert!(s.contains("4d tenure short"), "{s}");
}

#[test]
fn format_gap_returns_criteria_met_when_all_clear() {
    let gap = PromotionGap {
        sessions_short: -5,
        success_rate_short: -0.10,
        tenure_days_short: -2,
        all_met: true,
    };
    assert_eq!(format_gap(&gap), "criteria met");
}

#[test]
fn worker_level_next_walks_the_ladder() {
    assert_eq!(WorkerLevel::Entry.next(), Some(WorkerLevel::Junior));
    assert_eq!(WorkerLevel::Junior.next(), Some(WorkerLevel::Senior));
    assert_eq!(WorkerLevel::Senior.next(), Some(WorkerLevel::Lead));
    assert_eq!(WorkerLevel::Lead.next(), None);
}

#[test]
fn worker_level_round_trips_through_string() {
    use std::str::FromStr;
    for &l in &[
        WorkerLevel::Entry,
        WorkerLevel::Junior,
        WorkerLevel::Senior,
        WorkerLevel::Lead,
    ] {
        assert_eq!(WorkerLevel::from_str(l.as_str()).unwrap(), l);
        assert_eq!(WorkerLevel::from_str(&l.to_string()).unwrap(), l);
    }
    assert!(WorkerLevel::from_str("principal").is_err());
}
