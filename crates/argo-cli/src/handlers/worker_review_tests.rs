//! Unit tests for `worker_review` (mission claim 2 — periodic
//! review-cycle CLI). Period arithmetic, KPI delta math, success
//! tally, and store-driven build_report on a fixture pool.

use super::{
    build_report, first_of_month, first_of_quarter, kpi_deltas_from_rows, parse_since,
    success_from_kinds, Format, Period,
};
use crate::handlers::worker_review_md::{format_rate, render_markdown};
use argo_store::SessionStore;
use argo_store::worker_kpi;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

#[test]
fn period_parse_accepts_aliases_and_rejects_garbage() {
    assert_eq!(Period::parse("quarterly").unwrap(), Period::Quarterly);
    assert_eq!(Period::parse("q").unwrap(), Period::Quarterly);
    assert_eq!(Period::parse("monthly").unwrap(), Period::Monthly);
    assert_eq!(Period::parse("weekly").unwrap(), Period::Weekly);
    assert!(Period::parse("yearly").is_err());
}

#[test]
fn format_parse_accepts_aliases_and_rejects_garbage() {
    assert_eq!(Format::parse("markdown").unwrap(), Format::Markdown);
    assert_eq!(Format::parse("md").unwrap(), Format::Markdown);
    assert_eq!(Format::parse("json").unwrap(), Format::Json);
    assert!(Format::parse("yaml").is_err());
}

#[test]
fn first_of_quarter_buckets_each_calendar_month() {
    let q1 = Utc.with_ymd_and_hms(2026, 2, 14, 9, 0, 0).unwrap();
    let q2 = Utc.with_ymd_and_hms(2026, 5, 10, 9, 0, 0).unwrap();
    let q3 = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
    let q4 = Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap();
    assert_eq!(
        first_of_quarter(q1),
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    );
    assert_eq!(
        first_of_quarter(q2),
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap()
    );
    assert_eq!(
        first_of_quarter(q3),
        Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap()
    );
    assert_eq!(
        first_of_quarter(q4),
        Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap()
    );
}

#[test]
fn first_of_month_drops_the_day_and_time_components() {
    let now = Utc.with_ymd_and_hms(2026, 5, 10, 12, 34, 56).unwrap();
    assert_eq!(
        first_of_month(now),
        Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap()
    );
}

#[test]
fn weekly_period_rolls_back_seven_days() {
    let now = Utc.with_ymd_and_hms(2026, 5, 10, 12, 0, 0).unwrap();
    let start = Period::Weekly.start_for(now);
    assert_eq!(start, Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap());
}

#[test]
fn parse_since_round_trips_rfc3339() {
    let dt = parse_since("2026-04-01T00:00:00Z").unwrap();
    assert_eq!(dt, Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap());
    assert!(parse_since("not a date").is_err());
}

#[test]
fn kpi_delta_uses_last_pre_period_value_as_baseline() {
    let period_start = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let pre = Utc.with_ymd_and_hms(2026, 3, 30, 12, 0, 0).unwrap();
    let later_pre = Utc.with_ymd_and_hms(2026, 3, 31, 23, 0, 0).unwrap();
    let mid = Utc.with_ymd_and_hms(2026, 4, 5, 9, 0, 0).unwrap();
    let latest = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap();
    let rows = vec![
        ("csat".into(), pre.timestamp_micros(), 4.0),
        ("csat".into(), later_pre.timestamp_micros(), 4.2),
        ("csat".into(), mid.timestamp_micros(), 4.5),
        ("csat".into(), latest.timestamp_micros(), 4.8),
    ];
    let deltas = kpi_deltas_from_rows(&rows, period_start);
    assert_eq!(deltas.len(), 1);
    let row = &deltas[0];
    assert_eq!(row.kpi, "csat");
    assert_eq!(row.baseline, Some(4.2));
    assert_eq!(row.latest, 4.8);
    assert_eq!(row.delta, Some(4.8 - 4.2));
}

#[test]
fn kpi_delta_no_baseline_when_no_pre_period_reading() {
    let period_start = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let only = Utc.with_ymd_and_hms(2026, 4, 5, 0, 0, 0).unwrap();
    let rows = vec![("reply_sla_hours".into(), only.timestamp_micros(), 18.0)];
    let deltas = kpi_deltas_from_rows(&rows, period_start);
    assert_eq!(deltas.len(), 1);
    assert!(deltas[0].baseline.is_none());
    assert!(deltas[0].delta.is_none());
    assert_eq!(deltas[0].latest, 18.0);
}

#[test]
fn success_tally_counts_each_audit_kind_and_computes_rate() {
    let kinds = vec![
        "approval.granted".to_string(),
        "approval.granted".to_string(),
        "approval.granted".to_string(),
        "approval.revoked".to_string(),
        "approval.consumed".to_string(),
    ];
    let s = success_from_kinds(&kinds);
    assert_eq!(s.total, 5);
    assert_eq!(s.granted, 3);
    assert_eq!(s.revoked, 1);
    assert_eq!(s.consumed, 1);
    assert_eq!(s.approval_rate, Some(3.0 / 4.0));
}

#[test]
fn success_rate_is_none_with_no_grants_or_revokes() {
    let kinds = vec!["approval.consumed".to_string()];
    let s = success_from_kinds(&kinds);
    assert_eq!(s.total, 1);
    assert!(s.approval_rate.is_none());
}

#[test]
fn format_rate_renders_percentage_or_na() {
    assert_eq!(format_rate(Some(0.625)), "62.50%");
    assert_eq!(format_rate(None), "n/a");
}

