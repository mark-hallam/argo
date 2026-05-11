//! Unit tests for `spend_ceiling`. The store-side round-trip is
//! covered by `tests/spend_ceiling_smoke.rs`; these tests stay
//! pure-function so they don't touch SQLite.
use super::*;
use argo_types::SpendCounters;

fn cfg(session: Option<u64>, day: Option<u64>, month: Option<u64>) -> CeilingConfig {
    CeilingConfig {
        per_session_usd_micros: session,
        per_day_usd_micros: day,
        per_month_usd_micros: month,
    }
}

fn counters(session: u64, day: u64, month: u64) -> SpendCounters {
    SpendCounters {
        session_used: session,
        day_used: day,
        month_used: month,
    }
}

#[test]
fn uncapped_all_three_windows_is_no_op() {
    let check = check_from_counters(counters(999, 999, 999), &CeilingConfig::uncapped());
    assert!(check.session_remaining.is_none());
    assert!(check.day_remaining.is_none());
    assert!(check.month_remaining.is_none());
    // Even a huge projected cost passes through when nothing is set.
    check.project(u64::MAX).expect("uncapped never blocks");
}

#[test]
fn session_cap_only_blocks_session_window() {
    let check = check_from_counters(counters(500, 0, 0), &cfg(Some(1_000), None, None));
    assert_eq!(check.session_remaining, Some(500));
    assert!(check.day_remaining.is_none());
    assert!(check.month_remaining.is_none());
    let err = check.project(501).expect_err("over session cap");
    match err {
        ArgoError::BudgetExceeded {
            window,
            ceiling_micros,
            ..
        } => {
            assert_eq!(window, BudgetWindow::Session);
            assert_eq!(ceiling_micros, 1_000);
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn day_cap_only_blocks_day_window() {
    let check = check_from_counters(counters(0, 800, 0), &cfg(None, Some(1_000), None));
    let err = check.project(201).expect_err("over day cap");
    match err {
        ArgoError::BudgetExceeded { window, .. } => {
            assert_eq!(window, BudgetWindow::Day);
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn month_cap_only_blocks_month_window() {
    let check = check_from_counters(counters(0, 0, 1_000), &cfg(None, None, Some(1_500)));
    let err = check.project(501).expect_err("over month cap");
    match err {
        ArgoError::BudgetExceeded { window, .. } => {
            assert_eq!(window, BudgetWindow::Month);
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn all_three_set_session_fires_first() {
    // Each window would individually fail; the projection short-
    // circuits at the first (Session) so callers get the smallest-
    // scope diagnostic. Locks the iteration order.
    let check = check_from_counters(
        counters(900, 900, 900),
        &cfg(Some(1_000), Some(1_000), Some(1_000)),
    );
    let err = check.project(200).expect_err("any of the three would fail");
    match err {
        ArgoError::BudgetExceeded { window, .. } => {
            assert_eq!(window, BudgetWindow::Session);
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn exact_boundary_projected_equals_remaining_is_allowed() {
    // remaining = ceiling - used = 1000 - 500 = 500.
    // projected = 500 → after = 1000, equal to ceiling: allowed.
    let check = check_from_counters(counters(500, 0, 0), &cfg(Some(1_000), None, None));
    assert_eq!(check.session_remaining, Some(500));
    check.project(500).expect("boundary is inclusive");
}

#[test]
fn exceeds_by_one_micro_is_rejected() {
    // remaining = 500; projected = 501 → strict overshoot.
    let check = check_from_counters(counters(500, 0, 0), &cfg(Some(1_000), None, None));
    let err = check.project(501).expect_err("one over the boundary");
    match err {
        ArgoError::BudgetExceeded {
            used_micros,
            ceiling_micros,
            projected_cost_micros,
            ..
        } => {
            assert_eq!(used_micros, 500);
            assert_eq!(ceiling_micros, 1_000);
            assert_eq!(projected_cost_micros, 501);
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[test]
fn remaining_clamps_to_zero_when_already_over() {
    // If the ledger has already recorded more than the ceiling
    // (e.g. operator dropped the cap mid-flight), remaining
    // saturates at 0 — every projected cost > 0 fails.
    let check = check_from_counters(counters(2_000, 0, 0), &cfg(Some(1_000), None, None));
    assert_eq!(check.session_remaining, Some(0));
    assert!(check.project(1).is_err());
    // Projected = 0 is still a pass-through (no charge implied).
    check.project(0).expect("zero charge always passes");
}

#[test]
fn all_uncapped_helper_is_true_iff_all_three_none() {
    assert!(CeilingConfig::uncapped().all_uncapped());
    assert!(!cfg(Some(1), None, None).all_uncapped());
    assert!(!cfg(None, Some(1), None).all_uncapped());
    assert!(!cfg(None, None, Some(1)).all_uncapped());
}
