//! Smoke test for the OTLP exporter wiring.
//!
//! Spins up a tiny TCP server that pretends to be an OTLP/HTTP
//! collector — accepts incoming connections, returns 200 OK, and
//! counts them. Verifies that telemetry emitted via
//! `TracingTelemetry` after `otlp_exporter::init()` actually
//! reaches the wire.
//!
//! Skipped at compile time when the `otlp` feature is off so
//! `cargo test -p argo-telemetry` (default features) stays clean.
#![cfg(feature = "otlp")]

use argo_telemetry::{metrics, otlp_exporter, spans, TelemetrySink, TracingTelemetry};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn otlp_exporter_reaches_mock_collector() {
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
                // Read whatever the client sends — we don't need to
                // parse it, just count the request.
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

    // Point the exporter at the mock collector. Set vars BEFORE
    // calling `init()` so they are read at provider construction.
    // SAFETY: this is the only test in this binary, so no env-var
    // race with sibling tests.
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", format!("http://{addr}"));
    std::env::set_var("OTEL_SERVICE_NAME", "argo-otlp-smoke");
    let guard = otlp_exporter::init().expect("otlp init should succeed");

    // Emit one of each kind of telemetry. The TracingTelemetry sink
    // routes counter + histogram to the OTLP meter (because
    // feature=otlp is on) and opens both a tracing span and an OTLP
    // span on the global tracer.
    let sink = TracingTelemetry;
    {
        let _outer = sink.span(spans::AGENT_TURN, &[("test", "smoke")]);
        let _inner = sink.span(spans::AGENT_PROVIDER_REQUEST, &[("nested", "true")]);
        sink.counter(
            metrics::ROUTING_DECISIONS_TOTAL,
            1,
            &[("tier", "smoke")],
        );
        sink.observe(
            metrics::JUDGE_DURATION_MS,
            12.5,
            &[("verdict", "smoke")],
        );
    }

    // Dropping the ShutdownGuard runs `shutdown()` on both the
    // tracer and meter providers, which flushes pending batches
    // synchronously.
    drop(guard);

    // The mock-collector accept tasks may need a tick to record
    // their counters after the synchronous flush returns.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let count = request_count.load(Ordering::SeqCst);
    assert!(
        count >= 1,
        "expected at least one POST to mock OTLP endpoint, got {count}"
    );

    server_handle.abort();
}
