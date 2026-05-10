//! Unit tests for [`super`] (`claude-dashboard-fleet-view-html`).
//!
//! Covers the five scenarios called out in the brief:
//!
//! 1. `renders_empty_fleet` — empty input produces a clean "no
//!    workers" page, no panics.
//! 2. `renders_workers_with_status` — a populated snapshot list
//!    produces a card per worker with the correct status badge class.
//! 3. `escapes_html_in_action_text` — worker ids and action kinds
//!    that contain HTML metacharacters are escaped.
//! 4. `200_status_and_html_content_type` — the handler returns an
//!    HTTP/1.1 200 status line with `text/html; charset=utf-8`.
//! 5. `recent_activity_ordered_newest_first` — the row grouper sorts
//!    each worker's recent actions newest-first regardless of input
//!    order.

use super::{
    classify_status, group_rows_into_snapshots, handle, is_fleet, render_fleet, RecentAction,
    WorkerSnapshot, WorkerStatus,
};

fn snapshot(worker_id: &str, status: WorkerStatus, kind: &str) -> WorkerSnapshot {
    WorkerSnapshot {
        worker_id: worker_id.into(),
        status,
        last_kind: kind.into(),
        last_at_micros: 1_700_000_000_000_000,
        recent: vec![RecentAction {
            action_id: "abc123def456".into(),
            kind: kind.into(),
            at_micros: 1_700_000_000_000_000,
        }],
    }
}

#[test]
fn renders_empty_fleet() {
    let html = render_fleet(&[]);
    assert!(html.contains("<h1>Fleet</h1>"));
    assert!(html.contains("No workers have reported activity yet."));
    assert!(html.contains("setTimeout"));
    // No card markup is emitted for the empty case (inline CSS still
    // mentions class names; only the rendered DOM containers are
    // absent).
    assert!(!html.contains("<div class=\"fleet-grid\">"));
    assert!(!html.contains("<section class=\"fleet-card\">"));
}

#[test]
fn renders_workers_with_status() {
    let workers = vec![
        snapshot("worker-running-01", WorkerStatus::Running, "ToolCall"),
        snapshot("worker-blocked-02", WorkerStatus::Blocked, "ApprovalPending"),
        snapshot("worker-error-03", WorkerStatus::Error, "ProviderError"),
        snapshot("worker-idle-04", WorkerStatus::Idle, ""),
    ];
    let html = render_fleet(&workers);
    assert!(html.contains("worker-running-01"));
    assert!(html.contains("fleet-badge-running"));
    assert!(html.contains("fleet-badge-blocked"));
    assert!(html.contains("fleet-badge-error"));
    assert!(html.contains("fleet-badge-idle"));
    // Auto-refresh stays inline-only — no external script src.
    assert!(!html.contains("<script src="));
}

#[test]
fn escapes_html_in_action_text() {
    let workers = vec![WorkerSnapshot {
        worker_id: "<script>alert('xss')</script>".into(),
        status: WorkerStatus::Running,
        last_kind: "<img onerror=x>".into(),
        last_at_micros: 1_700_000_000_000_000,
        recent: vec![RecentAction {
            action_id: "<a href=\"\">".into(),
            kind: "<b>boom</b>".into(),
            at_micros: 1_700_000_000_000_000,
        }],
    }];
    let html = render_fleet(&workers);
    assert!(!html.contains("<script>alert"));
    assert!(!html.contains("<img onerror=x>"));
    assert!(!html.contains("<b>boom</b>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&lt;img onerror=x&gt;"));
    assert!(html.contains("&lt;b&gt;boom&lt;/b&gt;"));
    // Single quotes inside the embedded XSS payload are also escaped.
    assert!(html.contains("&#39;"));
}

#[tokio::test]
async fn status_200_and_html_content_type() {
    let (status, body) = handle(None).await;
    assert!(status.starts_with("200 OK"), "status line: {status}");
    assert!(
        status.contains("Content-Type: text/html; charset=utf-8"),
        "content-type: {status}"
    );
    assert!(status.contains("Cache-Control: no-store"));
    assert!(body.contains("<h1>Fleet</h1>"));
}

#[test]
fn recent_activity_ordered_newest_first() {
    // Three rows for the same worker, fed in oldest-first order.
    let rows = vec![
        (
            "alpha".to_string(),
            "First".to_string(),
            vec![1u8, 2, 3],
            1_000_000,
        ),
        (
            "alpha".to_string(),
            "Second".to_string(),
            vec![4u8, 5, 6],
            2_000_000,
        ),
        (
            "alpha".to_string(),
            "Third".to_string(),
            vec![7u8, 8, 9],
            3_000_000,
        ),
    ];
    let snapshots = group_rows_into_snapshots(rows);
    assert_eq!(snapshots.len(), 1);
    let alpha = &snapshots[0];
    assert_eq!(alpha.worker_id, "alpha");
    assert_eq!(alpha.recent.len(), 3);
    assert_eq!(alpha.recent[0].kind, "Third");
    assert_eq!(alpha.recent[1].kind, "Second");
    assert_eq!(alpha.recent[2].kind, "First");
    assert_eq!(alpha.last_kind, "Third");
    assert_eq!(alpha.last_at_micros, 3_000_000);
}

#[test]
fn is_fleet_matches_expected_paths() {
    assert!(is_fleet("GET /dashboard/fleet HTTP/1.1\r\n\r\n"));
    assert!(is_fleet("GET /dashboard/fleet/ HTTP/1.1\r\n\r\n"));
    assert!(is_fleet("GET /dashboard/fleet?refresh=1 HTTP/1.1\r\n\r\n"));
    assert!(!is_fleet("GET /dashboard/workers HTTP/1.1\r\n\r\n"));
    assert!(!is_fleet("POST /dashboard/fleet HTTP/1.1\r\n\r\n"));
    assert!(!is_fleet("GET /dashboard/fleet/sub HTTP/1.1\r\n\r\n"));
}

#[test]
fn classify_status_buckets_correctly() {
    assert_eq!(classify_status("ToolCall"), WorkerStatus::Running);
    assert_eq!(classify_status("ProviderError"), WorkerStatus::Error);
    assert_eq!(classify_status("PolicyFail"), WorkerStatus::Error);
    assert_eq!(classify_status("ApprovalPending"), WorkerStatus::Blocked);
    assert_eq!(classify_status(""), WorkerStatus::Idle);
}
