//! Integration smoke for [`argo_telemetry::http_target`].
//!
//! End-to-end exercise: install the [`HttpLogTargetLayer`] on a
//! `tracing-subscriber::Registry`, emit 5 tracing events under the
//! subscriber, drive a manual flush, assert the wiremock collector
//! saw exactly one POST whose body parses to a 5-element JSON array.
//!
//! Mirrors the otlp_smoke / otlp_bridge_smoke pattern of holding the
//! subscriber per-test (`with_default`, not `set_global_default`) so
//! the binary can grow sibling tests without subscriber races.

use argo_telemetry::http_target::{HttpLogTarget, HttpLogTargetLayer};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn five_events_reach_wiremock_in_one_post() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ingest"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..)
        .mount(&server)
        .await;

    let target = Arc::new(
        HttpLogTarget::new(
            reqwest::Url::parse(&format!("{}/ingest", server.uri())).unwrap(),
            None,
            // batch_size large enough to hold all 5 events in one POST
            16,
            Duration::from_millis(50),
            Duration::from_secs(2),
            1024,
        )
        .unwrap(),
    );

    // wrap=false to drive flushes deterministically from the test;
    // the production path uses install() which spawns a flusher.
    let layer = HttpLogTargetLayer::new(target.clone(), false);
    let subscriber = Registry::default().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        for i in 0..5_u32 {
            tracing::info!(
                tenant_id = "acme-co",
                worker_id = "w-7",
                session_id = "s-42",
                event_idx = i,
                "smoke event"
            );
        }
    });

    // Manually drive the flush so the test does not race the
    // background flush_interval. The Layer recorded 5 events;
    // batch_size=16 means a single drain ships all 5.
    let sent = target.flush_once().await;
    assert_eq!(sent, 5, "expected 5 events drained in a single batch");

    // Settle wiremock's request recorder before asserting.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let recv = server.received_requests().await.unwrap_or_default();
    assert_eq!(recv.len(), 1, "expected exactly one POST, got {}", recv.len());
    let body: serde_json::Value =
        serde_json::from_slice(&recv[0].body).expect("body parses as JSON");
    let array = body.as_array().expect("body is a JSON array");
    assert_eq!(array.len(), 5, "expected 5 events in the POSTed array");
    let last = &array[4];
    assert_eq!(last["tenant_id"], "acme-co");
    assert_eq!(last["worker_id"], "w-7");
    assert_eq!(last["session_id"], "s-42");
    assert_eq!(last["event_idx"], 4);
}
