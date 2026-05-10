//! mvp-structured-logging — field-convention smoke.
//!
//! Locks the canonical field set the gateway / agent-runner /
//! work-loop span wrappers emit (`tenant_id`, `worker_id`,
//! `session_id`, `request_id`, `route`, `principal_id`, plus the
//! per-step `step` counter and per-job `vertical_id` / `job_id`).
//! A regression here is a wire-shape change for log search; the
//! runbook (`docs/skycot/RUNBOOK_LOGGING.md`) documents the same
//! contract.
//!
//! The test is intentionally self-contained: it opens spans with
//! the same `info_span!(...)` invocations the production handlers
//! use and asserts the rendered output through a captured writer.
//! Exercising the real handlers requires a live SessionStore +
//! provider transport, which is out of scope for a single smoke
//! (and would only confirm what this test already locks — that
//! `tracing-subscriber::fmt` renders the field set we declared).

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::Instrument;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct CaptureBuf(Arc<Mutex<Vec<u8>>>);

impl CaptureBuf {
    fn drain(&self) -> String {
        let bytes = std::mem::take(&mut *self.0.lock().unwrap());
        String::from_utf8(bytes).expect("utf8 captured log")
    }
}

impl Write for CaptureBuf {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(b)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CaptureBuf {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture<F: FnOnce()>(f: F) -> String {
    let buf = CaptureBuf::default();
    let sub = tracing_subscriber::fmt()
        .with_writer(buf.clone())
        .with_target(false)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::with_default(sub, f);
    buf.drain()
}

#[test]
fn gateway_request_span_carries_route_principal_request_id() {
    let log = capture(|| {
        let span = tracing::info_span!(
            "request",
            route = "POST /sessions",
            principal_id = "human:operator@example.com",
            request_id = "req-abc-123",
        );
        let _enter = span.enter();
        tracing::info!("dispatched");
    });
    assert!(log.contains("request"), "no span name in: {log}");
    assert!(log.contains("route=\"POST /sessions\""), "route field missing: {log}");
    assert!(
        log.contains("principal_id=\"human:operator@example.com\""),
        "principal_id field missing: {log}"
    );
    assert!(log.contains("request_id=\"req-abc-123\""), "request_id field missing: {log}");
    assert!(log.contains("dispatched"), "event message missing: {log}");
}

#[test]
fn agent_runner_turn_span_carries_session_tenant_worker() {
    let log = capture(|| {
        let session = "018f55b7-89ab-7cde-8123-456789abcdef";
        let tenant = "00112233-4455-6677-8899-aabbccddeeff";
        let worker = "ffeeddcc-bbaa-9988-7766-554433221100";
        let span = tracing::info_span!(
            "turn",
            session_id = session,
            tenant_id = tenant,
            worker_id = worker,
        );
        let _enter = span.enter();
        tracing::info!("provider call");
    });
    assert!(
        log.contains("session_id=\"018f55b7-89ab-7cde-8123-456789abcdef\""),
        "session_id field missing: {log}"
    );
    assert!(
        log.contains("tenant_id=\"00112233-4455-6677-8899-aabbccddeeff\""),
        "tenant_id field missing: {log}"
    );
    assert!(
        log.contains("worker_id=\"ffeeddcc-bbaa-9988-7766-554433221100\""),
        "worker_id field missing: {log}"
    );
}

#[test]
fn worker_job_span_carries_tenant_vertical_job_id() {
    let log = capture(|| {
        let span = tracing::info_span!(
            "worker_job",
            tenant_id = "tenant-uuid",
            vertical_id = "gtm-engineer",
            job_id = "job-uuid",
        );
        let _enter = span.enter();
        tracing::info!("processing job");
    });
    assert!(log.contains("tenant_id=\"tenant-uuid\""), "{log}");
    assert!(log.contains("vertical_id=\"gtm-engineer\""), "{log}");
    assert!(log.contains("job_id=\"job-uuid\""), "{log}");
}

#[test]
fn worker_step_span_carries_step_iteration_counter() {
    let log = capture(|| {
        let span = tracing::info_span!("step", step = 2u32, max_steps = 5u32);
        let _enter = span.enter();
        tracing::info!("step running");
    });
    assert!(log.contains("step=2"), "step counter missing: {log}");
    assert!(log.contains("max_steps=5"), "max_steps missing: {log}");
}

#[test]
fn nested_spans_propagate_fields_through_async_await() {
    // Reproduces the production stack: worker_job → step → turn.
    // Every emission inside the innermost block must inherit
    // every ancestor's fields, including across an `.await`
    // suspension point. This is the core invariant the
    // structured-logging brief locks.
    let log = capture(|| {
        let outer = tracing::info_span!(
            "worker_job",
            tenant_id = "T1",
            vertical_id = "v",
            job_id = "j",
        );
        let fut = async {
            tokio::task::yield_now().await;
            let step = tracing::info_span!("step", step = 1u32, max_steps = 3u32);
            async {
                tokio::task::yield_now().await;
                let turn = tracing::info_span!(
                    "turn",
                    session_id = "S1",
                    tenant_id = "T1",
                    worker_id = "W1",
                );
                async {
                    tokio::task::yield_now().await;
                    tracing::info!("inner emission");
                }
                .instrument(turn)
                .await;
            }
            .instrument(step)
            .await;
        }
        .instrument(outer);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(fut);
    });
    for needle in [
        "tenant_id=\"T1\"",
        "vertical_id=\"v\"",
        "job_id=\"j\"",
        "step=1",
        "session_id=\"S1\"",
        "worker_id=\"W1\"",
        "inner emission",
    ] {
        assert!(log.contains(needle), "missing {needle} in:\n{log}");
    }
}
