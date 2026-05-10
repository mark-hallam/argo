//! Unit tests for the wired `terminal` tool. Split out of
//! `terminal.rs` to keep that file under the 500-LOC cap as the
//! sandbox-backend wiring grows tests for both routing and capability
//! differentiation.

use super::*;
use argo_sandbox::RunRequest;
use argo_types::Principal;
use std::sync::Mutex;
use tokio::sync::mpsc;

fn ctx(dir: &std::path::Path) -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        dir.to_path_buf(),
    )
}

/// Recording sandbox so dispatch-routing tests can confirm which
/// backend was selected without relying on tracing infrastructure.
#[derive(Debug)]
struct RecordingSandbox {
    id: &'static str,
    calls: Mutex<Vec<String>>,
}

impl RecordingSandbox {
    fn new(id: &'static str) -> Self {
        Self {
            id,
            calls: Mutex::new(Vec::new()),
        }
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Sandbox for RecordingSandbox {
    fn id(&self) -> &str {
        self.id
    }
    async fn run(&self, req: &RunRequest) -> ArgoResult<RunResult> {
        self.calls.lock().unwrap().push(req.command.clone());
        Ok(RunResult {
            stdout: format!("from {}", self.id),
            stderr: String::new(),
            exit_code: 0,
            timed_out: false,
            duration: Duration::from_millis(1),
        })
    }
}

fn body(r: &ToolCallResult) -> String {
    match &r.content[0] {
        argo_types::ContentBlock::Text { text } => text.clone(),
        _ => panic!("text"),
    }
}

// The first three tests shell out to Unix-shell built-ins
// (`echo`, `exit`, `sleep`) and don't translate to cmd.exe;
// gated to Unix runners. Schema/dispatcher checks below are
// platform-agnostic.
#[cfg(unix)]
#[tokio::test]
async fn echoes_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let r = Terminal::default()
        .dispatch(serde_json::json!({"command": "echo hi"}), &ctx(dir.path()))
        .await
        .unwrap();
    assert!(!r.is_error);
    let body = body(&r);
    assert!(body.contains("exit_code: 0"));
    assert!(body.contains("hi"));
}

#[cfg(unix)]
#[tokio::test]
async fn captures_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let r = Terminal::default()
        .dispatch(serde_json::json!({"command": "exit 7"}), &ctx(dir.path()))
        .await
        .unwrap();
    assert!(!r.is_error); // exec succeeded; exit code is content not failure.
    assert!(body(&r).contains("exit_code: 7"));
}

#[cfg(unix)]
#[tokio::test]
async fn timeout_reports_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let r = Terminal::default()
        .dispatch(
            serde_json::json!({"command": "sleep 5", "timeout_ms": 100}),
            &ctx(dir.path()),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let body = body(&r);
    assert!(body.contains("timeout"));
    assert!(body.contains("100 ms"));
}

#[test]
fn required_capability_defaults_to_subprocess_spawn() {
    let caps = Terminal::default()
        .required_capabilities(&serde_json::json!({"command": "x"}))
        .unwrap();
    assert!(caps.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Subprocess,
    }));
    assert!(!caps.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Container,
    }));
}

#[test]
fn required_capability_for_sandbox_backend_is_container() {
    let caps = Terminal::default()
        .required_capabilities(&serde_json::json!({
            "command": "x",
            "backend": "sandbox",
        }))
        .unwrap();
    assert!(caps.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Container,
    }));
    assert!(!caps.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Subprocess,
    }));
}

#[test]
fn required_capability_for_local_backend_is_subprocess() {
    let caps = Terminal::default()
        .required_capabilities(&serde_json::json!({
            "command": "x",
            "backend": "local",
        }))
        .unwrap();
    assert!(caps.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Subprocess,
    }));
}

#[test]
fn required_capability_rejects_unknown_backend() {
    let err = Terminal::default()
        .required_capabilities(&serde_json::json!({
            "command": "x",
            "backend": "wat",
        }))
        .unwrap_err();
    assert!(matches!(err, ArgoError::Tool { .. }));
}

#[tokio::test]
async fn dispatch_routes_local_to_local_sandbox() {
    let local = Arc::new(RecordingSandbox::new("local"));
    let sandboxed = Arc::new(RecordingSandbox::new("sandbox"));
    let t = Terminal::with_sandboxes(local.clone(), sandboxed.clone());
    let dir = tempfile::tempdir().unwrap();
    let r = t
        .dispatch(
            serde_json::json!({"command": "echo from-local", "backend": "local"}),
            &ctx(dir.path()),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r));
    assert_eq!(local.calls(), vec!["echo from-local".to_string()]);
    assert!(sandboxed.calls().is_empty());
    assert!(body(&r).contains("from local"));
}

#[tokio::test]
async fn dispatch_routes_sandbox_to_sandbox_backend() {
    let local = Arc::new(RecordingSandbox::new("local"));
    let sandboxed = Arc::new(RecordingSandbox::new("sandbox"));
    let t = Terminal::with_sandboxes(local.clone(), sandboxed.clone());
    let dir = tempfile::tempdir().unwrap();
    let r = t
        .dispatch(
            serde_json::json!({"command": "npm install", "backend": "sandbox"}),
            &ctx(dir.path()),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r));
    assert_eq!(sandboxed.calls(), vec!["npm install".to_string()]);
    assert!(local.calls().is_empty());
    assert!(body(&r).contains("from sandbox"));
}

