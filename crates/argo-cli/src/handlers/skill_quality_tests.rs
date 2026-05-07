//! Unit tests for `super::skill_quality` rendering helpers.
//!
//! These cover the pure render functions only. The integration
//! smoke (`tests/skill_quality_view.rs`) drives the full path
//! including the store + CLI dispatch.
use super::*;
use argo_skill::{QualityInputs, QualityScore};
use argo_types::SkillId;
use chrono::TimeZone;

fn fixture(score: u8, success_rate: f64, ts_secs: i64) -> QualityScore {
    QualityScore {
        skill_id: SkillId::new(),
        score,
        inputs: QualityInputs {
            success_rate,
            avg_latency_ms: 12.5,
            retry_count: 0,
            p99_latency_ms: 30.0,
        },
        computed_at: chrono::Utc.timestamp_opt(ts_secs, 0).unwrap(),
    }
}

#[test]
fn render_markdown_empty_history_pins_placeholder() {
    let out = render_markdown("alpha", &[], 1);
    assert!(out.contains("# Skill quality — `alpha`"));
    assert!(out.contains("(no quality score recorded yet)"));
    assert!(!out.contains("## Latest"));
}

#[test]
fn render_markdown_latest_only_drops_history_section() {
    let scores = vec![fixture(82, 0.95, 1_700_000_000)];
    let out = render_markdown("alpha", &scores, 1);
    assert!(out.contains("## Latest"));
    assert!(out.contains("**score**: `82`"));
    assert!(out.contains("success_rate: `0.950`"));
    assert!(
        !out.contains("## History"),
        "limit=1 must not render history block, got: {out}"
    );
}

#[test]
fn render_markdown_with_history_renders_table_rows() {
    let scores = vec![
        fixture(75, 0.90, 1_700_000_300),
        fixture(80, 0.92, 1_700_000_200),
        fixture(85, 0.95, 1_700_000_100),
    ];
    let out = render_markdown("beta", &scores, 3);
    assert!(out.contains("**score**: `75`"));
    assert!(out.contains("## History (2 prior scores)"));
    assert!(out.contains("| computed_at | score |"));
    // Latest row is the markdown body; history rows are score=80 + score=85
    assert!(out.contains("| 80 |"));
    assert!(out.contains("| 85 |"));
}

#[test]
fn render_markdown_singularises_one_history_row() {
    let scores = vec![
        fixture(75, 0.90, 1_700_000_200),
        fixture(80, 0.92, 1_700_000_100),
    ];
    let out = render_markdown("beta", &scores, 2);
    assert!(out.contains("## History (1 prior score)"));
    assert!(!out.contains("1 prior scores"));
}
