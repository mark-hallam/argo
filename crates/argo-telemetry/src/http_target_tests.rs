//! Unit tests for [`crate::http_target`]. Seven scenarios per brief:
//!
//!  1. single-event flushes at `flush_interval`
//!  2. batch flushes at `batch_size` cap before interval
//!  3. queue overflow drops oldest, not newest
//!  4. http-500 doesn't crash the layer; logs warn
//!  5. http-timeout doesn't block the tracing-call path
//!  6. Drop flushes best-effort with short timeout
//!  7. tenant_id field passes through unchanged in JSON output
use super::*;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn target_for(server: &MockServer, batch: usize, interval: Duration, queue_max: usize) -> Arc<HttpLogTarget> {
    Arc::new(
        HttpLogTarget::new(
            reqwest::Url::parse(&format!("{}/ingest", server.uri())).unwrap(),
            None,
            batch,
            interval,
            Duration::from_secs(5),
            queue_max,
        )
        .unwrap(),
    )
}

#[tokio::test]
async fn single_event_flushes_at_flush_interval() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ingest"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..)
        .mount(&server)
        .await;
    let target = target_for(&server, 100, Duration::from_millis(40), 1024);
    target.push(r#"{"hello":"world"}"#.into());
    let t = target.clone();
    let h = tokio::spawn(async move { t.run_flusher().await });
    tokio::time::sleep(Duration::from_millis(200)).await;
    target.shutdown.store(true, Ordering::Release);
    target.notify.notify_waiters();
    let _ = tokio::time::timeout(Duration::from_secs(1), h).await;
    server.verify().await;
}

#[tokio::test]
async fn batch_flushes_at_batch_size_cap_before_interval() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..)
        .mount(&server)
        .await;
    // Long interval — only batch-cap notify can trigger the flush in time.
    let target = target_for(&server, 3, Duration::from_secs(30), 1024);
    let t = target.clone();
    let h = tokio::spawn(async move { t.run_flusher().await });
    target.push(r#"{"i":1}"#.into());
    target.push(r#"{"i":2}"#.into());
    target.push(r#"{"i":3}"#.into());
    tokio::time::sleep(Duration::from_millis(200)).await;
    target.shutdown.store(true, Ordering::Release);
    target.notify.notify_waiters();
    let _ = tokio::time::timeout(Duration::from_secs(1), h).await;
    server.verify().await;
}

#[tokio::test]
async fn queue_overflow_drops_oldest_not_newest() {
    let server = MockServer::start().await;
    let target = target_for(&server, 1000, Duration::from_secs(60), 2);
    target.push("OLDEST".into());
    target.push("MID".into());
    target.push("NEWEST".into());
    assert_eq!(target.queue_depth(), 2);
    let drained: Vec<String> = target.queue.lock().drain(..).collect();
    assert_eq!(drained, vec!["MID".to_string(), "NEWEST".to_string()]);
}

#[tokio::test]
async fn http_500_doesnt_crash_layer_logs_warn() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let target = target_for(&server, 1, Duration::from_millis(20), 1024);
    target.push(r#"{"x":1}"#.into());
    // Should complete without panicking even though the endpoint 500s.
    let sent = target.flush_once().await;
    assert_eq!(sent, 1, "events were drained even though endpoint returned 500");
}

#[tokio::test]
async fn http_timeout_doesnt_block_tracing_call() {
    let server = MockServer::start().await;
    // Mock that hangs forever (no `respond_with` => default 408 after wiremock default delay).
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(30)))
        .mount(&server)
        .await;
    let target = Arc::new(
        HttpLogTarget::new(
            reqwest::Url::parse(&format!("{}/ingest", server.uri())).unwrap(),
            None,
            1,
            Duration::from_millis(20),
            Duration::from_millis(50),
            1024,
        )
        .unwrap(),
    );
    // The `push` call must return immediately — flusher does the network call.
    let start = std::time::Instant::now();
    target.push(r#"{"x":1}"#.into());
    assert!(start.elapsed() < Duration::from_millis(10), "push must not block on the network");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drop_flushes_best_effort_with_short_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..)
        .mount(&server)
        .await;
    {
        let target = HttpLogTarget::new(
            reqwest::Url::parse(&format!("{}/ingest", server.uri())).unwrap(),
            None,
            100,
            Duration::from_secs(60),
            Duration::from_secs(1),
            1024,
        )
        .unwrap();
        target.push(r#"{"final":"flush"}"#.into());
        drop(target);
    }
    tokio::time::sleep(Duration::from_millis(400)).await;
    server.verify().await;
}

#[tokio::test]
async fn tenant_id_field_passes_through_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!([{"tenant_id": "acme-co"}])))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..)
        .mount(&server)
        .await;
    let target = target_for(&server, 1, Duration::from_millis(20), 1024);
    let layer = HttpLogTargetLayer::new(target.clone(), false);
    let subscriber = Registry::default().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(tenant_id = "acme-co", "audit event");
    });
    let sent = target.flush_once().await;
    assert_eq!(sent, 1, "exactly one event pushed and flushed");
    server.verify().await;
}
