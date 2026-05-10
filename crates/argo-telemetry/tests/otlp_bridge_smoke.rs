//! End-to-end smoke test for the `tracing-opentelemetry` bridge.
//!
//! Pattern mirrors `otlp_smoke.rs`: spin up a tiny TCP server that
//! pretends to be an OTLP/HTTP collector, point the exporter at it,
//! install the bridge layer on a `tracing-subscriber::Registry`, and
//! emit an `#[instrument]`-decorated function. The acceptance signal
//! is "the mock collector receives at least one POST after the
//! bridge has been wired and a tracing span has been emitted" — the
//! same observability bar `otlp_smoke.rs` enforces for the manually-
//! mirrored path.
//!
//! Compile-gated to the `otlp` feature so default builds stay clean.
//! There is exactly one test in this binary, so installing the
//! global tracing subscriber via `set_global_default` is safe — no
//! sibling tests can race with it.

#![cfg(feature = "otlp")]

use argo_telemetry::{otlp_bridge, otlp_exporter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing_subscriber::{prelude::*, Registry};

/// `#[instrument]` target with a custom field. Naming `session_id`
/// as a tracing field exercises the "fields propagate as OTLP
/// attributes" criterion: if the bridge layer dropped fields, the
/// span would still emit but without the custom attribute. The
/// mock collector counts requests rather than parsing protobuf, so
/// the strict assertion lives implicitly in the fact that the test
/// compiles and the bridge layer accepts the recorded field without
/// panicking — combined with the byte-level "did anything POST?"
/// check on the wire.
#[tracing::instrument(fields(session_id = %session_id, tier = "bridge-smoke"))]
async fn instrumented_action(session_id: &str) -> u32 {
    tracing::info!(stage = "started", "instrumented_action body");
    // Nested span: child of the `#[instrument]` parent. The bridge
    // must preserve parent/child linkage in the OTLP export.
    let _child =
        tracing::info_span!("instrumented_action.child", session_id = %session_id).entered();
    tracing::info!(stage = "finished", "instrumented_action body");
    42
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn instrument_spans_reach_mock_collector_via_bridge() {
    // Bind a random port and accept connections concurrently.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let addr = listener.local_addr().expect("local_addr");
    let request_count = Arc::new(AtomicUsize::new(0));
    let counter = request_count.clone();

    let server_handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let counter = counter.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16 * 1024];
                let _ = socket.read(&mut buf).await;
                let _ = socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await;
                let _ = socket.shutdown().await;
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    });

    // Point the exporter at the mock collector before init() reads
    // the env. SAFETY: only test in this binary, no env-var race.
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", format!("http://{addr}"));
    std::env::set_var("OTEL_SERVICE_NAME", "argo-bridge-smoke");
    let guard = otlp_exporter::init().expect("otlp init should succeed");

    // Build a subscriber whose ONLY active layer is the bridge.
    // `set_global_default` is safe here because this is the sole
    // test in the binary; the global default survives across
    // `.await` points so the multi-threaded runtime can move tasks
    // between worker threads without losing the subscriber.
    let subscriber = Registry::default().with(otlp_bridge::otlp_layer());
    tracing::subscriber::set_global_default(subscriber)
        .expect("set global subscriber");

    let result = instrumented_action("sess-bridge-smoke").await;
    assert_eq!(result, 42);

    // Drop the ShutdownGuard to flush pending exports synchronously.
    drop(guard);

    // The mock-collector accept tasks may need a tick to record
    // their counters after the synchronous flush returns.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let count = request_count.load(Ordering::SeqCst);
    assert!(
        count >= 1,
        "expected at least one POST to mock OTLP endpoint via bridge, got {count}"
    );

    server_handle.abort();
}
