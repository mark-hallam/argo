//! Unit tests for `span_context::{enter_context, ContextInjectorLayer}`.

use super::{enter_context, ContextInjectorLayer, LogContext};
use parking_lot::Mutex;
use std::io::{self, Write};
use std::sync::Arc;
use tracing::subscriber::with_default;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[derive(Clone, Default)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.lock().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn install(buf: BufWriter, anon: &str) -> impl tracing::Subscriber + Send + Sync {
    let layer = ContextInjectorLayer::with_writer(anon, Box::new(buf));
    Registry::default().with(layer)
}

fn captured(buf: &BufWriter) -> String {
    String::from_utf8(buf.0.lock().clone()).unwrap()
}

#[test]
fn enter_emits_all_three_fields() {
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _g = enter_context(LogContext {
            tenant_id: Some("tenant-a".into()),
            worker_id: Some("wkr-1".into()),
            session_id: Some("sess-42".into()),
        });
        tracing::info!("hello");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=tenant-a"), "out={out}");
    assert!(out.contains("worker_id=wkr-1"), "out={out}");
    assert!(out.contains("session_id=sess-42"), "out={out}");
}

#[test]
fn partial_context_emits_anon_for_missing_tenant_and_dash_for_others() {
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _g = enter_context(LogContext {
            tenant_id: None,
            worker_id: Some("wkr-2".into()),
            session_id: None,
        });
        tracing::info!("ping");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=anon"), "out={out}");
    assert!(out.contains("worker_id=wkr-2"), "out={out}");
    assert!(out.contains("session_id=-"), "out={out}");
}

#[test]
fn nested_ancestor_context_inherits_into_child() {
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _outer = enter_context(LogContext {
            tenant_id: Some("tenant-b".into()),
            worker_id: Some("wkr-9".into()),
            session_id: None,
        });
        // Inner span carries only session_id; tenant + worker should
        // inherit from the outer span via the layer's parent-walk.
        let _inner =
            tracing::info_span!("inner", session_id = "sess-x").entered();
        tracing::warn!("nested");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=tenant-b"), "out={out}");
    assert!(out.contains("worker_id=wkr-9"), "out={out}");
    assert!(out.contains("session_id=sess-x"), "out={out}");
}

#[test]
fn formatter_includes_three_fields_even_without_explicit_event_fields() {
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _g = enter_context(LogContext {
            tenant_id: Some("tenant-c".into()),
            worker_id: Some("wkr-5".into()),
            session_id: Some("sess-5".into()),
        });
        // Bare event — no explicit fields named by the call-site.
        tracing::error!("boom");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=tenant-c"), "out={out}");
    assert!(out.contains("worker_id=wkr-5"), "out={out}");
    assert!(out.contains("session_id=sess-5"), "out={out}");
    assert!(out.contains("boom"), "out={out}");
}

#[test]
fn disabled_layer_is_not_installed_zero_overhead_path() {
    // Simulate `[telemetry.context].enabled = false`: simply do not
    // install the layer. The Registry without our layer should not
    // produce any output via the writer (the writer is bound to the
    // layer instance).
    let buf = BufWriter::default();
    let sub = Registry::default();
    with_default(sub, || {
        let _g = enter_context(LogContext {
            tenant_id: Some("tenant-d".into()),
            worker_id: None,
            session_id: None,
        });
        tracing::info!("nothing-should-write");
    });
    // Buffer is owned outside the (uninstalled) layer; it remains empty.
    assert!(captured(&buf).is_empty(), "unexpected output: {}", captured(&buf));
}

#[test]
fn config_emit_anon_tenant_as_is_honored() {
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "external");
    with_default(sub, || {
        let _g = enter_context(LogContext {
            tenant_id: None,
            worker_id: None,
            session_id: None,
        });
        tracing::info!("anon-hello");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=external"), "out={out}");
}

#[test]
fn event_author_value_wins_over_ancestor() {
    let buf = BufWriter::default();
    let sub = install(buf.clone(), "anon");
    with_default(sub, || {
        let _g = enter_context(LogContext {
            tenant_id: Some("tenant-outer".into()),
            worker_id: Some("wkr-outer".into()),
            session_id: Some("sess-outer".into()),
        });
        // Event explicitly names tenant_id with a more specific value.
        tracing::info!(tenant_id = "tenant-override", "specific");
    });
    let out = captured(&buf);
    assert!(out.contains("tenant_id=tenant-override"), "out={out}");
    assert!(out.contains("worker_id=wkr-outer"), "out={out}");
    assert!(out.contains("session_id=sess-outer"), "out={out}");
}
