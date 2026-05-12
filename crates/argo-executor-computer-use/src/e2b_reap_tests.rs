//! Unit tests for [`crate::e2b_reap`]. Pulled in via
//! `#[cfg(test)] #[path = "e2b_reap_tests.rs"] mod tests;` from
//! `e2b_reap.rs` to keep the production module under the 500-LOC
//! per-file invariant.
//!
//! Coverage: HTTP-level pagination + status parsing via wiremock;
//! trait-level reap semantics (threshold boundary, error-continue,
//! empty-no-op) via [`crate::testing::MockE2bClient`].

use super::*;
use crate::e2b::{E2bCall, E2bClient};
use crate::testing::MockE2bClient;
use chrono::{Duration, Utc};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture(id: &str, last_activity_at_secs_ago: i64) -> E2bSandboxInfo {
    let now = Utc::now();
    E2bSandboxInfo {
        id: id.to_owned(),
        template: "browser".to_owned(),
        started_at: now - Duration::seconds(last_activity_at_secs_ago + 60),
        last_activity_at: Some(now - Duration::seconds(last_activity_at_secs_ago)),
        status: "running".to_owned(),
    }
}

#[tokio::test]
async fn list_sandboxes_returns_two_rows_from_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sandboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sandboxes": [
                {"sandboxId": "a", "templateId": "browser",
                 "startedAt": "2026-05-12T00:00:00Z",
                 "lastActivityAt": "2026-05-12T00:01:00Z",
                 "status": "running"},
                {"sandboxId": "b", "templateId": "browser",
                 "startedAt": "2026-05-12T00:00:00Z",
                 "lastActivityAt": "2026-05-12T00:02:00Z",
                 "status": "running"}
            ],
            "nextCursor": null
        })))
        .mount(&server)
        .await;
    let client = E2bClient::new(server.uri(), "k".into(), 5).unwrap();
    let rows = client.list_sandboxes().await.expect("list ok");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "a");
    assert_eq!(rows[1].id, "b");
}

#[tokio::test]
async fn reap_idle_with_no_stale_returns_empty_report() {
    let mock = MockE2bClient::scripted(Vec::new(), Vec::new())
        .with_sandboxes(vec![fixture("fresh-1", 10), fixture("fresh-2", 20)]);
    let report = mock.reap_idle(600).await.expect("reap ok");
    assert_eq!(report.scanned, 2);
    assert!(report.reaped.is_empty());
    assert!(report.errors.is_empty());
}

#[tokio::test]
async fn reap_idle_with_three_stale_closes_all_three() {
    let mock = MockE2bClient::scripted(Vec::new(), Vec::new()).with_sandboxes(vec![
        fixture("stale-1", 1200),
        fixture("stale-2", 900),
        fixture("stale-3", 800),
    ]);
    let report = mock.reap_idle(600).await.expect("reap ok");
    assert_eq!(report.scanned, 3);
    assert_eq!(report.reaped.len(), 3);
    assert!(report.errors.is_empty());
}

#[tokio::test]
async fn reap_idle_with_one_close_error_continues_and_reports() {
    let mock = MockE2bClient::scripted(Vec::new(), Vec::new())
        .with_sandboxes(vec![
            fixture("stale-ok", 900),
            fixture("stale-err", 900),
            fixture("stale-ok-2", 900),
        ])
        .with_close_error("stale-err", "e2b returned 502");
    let report = mock.reap_idle(600).await.expect("reap ok");
    assert_eq!(report.scanned, 3);
    assert_eq!(report.reaped.len(), 2);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].0, "stale-err");
}

#[tokio::test]
async fn reap_idle_honors_max_idle_threshold_boundary() {
    // Exactly-at-threshold is NOT reaped (strict >).
    let mock = MockE2bClient::scripted(Vec::new(), Vec::new()).with_sandboxes(vec![
        fixture("at-thresh", 600),
        fixture("over-thresh", 601),
    ]);
    let report = mock.reap_idle(600).await.expect("reap ok");
    assert_eq!(report.reaped.len(), 1);
    assert_eq!(report.reaped[0], "over-thresh");
}

#[tokio::test]
async fn list_sandboxes_handles_pagination_over_100_rows() {
    let server = MockServer::start().await;
    let page_one: Vec<serde_json::Value> = (0..100)
        .map(|i| serde_json::json!({
            "sandboxId": format!("p1-{i}"), "templateId": "browser",
            "startedAt": "2026-05-12T00:00:00Z",
            "lastActivityAt": "2026-05-12T00:01:00Z",
            "status": "running"
        }))
        .collect();
    let page_two: Vec<serde_json::Value> = (0..50)
        .map(|i| serde_json::json!({
            "sandboxId": format!("p2-{i}"), "templateId": "browser",
            "startedAt": "2026-05-12T00:00:00Z",
            "lastActivityAt": "2026-05-12T00:01:00Z",
            "status": "running"
        }))
        .collect();
    Mock::given(method("GET"))
        .and(path("/sandboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sandboxes": page_one,
            "nextCursor": "p2"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sandboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sandboxes": page_two,
            "nextCursor": null
        })))
        .mount(&server)
        .await;
    let client = E2bClient::new(server.uri(), "k".into(), 5).unwrap();
    let rows = client.list_sandboxes().await.expect("list ok");
    assert_eq!(rows.len(), 150);
    assert_eq!(rows[0].id, "p1-0");
    assert_eq!(rows[100].id, "p2-0");
}
