//! Unit tests for the auto-approval evaluator. Loaded via
//! `#[cfg(test)] #[path = ...] mod tests;` from
//! `auto_approval.rs` to keep that file under the per-file
//! 500-LOC budget.

use super::*;
use chrono::TimeZone;

fn pending(risk: &'static str, requires: bool) -> PendingAction<'static> {
    PendingAction {
        risk_level: risk,
        requires_approval: requires,
        estimated_usd_micros: None,
        action_type: "test_action",
    }
}

fn pending_at(
    risk: &'static str,
    requires: bool,
    estimate_usd_micros: u64,
) -> PendingAction<'static> {
    PendingAction {
        risk_level: risk,
        requires_approval: requires,
        estimated_usd_micros: Some(estimate_usd_micros),
        action_type: "test_action",
    }
}

fn now() -> DateTime<Utc> {
    // Wed 2026-05-06 12:00:00 UTC
    Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()
}

fn enabled_default() -> AutoApprovalPolicy {
    AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    }
}

#[test]
fn autonomous_safe_action_always_auto_approves() {
    let policy = AutoApprovalPolicy::default();
    let p = pending("low", false);
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), now());
    assert!(d.is_auto_approved());
    assert!(d.reason().contains("manifest:requires_approval=false"));
}

#[test]
fn disabled_policy_routes_every_approval_gated_action_to_human() {
    let policy = AutoApprovalPolicy::default();
    let p = pending("low", true);
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), now());
    assert!(matches!(d, Decision::Ask { .. }));
    assert!(d.reason().contains("policy:disabled"));
}

#[test]
fn enabled_policy_auto_approves_low_risk_within_caps() {
    let policy = enabled_default();
    let p = pending("low", true);
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), now());
    assert!(d.is_auto_approved());
}

#[test]
fn high_risk_actions_never_auto_approve_even_when_enabled() {
    let policy = enabled_default();
    let p = pending("high", true);
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), now());
    assert!(matches!(d, Decision::Ask { .. }));
    assert!(d.reason().contains("high_risk"));
}

#[test]
fn per_action_cap_routes_overage_to_human() {
    let policy = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(dollars_to_micros(0.50)),
        ..AutoApprovalPolicy::default()
    };
    // $1 estimate > $0.50 cap → Ask
    let p = pending_at("low", true, dollars_to_micros(1.0));
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), now());
    assert!(matches!(d, Decision::Ask { .. }));
    assert!(d.reason().contains("per_action_cap"));
}

#[test]
fn daily_cap_blocks_when_projected_total_exceeds() {
    let policy = AutoApprovalPolicy {
        enabled: true,
        daily_budget_usd_micros: Some(dollars_to_micros(10.0)),
        ..AutoApprovalPolicy::default()
    };
    let spent = SpendSnapshot {
        today_usd_micros: dollars_to_micros(9.50),
        this_month_usd_micros: 0,
    };
    // Today=$9.50 + this=$1 → $10.50 > $10 cap → Block
    let p = pending_at("low", true, dollars_to_micros(1.0));
    let d = evaluate(&policy, &p, spent, now());
    match d {
        Decision::Block { reason } => assert!(reason.contains("daily_cap")),
        other => panic!("expected Block, got {other:?}"),
    }
}

#[test]
fn time_window_outside_business_hours_routes_to_human() {
    let policy = AutoApprovalPolicy {
        enabled: true,
        time_windows: vec![TimeWindow {
            days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        }],
        ..AutoApprovalPolicy::default()
    };
    // Sat 10am — not a weekday in the window.
    let sat = Utc.with_ymd_and_hms(2026, 5, 9, 10, 0, 0).unwrap();
    let p = pending("low", true);
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), sat);
    assert!(matches!(d, Decision::Ask { .. }));
    assert!(d.reason().contains("outside_time_window"));
}

#[test]
fn time_window_inside_business_hours_auto_approves() {
    let policy = AutoApprovalPolicy {
        enabled: true,
        time_windows: vec![TimeWindow {
            days: vec![Weekday::Wed],
            start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        }],
        ..AutoApprovalPolicy::default()
    };
    // Wed 12:00 — inside the window.
    let p = pending("low", true);
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), now());
    assert!(d.is_auto_approved());
}

#[test]
fn cross_midnight_window_matches_late_night_and_early_morning() {
    // 22:00 → 06:00 wraps past midnight
    let policy = AutoApprovalPolicy {
        enabled: true,
        time_windows: vec![TimeWindow {
            days: vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
                Weekday::Sat,
                Weekday::Sun,
            ],
            start: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        }],
        ..AutoApprovalPolicy::default()
    };
    let p = pending("low", true);
    // Wed 23:30 — inside the wrapping window.
    let late = Utc.with_ymd_and_hms(2026, 5, 6, 23, 30, 0).unwrap();
    assert!(evaluate(&policy, &p, SpendSnapshot::zero(), late).is_auto_approved());
    // Wed 03:30 — also inside.
    let early = Utc.with_ymd_and_hms(2026, 5, 6, 3, 30, 0).unwrap();
    assert!(evaluate(&policy, &p, SpendSnapshot::zero(), early).is_auto_approved());
    // Wed 12:00 — outside (between end and start).
    let mid = now();
    assert!(matches!(
        evaluate(&policy, &p, SpendSnapshot::zero(), mid),
        Decision::Ask { .. }
    ));
}

#[test]
fn wake_action_type_bypasses_time_window() {
    let policy = AutoApprovalPolicy {
        enabled: true,
        time_windows: vec![TimeWindow {
            days: vec![Weekday::Wed],
            start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        }],
        wake_action_types: vec!["page_oncall".into()],
        ..AutoApprovalPolicy::default()
    };
    // Sat (outside the day) — would normally Ask, but the
    // urgent action_type bypasses.
    let sat = Utc.with_ymd_and_hms(2026, 5, 9, 23, 0, 0).unwrap();
    let p = PendingAction {
        risk_level: "low",
        requires_approval: true,
        estimated_usd_micros: None,
        action_type: "page_oncall",
    };
    let d = evaluate(&policy, &p, SpendSnapshot::zero(), sat);
    assert!(d.is_auto_approved(), "wake action should bypass: got {d:?}");
}

#[test]
fn dollars_micros_round_trip() {
    assert_eq!(dollars_to_micros(0.50), 500_000);
    assert_eq!(micros_to_dollars(500_000), 0.50);
    assert_eq!(dollars_to_micros(0.0), 0);
    // Negative dollars clamp to 0.
    assert_eq!(dollars_to_micros(-1.0), 0);
}

#[test]
fn decision_reason_round_trips() {
    let d = Decision::AutoApprove {
        reason: "test".into(),
    };
    assert_eq!(d.reason(), "test");
    assert!(d.is_auto_approved());

    let d = Decision::Ask {
        channel: Some("email".into()),
        reason: "needed".into(),
    };
    assert_eq!(d.reason(), "needed");
    assert!(!d.is_auto_approved());

    let d = Decision::Block {
        reason: "too costly".into(),
    };
    assert_eq!(d.reason(), "too costly");
    assert!(!d.is_auto_approved());
}