#[test]
fn render_markdown_includes_all_six_sections() {
    use super::{
        AnomalySection, FeedbackSample, HandoffSample, KpiDelta, ReviewReport, SuccessSection,
        SummarySection,
    };
    let report = ReviewReport {
        worker_id: "11111111-2222-7333-8444-555555555555".into(),
        worker_name: Some("Maya".into()),
        period: "quarterly".into(),
        period_start_rfc3339: "2026-04-01T00:00:00+00:00".into(),
        generated_at_rfc3339: "2026-05-10T12:00:00+00:00".into(),
        summary: SummarySection {
            session_count: 12,
            last_session_rfc3339: Some("2026-05-09T17:00:00+00:00".into()),
        },
        kpis: vec![KpiDelta {
            kpi: "csat".into(),
            baseline: Some(4.2),
            latest: 4.8,
            delta: Some(0.6),
            latest_at_rfc3339: "2026-05-01T00:00:00+00:00".into(),
        }],
        success: SuccessSection {
            granted: 9,
            revoked: 1,
            consumed: 7,
            total: 17,
            approval_rate: Some(0.9),
        },
        feedback: vec![FeedbackSample {
            vote: "up".into(),
            action_id: "act-1".into(),
            comment: "great work".into(),
            given_at_rfc3339: "2026-05-08T09:00:00+00:00".into(),
        }],
        handoffs_sent: vec![HandoffSample {
            handoff_id: "h-1".into(),
            counterpart: "wkr-b".into(),
            status: "settled".into(),
            expected_outcome: "shipped".into(),
            created_at_rfc3339: "2026-05-07T09:00:00+00:00".into(),
        }],
        handoffs_received: vec![],
        anomalies: Some(AnomalySection { count: 2 }),
    };
    let md = render_markdown(&report);
    assert!(md.starts_with("# Worker Maya — quarterly review"));
    for section in [
        "## Summary",
        "## KPIs",
        "## Success rate",
        "## Operator feedback",
        "## Handoffs",
        "## Anomalies",
    ] {
        assert!(md.contains(section), "missing {section} in:\n{md}");
    }
    assert!(md.contains("Sessions: **12**"));
    assert!(md.contains("Approval rate: **90.00%**"));
    assert!(md.contains("`up` on `act-1`"));
}

#[test]
fn render_markdown_uses_placeholders_when_sections_empty() {
    use super::{
        ReviewReport, SuccessSection, SummarySection,
    };
    let report = ReviewReport {
        worker_id: "ZERO".into(),
        worker_name: None,
        period: "weekly".into(),
        period_start_rfc3339: "2026-05-03T00:00:00+00:00".into(),
        generated_at_rfc3339: "2026-05-10T00:00:00+00:00".into(),
        summary: SummarySection {
            session_count: 0,
            last_session_rfc3339: None,
        },
        kpis: vec![],
        success: SuccessSection {
            granted: 0,
            revoked: 0,
            consumed: 0,
            total: 0,
            approval_rate: None,
        },
        feedback: vec![],
        handoffs_sent: vec![],
        handoffs_received: vec![],
        anomalies: None,
    };
    let md = render_markdown(&report);
    assert!(md.contains("# Worker (unknown worker) — weekly review"));
    assert!(md.contains("_No KPI observations._"));
    assert!(md.contains("_No operator feedback._"));
    assert!(md.contains("_None._")); // both handoff sub-sections
    assert!(md.contains("Anomaly tracking not enabled"));
    assert!(md.contains("Approval rate: **n/a**"));
}

#[tokio::test]
async fn build_report_on_empty_db_returns_zero_counts() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let now = Utc::now();
    let period_start = Period::Quarterly.start_for(now);
    let worker_id = Uuid::now_v7().to_string();
    let report = build_report(store.pool(), &worker_id, Period::Quarterly, period_start, now)
        .await
        .expect("build_report runs against an empty migrated db");
    assert_eq!(report.summary.session_count, 0);
    assert!(report.kpis.is_empty());
    assert_eq!(report.success.total, 0);
    assert!(report.feedback.is_empty());
    assert!(report.handoffs_sent.is_empty());
    assert!(report.handoffs_received.is_empty());
    // worker_anomaly_event ships in the migrated schema, so the
    // section IS populated (count = 0) when present.
    assert_eq!(report.anomalies.as_ref().map(|a| a.count), Some(0));
}

#[tokio::test]
async fn build_report_picks_up_kpi_observations_with_baseline() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let worker_id = Uuid::now_v7().to_string();
    // record_value timestamps with Utc::now(); period_start back-dated
    // 1ns means both readings land AFTER the cutoff → no baseline.
    // To get a baseline we record one, take Utc::now() between, then
    // record a second.
    worker_kpi::record_value(store.pool(), &worker_id, "tnt", "csat", 4.2, None)
        .await
        .unwrap();
    let mid = Utc::now();
    // tiny sleep to keep the second write strictly later in micros.
    std::thread::sleep(std::time::Duration::from_millis(2));
    worker_kpi::record_value(store.pool(), &worker_id, "tnt", "csat", 4.8, None)
        .await
        .unwrap();
    let report = build_report(
        store.pool(),
        &worker_id,
        Period::Weekly,
        mid,
        Utc::now(),
    )
    .await
    .expect("build_report runs");
    assert_eq!(report.kpis.len(), 1);
    let kpi = &report.kpis[0];
    assert_eq!(kpi.kpi, "csat");
    assert_eq!(kpi.baseline, Some(4.2));
    assert_eq!(kpi.latest, 4.8);
}
