//! Snapshot + helper tests for the OKR handler. End-to-end record→
//! show→progress is covered by `tests/worker_okr_smoke.rs`; these
//! pin the markdown shape and JSON helpers without spawning a binary.

use super::*;
use argo_store::worker_okr::{
    Comparison, KeyResultProgress, KeyResultRow, KeyResultSpec, ObjectiveRow, ObjectiveStatus,
    ProgressReport,
};

fn objective(id: i64, period: &str, title: &str) -> ObjectiveRow {
    ObjectiveRow {
        id,
        worker_id: "11111111-1111-1111-1111-111111111111".into(),
        tenant_id: "22222222-2222-2222-2222-222222222222".into(),
        period: period.into(),
        title: title.into(),
        description: Some("Lift csat above 4.5".into()),
        status: ObjectiveStatus::Active,
        created_at: 1_700_000_000_000_000,
    }
}

fn kr(id: i64, kpi: &str, op: Comparison, target: f64, weight: f64) -> KeyResultRow {
    KeyResultRow {
        id,
        objective_id: 1,
        kpi_name: kpi.into(),
        target_value: target,
        comparison: op,
        weight,
    }
}

#[test]
fn render_markdown_emits_objective_and_kr_table() {
    let bundles = vec![(
        objective(1, "2026Q2", "Lift CSAT"),
        vec![
            kr(1, "csat", Comparison::Gte, 4.5, 2.0),
            kr(2, "reply_sla_hours", Comparison::Lte, 12.0, 1.0),
        ],
    )];
    let md = render_markdown("worker-1", Some("2026Q2"), &bundles);
    assert!(md.contains("# Worker worker-1 OKRs (period 2026Q2)"));
    assert!(md.contains("## 2026Q2 — Lift CSAT (active)"));
    assert!(md.contains("Lift csat above 4.5"));
    assert!(md.contains("| KPI | op | target | weight |"));
    assert!(md.contains("| csat | gte | 4.5000 | 2.0000 |"));
    assert!(md.contains("| reply_sla_hours | lte | 12.0000 | 1.0000 |"));
}

#[test]
fn render_markdown_handles_empty() {
    let md = render_markdown("w", None, &[]);
    assert!(md.contains("# Worker w OKRs (all periods)"));
    assert!(md.contains("No objectives recorded"));
}

#[test]
fn report_json_includes_weighted_score_and_achieved_flags() {
    let report = ProgressReport {
        objective: objective(7, "2026Q2", "Lift CSAT"),
        key_results: vec![
            KeyResultProgress {
                key_result: kr(1, "csat", Comparison::Gte, 4.5, 2.0),
                actual_value: Some(4.7),
                achievement: 1.0,
                achieved: true,
            },
            KeyResultProgress {
                key_result: kr(2, "reply_sla_hours", Comparison::Lte, 12.0, 1.0),
                actual_value: Some(24.0),
                achievement: 0.5,
                achieved: false,
            },
        ],
        weighted_score: 2.5 / 3.0,
    };
    let body = report_json(&report);
    assert_eq!(body["objective_id"], 7);
    assert_eq!(body["period"], "2026Q2");
    let krs = body["key_results"].as_array().unwrap();
    assert_eq!(krs.len(), 2);
    assert_eq!(krs[0]["achieved"], true);
    assert_eq!(krs[1]["achieved"], false);
    let score = body["weighted_score"].as_f64().unwrap();
    assert!((score - (2.5 / 3.0)).abs() < 1e-9);
}

#[test]
fn kr_spec_json_round_trips_target_and_weight() {
    let spec = KeyResultSpec {
        kpi_name: "csat".into(),
        comparison: Comparison::Gte,
        target_value: 4.5,
        weight: 2.0,
    };
    let body = kr_spec_json(&spec);
    assert_eq!(body["kpi"], "csat");
    assert_eq!(body["op"], "gte");
    assert!((body["target"].as_f64().unwrap() - 4.5).abs() < f64::EPSILON);
    assert!((body["weight"].as_f64().unwrap() - 2.0).abs() < f64::EPSILON);
}

#[test]
fn progress_kr_json_handles_missing_actual() {
    let kr_progress = KeyResultProgress {
        key_result: kr(1, "csat", Comparison::Gte, 4.5, 1.0),
        actual_value: None,
        achievement: 0.0,
        achieved: false,
    };
    let body = progress_kr_json(&kr_progress);
    assert!(body["actual"].is_null());
    assert_eq!(body["achieved"], false);
    assert!((body["achievement"].as_f64().unwrap() - 0.0).abs() < f64::EPSILON);
}
