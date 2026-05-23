//! Integration smoke test for the
//! `ARGO_AGENT_DEBUG_SYSTEM_MESSAGE=1` operator-visible debug
//! emission path — design doc §7 Pillar 1 acceptance #2.
//!
//! Sets the env var, installs a tracing subscriber that writes into
//! an in-memory buffer, and asserts a single line at the
//! `argo::system_message::debug` target appears with the workplace
//! block inline.

use argo_agent_runner::compose_system_prompt_for_diagnostics;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId};
use argo_workplace_knowledge::doc::{self, KnowledgeCategory};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

/// Shared in-memory sink for `tracing_subscriber` to write into. We
/// snapshot it after the turn-build to assert on the captured text.
#[derive(Clone, Default)]
struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl BufferWriter {
    fn snapshot(&self) -> String {
        let guard = self.0.lock().expect("buffer mutex poisoned");
        String::from_utf8_lossy(&guard).to_string()
    }
}

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("buffer mutex poisoned"))?;
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for BufferWriter {
    type Writer = BufferWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test]
async fn debug_env_emits_literal_system_message_with_workplace_block() {
    // Set env var BEFORE composing the prompt so the gate fires.
    // SAFETY: this test owns the env-var key; no other thread in this
    // tokio runtime reads it concurrently.
    // Safety: env_set required even in 2024 edition; the test binary
    // is the sole reader of this variable in-process.
    unsafe {
        std::env::set_var("ARGO_AGENT_DEBUG_SYSTEM_MESSAGE", "1");
    }

    let buffer = BufferWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buffer.clone())
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .finish();

    // Install the subscriber as the thread-local default for the rest
    // of this `#[tokio::test]`. `set_default` returns a guard that
    // uninstalls on drop. `#[tokio::test]` uses the current-thread
    // runtime by default, so the subscriber is active across awaits.
    let _guard = tracing::subscriber::set_default(subscriber);

    let store = Arc::new(
        SessionStore::open(":memory:")
            .await
            .expect("open in-memory store"),
    );
    store.migrate().await.expect("apply migrations");
    let tenant = TenantId::new();
    doc::create(
        store.pool(),
        &tenant,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "Never say cheap.",
        "ops@example.com",
    )
    .await
    .expect("seed workplace doc");

    compose_system_prompt_for_diagnostics(
        &store,
        SessionId(uuid::Uuid::nil()),
        None,
        tenant,
        None,
    )
    .await
    .expect("compose system message")
    .expect("expected non-empty system message");

    let captured = buffer.snapshot();

    // Reset the env var so it doesn't leak into other tests.
    // Safety: same justification as the `set_var` above.
    unsafe {
        std::env::remove_var("ARGO_AGENT_DEBUG_SYSTEM_MESSAGE");
    }

    assert!(
        captured.contains("argo::system_message::debug"),
        "expected the debug target in captured tracing output; got:\n{captured}"
    );
    assert!(
        captured.contains("Never say cheap."),
        "expected the workplace doc body in the emitted system message; got:\n{captured}"
    );
    assert!(
        captured.contains("[WORKPLACE KNOWLEDGE — read-only, do not contradict]"),
        "expected workplace-knowledge banner inline; got:\n{captured}"
    );
}
