//! Pure-function tests for `routes_worker_overview` — no DB / env lock so
//! they can run in parallel with the data + status suites.

use super::*;
use chrono::{Duration, Utc};

#[test]
fn classify_source_recovers_bullet_source_via_format_probe() {
    let cs = bullets_for_vertical("customer-support");
    assert_eq!(cs.len(), 3);
    let kinds: Vec<SourceKind> = cs.iter().map(classify_source).collect();
    assert_eq!(kinds[0], SourceKind::ToolCalls);
    assert_eq!(kinds[1], SourceKind::NotComputable);
    assert_eq!(kinds[2], SourceKind::Pending);
    let fallback = bullets_for_vertical("totally-unknown-vertical");
    assert!(fallback
        .iter()
        .map(classify_source)
        .any(|k| k == SourceKind::AnyAction));
}

#[test]
fn label_from_template_strips_count_prefix_and_capitalises() {
    assert_eq!(label_from_template("{n} replies sent"), "Replies sent");
    assert_eq!(
        label_from_template("{n} pending escalations"),
        "Pending escalations"
    );
    assert_eq!(
        label_from_template("revenue this week"),
        "Revenue this week"
    );
}

#[test]
fn slugify_lowercases_and_collapses_non_alnum() {
    assert_eq!(slugify("Replies sent"), "replies_sent");
    assert_eq!(slugify("Avg response time"), "avg_response_time");
    assert_eq!(slugify("Pending escalations"), "pending_escalations");
}

#[test]
fn trend_for_up_down_flat_and_prev_zero_special_cases() {
    let (dir, pct) = trend_for(120.0, 100.0);
    assert_eq!(dir, "up");
    assert!((pct - 20.0).abs() < 0.05);
    let (dir, pct) = trend_for(80.0, 100.0);
    assert_eq!(dir, "down");
    assert!((pct + 20.0).abs() < 0.05);
    let (dir, pct) = trend_for(101.0, 100.0);
    assert_eq!(dir, "flat");
    assert!(pct.abs() < 2.0);
    let (dir, _) = trend_for(5.0, 0.0);
    assert_eq!(dir, "up");
    let (dir, pct) = trend_for(0.0, 0.0);
    assert_eq!(dir, "flat");
    assert!(pct.abs() < 0.01);
}

#[test]
fn next_fire_for_returns_a_future_instant_when_cron_is_valid() {
    let now = Utc::now();
    let fire = next_fire_for("0 6 * * * *", None, now).expect("cron parsable");
    assert!(fire >= now);
    assert!(fire <= now + Duration::days(1));
    assert!(next_fire_for("not a cron", None, now).is_none());
}

#[test]
fn parse_role_header_recognises_canonical_roles_and_aliases() {
    fn req(value: &str) -> String {
        format!("GET / HTTP/1.1\r\nHost: x\r\nX-Argo-Role: {value}\r\n\r\n")
    }
    assert_eq!(
        parse_role_header(&req("operator")),
        Some(PrincipalRole::Operator)
    );
    assert_eq!(
        parse_role_header(&req("delegator")),
        Some(PrincipalRole::Delegator)
    );
    assert_eq!(
        parse_role_header(&req("auditor")),
        Some(PrincipalRole::Auditor)
    );
    assert_eq!(
        parse_role_header(&req("compliance-officer")),
        Some(PrincipalRole::ComplianceOfficer)
    );
    assert_eq!(
        parse_role_header(&req("compliance_officer")),
        Some(PrincipalRole::ComplianceOfficer)
    );
    assert!(parse_role_header(&req("nonsense")).is_none());
    assert!(parse_role_header("GET / HTTP/1.1\r\n\r\n").is_none());
}
