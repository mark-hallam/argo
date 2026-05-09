//! Unit tests for `FilesystemExecutor` audit-event emission.
//!
//! Verifies the dispatch site emits one `tracing::info!` event under
//! target `argo::fs::audit` with the agreed structured fields
//! `{tenant, worker, op, path, decision, reason}`. The success path
//! requires a real fs_root + sandbox + tokio runtime, so these tests
//! focus on the early-return denial paths (missing `op`, unknown
//! `op`); the success-path emit shape is structurally identical
//! (`decision = "allowed"`, empty `reason`) and is exercised in
//! production by the FilesystemExecutor's Ok arm.

use super::*;
use argo_identity::WorkerId;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct Buf(Arc<Mutex<Vec<u8>>>);

impl Write for Buf {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Buf {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn rec(payload: serde_json::Value) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.into(),
        action_id: "a".into(),
        job_id: "j".into(),
        worker_id: WorkerId::nil(),
        item_id: None,
        action_type: "filesystem".into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: "ready".into(),
        payload,
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-09T00:00:00Z".into(),
    }
}

fn capture(rec: ActionRecord, cwd: &Path) -> (ArgoResult<ActionOutcome>, String) {
    let buf = Buf::default();
    let sub = tracing_subscriber::fmt()
        .with_writer(buf.clone())
        .with_target(true)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .finish();
    let result = tracing::subscriber::with_default(sub, || {
        FilesystemExecutor::new().dispatch(&rec, cwd)
    });
    let log = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    (result, log)
}

#[test]
fn dispatch_emits_denied_audit_on_missing_op() {
    let (result, log) = capture(rec(serde_json::json!({})), std::path::Path::new("/tmp"));
    assert!(result.is_err(), "expected missing-op error: {result:?}");
    assert!(log.contains("argo::fs::audit"), "no audit target: {log}");
    assert!(log.contains("denied"), "no denied decision: {log}");
    assert!(log.contains("(missing)"), "no missing-op marker: {log}");
    assert!(log.contains("default-tenant"), "no tenant marker: {log}");
    assert!(
        log.contains("00000000-0000-0000-0000-000000000000"),
        "no worker uuid: {log}"
    );
    assert!(log.contains("reason"), "no reason field on denial: {log}");
}

#[test]
fn dispatch_emits_denied_audit_on_unknown_op() {
    let (result, log) = capture(
        rec(serde_json::json!({"op": "delete", "path": "x.txt"})),
        std::path::Path::new("/tmp"),
    );
    assert!(result.is_err(), "expected unknown-op error: {result:?}");
    assert!(log.contains("argo::fs::audit"), "no audit target: {log}");
    assert!(log.contains("denied"), "no denied decision: {log}");
    assert!(log.contains("delete"), "no op marker: {log}");
    assert!(log.contains("x.txt"), "no path marker: {log}");
    assert!(log.contains("reason"), "no reason field on denial: {log}");
}
