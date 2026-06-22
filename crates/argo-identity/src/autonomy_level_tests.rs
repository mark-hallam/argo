//! Unit tests for the DEC-060 autonomy-level → spend-envelope mapping.
use super::*;

#[test]
fn from_label_maps_storage_and_display_labels() {
    assert_eq!(AutonomyLevel::from_label("always_ask"), Some(AutonomyLevel::AlwaysAsk));
    assert_eq!(AutonomyLevel::from_label("none"), Some(AutonomyLevel::AlwaysAsk));
    assert_eq!(AutonomyLevel::from_label("conservative"), Some(AutonomyLevel::AcceptSafe));
    assert_eq!(AutonomyLevel::from_label("accept_safe"), Some(AutonomyLevel::AcceptSafe));
    assert_eq!(AutonomyLevel::from_label("balanced"), Some(AutonomyLevel::Auto));
    assert_eq!(AutonomyLevel::from_label("auto"), Some(AutonomyLevel::Auto));
    assert_eq!(AutonomyLevel::from_label("aggressive"), Some(AutonomyLevel::Bypass));
    assert_eq!(AutonomyLevel::from_label("bypass"), Some(AutonomyLevel::Bypass));
    assert_eq!(AutonomyLevel::from_label("garbage"), None);
}

#[test]
fn label_roundtrips_through_from_label() {
    for lvl in [
        AutonomyLevel::AlwaysAsk,
        AutonomyLevel::AcceptSafe,
        AutonomyLevel::Auto,
        AutonomyLevel::Bypass,
    ] {
        assert_eq!(AutonomyLevel::from_label(lvl.label()), Some(lvl));
    }
}

#[test]
fn always_ask_disables_auto_approval() {
    let p = AutonomyLevel::AlwaysAsk.policy();
    assert!(!p.enabled);
    assert!(p.max_usd_micros_per_action.is_none());
    assert!(p.daily_budget_usd_micros.is_none());
}

#[test]
fn higher_rungs_widen_the_envelope_monotonically() {
    let safe = AutonomyLevel::AcceptSafe.policy();
    let auto = AutonomyLevel::Auto.policy();
    let bypass = AutonomyLevel::Bypass.policy();
    assert!(safe.enabled && auto.enabled && bypass.enabled);
    let per = |p: &AutoApprovalPolicy| p.max_usd_micros_per_action.unwrap();
    let day = |p: &AutoApprovalPolicy| p.daily_budget_usd_micros.unwrap();
    assert!(per(&safe) < per(&auto) && per(&auto) < per(&bypass));
    assert!(day(&safe) < day(&auto) && day(&auto) < day(&bypass));
}
