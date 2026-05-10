//! Tests for [`crate::handoff_stats_query`] — split out under the
//! 500-LOC per-file ceiling.

use super::handoff_stats_query::{
    breakdown_by, classify, destination_label, overview, top_n_from, top_n_to, volume_per_bucket,
    BreakdownAxis, HandoffWindowRow, Outcome,
};

fn row(
    session: &str,
    from: &str,
    to_json: &str,
    outcome: &str,
    status: &str,
    created: i64,
    resolved: Option<i64>,
) -> HandoffWindowRow {
    HandoffWindowRow {
        handoff_id: format!("h-{session}-{created}"),
        session_id: session.into(),
        from_worker_id: from.into(),
        to_json: to_json.into(),
        expected_outcome: outcome.into(),
        status: status.into(),
        created_at: created,
        resolved_at: resolved,
    }
}

fn worker_to(id: &str) -> String {
    format!(r#"{{"kind":"worker","worker_id":"{id}"}}"#)
}

fn sample_window() -> Vec<HandoffWindowRow> {
    // Times are micros. Spread across 5 seconds so the
    // histogram test partitions by 1-second buckets cleanly.
    vec![
        // s-1: chain advances — ack then settled
        row("s-1", "w-a", &worker_to("w-b"), "ship", "acknowledged", 1_000_000, None),
        row("s-1", "w-b", &worker_to("w-c"), "ship", "settled", 2_000_000, Some(2_000_600)),
        // s-2: settled standalone
        row("s-2", "w-a", &worker_to("w-b"), "review", "settled", 3_000_000, Some(3_000_600)),
        // s-3: still in flight
        row("s-3", "w-c", &worker_to("w-a"), "ship", "pending", 4_000_000, None),
        // s-4: cancelled to a human
        row(
            "s-4",
            "w-a",
            r#"{"kind":"human","addr":"ops@x"}"#,
            "ship",
            "cancelled",
            5_000_000,
            Some(5_000_200),
        ),
    ]
}

#[test]
fn destination_label_handles_workers_humans_and_garbage() {
    assert_eq!(destination_label(&worker_to("w-1")), "w-1");
    assert_eq!(
        destination_label(r#"{"kind":"human","addr":"ops@x"}"#),
        "human:ops@x"
    );
    assert_eq!(destination_label("not-json"), "(unknown)");
    assert_eq!(destination_label(r#"{"kind":"unknown"}"#), "(unknown)");
}

#[test]
fn overview_buckets_outcomes_and_computes_rates() {
    let rows = sample_window();
    let o = overview(&rows);
    assert_eq!(o.total, 5);
    assert_eq!(o.resolved, 2);
    assert_eq!(o.abandoned, 1);
    assert_eq!(o.escalated, 1);
    assert_eq!(o.pending, 1);
    // success_rate = resolved / (resolved + abandoned) = 2/3
    assert!((o.success_rate.unwrap() - 2.0 / 3.0).abs() < 1e-9);
    // escalation_rate = sessions_with_>1_handoff / sessions = 1/4
    let er = o.escalation_rate.unwrap();
    assert!((er - 0.25).abs() < 1e-9, "got {er}");
    // Median over the two settled rows (latency 600 each).
    assert_eq!(o.median_latency_micros, Some(600));
}

#[test]
fn top_n_from_groups_by_origin_and_truncates() {
    let rows = sample_window();
    let out = top_n_from(&rows, 2);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].key, "w-a");
    assert_eq!(out[0].count, 3);
    assert_eq!(out[1].key, "w-b");
    assert_eq!(out[1].count, 1);
}

#[test]
fn top_n_to_uses_destination_label_projection() {
    let rows = sample_window();
    let out = top_n_to(&rows, 5);
    let keys: Vec<&str> = out.iter().map(|r| r.key.as_str()).collect();
    assert!(keys.contains(&"w-b"));
    assert!(keys.contains(&"human:ops@x"));
    let wb = out.iter().find(|r| r.key == "w-b").unwrap();
    assert_eq!(wb.count, 2);
}

#[test]
fn volume_per_bucket_partitions_window_into_fixed_buckets() {
    // 6-second window, 1-second buckets: rows are at 1s..5s
    // (micros 1_000_000..5_000_000), so buckets 1..5 each carry
    // exactly one row; bucket 0 is empty.
    let rows = sample_window();
    let out = volume_per_bucket(&rows, 0, 6_000_000, 1);
    assert_eq!(out.len(), 6);
    assert_eq!(out[0].count, 0);
    assert_eq!(out[1].count, 1);
    assert_eq!(out[2].count, 1);
    assert_eq!(out[3].count, 1);
    assert_eq!(out[4].count, 1);
    assert_eq!(out[5].count, 1);
}

#[test]
fn breakdown_by_worker_carries_per_key_outcome_split() {
    let rows = sample_window();
    let out = breakdown_by(&rows, BreakdownAxis::Worker);
    // w-a originated 3 rows: s-1@1s (escalated ack), s-2@3s
    // (resolved settled), s-4@5s (abandoned cancelled).
    let wa = out.iter().find(|r| r.key == "w-a").unwrap();
    assert_eq!(wa.count, 3);
    assert_eq!(wa.resolved, 1);
    assert_eq!(wa.escalated, 1);
    assert_eq!(wa.abandoned, 1);
    assert_eq!(wa.pending, 0);
    // success_rate = 1 / (1 + 1) = 0.5
    assert!((wa.success_rate.unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn classify_marks_escalated_only_when_session_has_later_row() {
    let r = row("s-1", "w", &worker_to("w"), "x", "acknowledged", 1, None);
    assert_eq!(classify(&r, false), Outcome::Pending);
    assert_eq!(classify(&r, true), Outcome::Escalated);
}