#[tokio::test]
async fn dispatch_rejects_unknown_backend() {
    let dir = tempfile::tempdir().unwrap();
    let r = Terminal::default()
        .dispatch(
            serde_json::json!({"command": "x", "backend": "wat"}),
            &ctx(dir.path()),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    assert!(body(&r).contains("unknown backend"));
}

#[test]
fn schema_exposes_backend_enum_with_local_default() {
    let p = Terminal::default().parameters();
    let backend = &p["properties"]["backend"];
    assert_eq!(backend["type"], "string");
    assert_eq!(backend["default"], "local");
    assert_eq!(backend["enum"], serde_json::json!(["local", "sandbox"]));
}

#[test]
fn split_chunks_handles_oversize_lines() {
    let chunks = split_chunks("aaaaaaaaaa", 3);
    assert_eq!(chunks, vec!["aaa", "aaa", "aaa", "a"]);
    let chunks = split_chunks("hi", 8);
    assert_eq!(chunks, vec!["hi"]);
}

#[test]
fn render_capped_marker_matches_buffered_path() {
    assert_eq!(render_capped(b"abcd", 4), "abcd");
    assert_eq!(
        render_capped(b"abcd", 100),
        "abcd\n... [truncated 96 bytes from this stream]"
    );
}

/// `progress_tx = None` is a no-op: dispatch goes through the
/// sandbox seam exactly as before.
#[cfg(unix)]
#[tokio::test]
async fn no_progress_tx_routes_through_sandbox_seam() {
    let local = Arc::new(RecordingSandbox::new("local"));
    let sandboxed = Arc::new(RecordingSandbox::new("sandbox"));
    let t = Terminal::with_sandboxes(local.clone(), sandboxed.clone());
    let dir = tempfile::tempdir().unwrap();
    let r = t
        .dispatch(serde_json::json!({"command": "echo hi"}), &ctx(dir.path()))
        .await
        .unwrap();
    assert!(!r.is_error);
    assert_eq!(local.calls(), vec!["echo hi".to_string()]);
}

/// `progress_tx = Some(tx)` on the local backend bypasses the
/// sandbox seam (so the recording sandbox sees no calls) and emits
/// per-line `Stdout` events plus a final `Exited` sentinel.
#[cfg(unix)]
#[tokio::test]
async fn streaming_local_bypasses_sandbox_seam_and_emits_events() {
    let local = Arc::new(RecordingSandbox::new("local"));
    let sandboxed = Arc::new(RecordingSandbox::new("sandbox"));
    let t = Terminal::with_sandboxes(local.clone(), sandboxed.clone());
    let dir = tempfile::tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel::<ToolProgressEvent>(32);
    let r = {
        // Scoped so `context` (and its tx clone) drops before we
        // drain `rx` — otherwise `recv()` would never see `None`.
        let context = ctx(dir.path()).with_progress_tx(tx);
        t.dispatch(
            serde_json::json!({"command": "printf 'a\\nb\\n'"}),
            &context,
        )
        .await
        .unwrap()
    };
    assert!(!r.is_error);
    assert!(local.calls().is_empty(), "streaming should bypass seam");

    let mut stdouts = Vec::new();
    let mut exited = None;
    while let Some(ev) = rx.recv().await {
        match ev {
            ToolProgressEvent::Stdout(s) => stdouts.push(s),
            ToolProgressEvent::Exited { exit_code } => exited = Some(exit_code),
            ToolProgressEvent::Stderr(_) => {}
        }
    }
    assert_eq!(stdouts, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(exited, Some(0));
}

/// Streaming on the sandbox backend keeps the buffered seam path
/// (streaming through container backends is a follow-up).
#[cfg(unix)]
#[tokio::test]
async fn streaming_sandbox_backend_still_uses_seam() {
    let local = Arc::new(RecordingSandbox::new("local"));
    let sandboxed = Arc::new(RecordingSandbox::new("sandbox"));
    let t = Terminal::with_sandboxes(local.clone(), sandboxed.clone());
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = mpsc::channel::<ToolProgressEvent>(8);
    let context = ctx(dir.path()).with_progress_tx(tx);
    let _ = t
        .dispatch(
            serde_json::json!({"command": "echo hi", "backend": "sandbox"}),
            &context,
        )
        .await
        .unwrap();
    assert_eq!(sandboxed.calls(), vec!["echo hi".to_string()]);
    assert!(local.calls().is_empty());
}

/// Backpressure: a tiny channel with no consumer must not block the
/// child. The dispatch must still complete and return a valid result.
#[cfg(unix)]
#[tokio::test]
async fn streaming_backpressure_does_not_deadlock() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, _rx) = mpsc::channel::<ToolProgressEvent>(1);
    let context = ctx(dir.path()).with_progress_tx(tx);
    let r = tokio::time::timeout(
        Duration::from_secs(10),
        Terminal::default().dispatch(
            serde_json::json!({
                "command": "for i in $(seq 1 1000); do echo line-$i; done",
                "timeout_ms": 9000
            }),
            &context,
        ),
    )
    .await
    .expect("dispatch must complete despite full channel")
    .unwrap();
    assert!(!r.is_error);
}
