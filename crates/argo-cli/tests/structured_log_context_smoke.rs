//! Integration smoke for Stream
//! `mission-structured-logging-tenant-worker-session`. Verifies the
//! [`ContextInjectorLayer`] from `argo-telemetry` produces structured
//! log lines carrying `tenant_id` / `worker_id` / `session_id` when
//! installed in a `tracing-subscriber` registry alongside emission
//! from the three brief-installed call-sites:
//!
//! 1. `crates/argo-cli/src/handlers/gateway/boot.rs` — boot-time
//!    anon context (`tenant_id=anon worker_id=- session_id=-`)
//! 2. `crates/argo-cli/src/handlers/worker_actions.rs::run` —
//!    worker-iteration context (`worker_id=…`)
//! 3. `crates/argo-orchestrator/src/session.rs::Session::new` /
//!    `Session::enter_log_context` — session context
//!    (`tenant_id=… session_id=…`)
//!
//! The smoke does not spin up a gateway HTTP listener (out of scope
//! for a smoke; the per-route bearer→tenant_id wiring lands in a
//! follow-up brief). Instead it exercises the same primitives the
//! brief installs at those call-sites against an in-process subscriber
//! and asserts the three fields appear on every emitted line.

use argo_telemetry::span_context::{enter_context, ContextInjectorLayer, LogContext};
use argo_orchestrator::Session;
use argo_types::TenantId;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::subscriber::with_default;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[derive(Clone, Default)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("buf").extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn install(buf: BufWriter, anon: &str) -> impl tracing::Subscriber + Send + Sync {
    Registry::default().with(ContextInjectorLayer::with_writer(anon, Box::new(buf)))
}

fn captured(buf: &BufWriter) -> String {
    String::from_utf8(buf.0.lock().expect("buf").clone()).unwrap()
}

#[test]
fn gateway_boot_anon_context_renders_anon_marker() {
    // Mirrors the boot-time context installed at the top of
    // `gateway::boot::start` — no tenant resolved yet, so all three
    // fields default. Anon marker is the operator-configured string;
    // worker/session render as `-`.
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _boot_ctx = enter_context(LogContext::default());
        tracing::info!("argo gateway: listening on 127.0.0.1:8787");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=anon"), "out={out}");
    assert!(out.contains("worker_id=-"), "out={out}");
    assert!(out.contains("session_id=-"), "out={out}");
    assert!(out.contains("argo gateway"), "out={out}");
}

#[test]
fn worker_iteration_context_carries_worker_id_on_every_line() {
    // Mirrors `handlers::worker_actions::run` — every CLI worker
    // subcommand enters a context with its `worker_id` (when present).
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _worker_ctx = enter_context(LogContext {
            worker_id: Some("wkr-emit".into()),
            ..LogContext::default()
        });
        // Multiple lines under the same iteration — every one must
        // carry worker_id.
        tracing::info!("planning iteration");
        tracing::warn!("retrying provider call");
        tracing::error!("iteration finished with errors");
    });
    let out = captured(&buf);
    let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, out={out}");
    for line in &lines {
        assert!(
            line.contains("worker_id=wkr-emit"),
            "missing worker_id on line: {line}"
        );
        assert!(
            line.contains("tenant_id=anon"),
            "missing tenant_id=anon on line: {line}"
        );
    }
}

#[test]
fn session_context_carries_tenant_and_session_via_session_enter() {
    // Mirrors `Session::enter_log_context` — every log line emitted
    // under the guard carries the session's tenant_id and session_id.
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    let tenant = TenantId::new();
    let tenant_str = tenant.to_string();
    let session = Session::new(tenant, "smoke-session");
    let session_id_str = session.id.as_str().to_string();
    with_default(sub, || {
        let _session_ctx = session.enter_log_context();
        tracing::info!("under session");
    });
    let out = captured(&buf);
    assert!(
        out.contains(&format!("tenant_id={tenant_str}")),
        "out={out}"
    );
    assert!(
        out.contains(&format!("session_id={session_id_str}")),
        "out={out}"
    );
    // worker_id not part of session context — anon-marker for worker
    // would be `-`, since the session ancestor doesn't set it.
    assert!(out.contains("worker_id=-"), "out={out}");
}

#[test]
fn nested_worker_under_session_inherits_session_and_layers_worker() {
    // Real-world composition: worker joins a session, emits work.
    // Outer span: session context (tenant + session). Inner span:
    // worker context (worker_id). Events under inner inherit all
    // three.
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    let tenant = TenantId::new();
    let tenant_str = tenant.to_string();
    let session = Session::new(tenant, "compose");
    let session_id_str = session.id.as_str().to_string();
    with_default(sub, || {
        let _session_ctx = session.enter_log_context();
        let _worker_ctx = enter_context(LogContext {
            worker_id: Some("wkr-joins".into()),
            ..LogContext::default()
        });
        tracing::info!("worker emits under session");
    });
    let out = captured(&buf);
    assert!(
        out.contains(&format!("tenant_id={tenant_str}")),
        "tenant from session: out={out}"
    );
    assert!(
        out.contains(&format!("session_id={session_id_str}")),
        "session from session: out={out}"
    );
    assert!(
        out.contains("worker_id=wkr-joins"),
        "worker from inner: out={out}"
    );
}
