//! Driver tests for the Claude Code CLI subprocess provider.
//!
//! Each test invokes a mock `claude` binary (a bash script at
//! `tests/fixtures/mock-claude.sh`) and asserts the driver's stdout
//! parsing, error reporting, and timeout behaviour. The fixture's
//! behaviour is selected via `MOCK_CLAUDE_MODE` — see the script header.
//!
//! Env-var mutation across tests is serialised with a global mutex so
//! the tokio test runner can use multiple worker threads safely.
use super::driver::{parsed_to_response, ClaudeCodeCliTransport};
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, Sampling, StopReason, Transport};
use argo_types::{ContentBlock, Message, MessageId, MessageRole, SessionId};
use chrono::Utc;
use std::path::PathBuf;
use tokio::sync::Mutex;

/// Serialises env-var mutations across tests in this file. The CLI driver
/// reads `ARGO_CLAUDE_CODE_BINARY` from the process env at spawn time;
/// without a mutex, two parallel tests could clobber each other's value.
/// Async mutex so the guard can be held across `.await` cleanly.
static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const MANIFEST_TOML: &str = include_str!("../../../../manifests/providers/claude-code-cli.toml");

fn manifest() -> ProviderManifest {
    ProviderManifest::from_toml_str(MANIFEST_TOML).expect("manifest parses")
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mock-claude.sh")
}

fn user_msg(text: &str) -> Message {
    Message {
        id: MessageId::new(),
        session_id: SessionId::new(),
        role: MessageRole::User,
        content: vec![ContentBlock::Text { text: text.into() }],
        tool_calls: vec![],
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

fn req() -> ProviderRequest {
    req_with_model("claude-sonnet-4-6")
}

fn req_with_model(model: &str) -> ProviderRequest {
    ProviderRequest {
        model: model.into(),
        messages: vec![user_msg("hi")],
        tools: vec![],
        system: Some("you are helpful".into()),
        sampling: Sampling::default(),
    }
}

/// Scoped env-var setter that restores the previous value on drop.
struct EnvGuard {
    keys: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn new() -> Self {
        Self { keys: Vec::new() }
    }

    fn set(&mut self, key: &str, value: &str) {
        let prev = std::env::var(key).ok();
        self.keys.push((key.to_string(), prev));
        std::env::set_var(key, value);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, prev) in self.keys.drain(..) {
            match prev {
                Some(v) => std::env::set_var(&k, v),
                None => std::env::remove_var(&k),
            }
        }
    }
}

/// A scratch file under the OS temp dir that the mock binary writes its
/// argv into, removed on drop. Avoids pulling in a `tempfile`
/// dev-dependency for the three model-pinning tests.
struct ArgvCapture {
    path: PathBuf,
}

impl ArgvCapture {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "argo-mock-claude-argv-{tag}-{}-{nanos}.txt",
            std::process::id()
        ));
        Self { path }
    }

    fn path_str(&self) -> &str {
        self.path.to_str().expect("temp path is utf8")
    }

    fn read_lines(&self) -> Vec<String> {
        std::fs::read_to_string(&self.path)
            .expect("read captured argv")
            .lines()
            .map(str::to_string)
            .collect()
    }
}

impl Drop for ArgvCapture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn from_manifest_rejects_http_transport_kind() {
    // Swap the manifest's transport_kind for the default ("http") and
    // assert the constructor surfaces the misconfiguration.
    let mut m = manifest();
    m.provider.transport_kind = crate::manifest::TransportKind::Http;
    let err = ClaudeCodeCliTransport::from_manifest(m).unwrap_err();
    assert!(
        format!("{err}").contains("transport_kind"),
        "error should name transport_kind; got: {err}"
    );
}

#[test]
fn from_manifest_requires_subprocess_block() {
    let mut m = manifest();
    m.subprocess = None;
    let err = ClaudeCodeCliTransport::from_manifest(m).unwrap_err();
    assert!(
        format!("{err}").contains("[subprocess]"),
        "error should name the [subprocess] block; got: {err}"
    );
}

#[test]
fn parsed_to_response_maps_canonical_fields() {
    let parsed: super::cli_response::ClaudeCliJsonResponse = serde_json::from_str(
        r#"{"result":"hi","stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":3},"modelUsage":{"claude-sonnet-4-6":{"outputTokens":3}}}"#,
    )
    .expect("parse");
    let resp = parsed_to_response(parsed, "claude-code-cli", None);
    assert_eq!(resp.text.as_deref(), Some("hi"));
    assert_eq!(resp.tokens_in, Some(10));
    assert_eq!(resp.tokens_out, Some(3));
    assert!(matches!(resp.stop_reason, StopReason::EndTurn));
    assert_eq!(resp.provider_meta["model"], "claude-sonnet-4-6");
    assert_eq!(resp.provider_meta["provider"], "claude-code-cli");
}

#[test]
fn pinned_model_overrides_dominant_model_for_reporting() {
    // The CLI's internal title-gen side-call (Haiku) out-tokens the
    // main answer, so `dominant_model()` would report Haiku. When the
    // request pinned Sonnet, the reported model must be Sonnet so the
    // audit row reflects the model that actually answered — and Haiku
    // never surfaces as the answering model.
    let parsed: super::cli_response::ClaudeCliJsonResponse = serde_json::from_str(
        r#"{"result":"pong","stop_reason":"end_turn","modelUsage":{"claude-haiku-4-5-20251001":{"outputTokens":50},"claude-sonnet-4-6":{"outputTokens":2}}}"#,
    )
    .expect("parse");
    let resp = parsed_to_response(parsed, "claude-code-cli", Some("claude-sonnet-4-6"));
    assert_eq!(
        resp.provider_meta["model"], "claude-sonnet-4-6",
        "pinned model must win over the higher-token Haiku side-call"
    );
}

#[tokio::test]
async fn complete_parses_canonical_mock_output() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "ok");

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let resp = transport.complete(&req()).await.expect("complete");
    assert_eq!(resp.text.as_deref(), Some("hello from mock"));
    assert_eq!(resp.tokens_in, Some(10));
    assert_eq!(resp.tokens_out, Some(3));
    assert!(matches!(resp.stop_reason, StopReason::EndTurn));
}

#[tokio::test]
async fn complete_accepts_brief_reference_shape() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "brief_shape");

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let resp = transport.complete(&req()).await.expect("complete");
    assert_eq!(resp.text.as_deref(), Some("hello from mock"));
    assert_eq!(resp.provider_meta["model"], "claude-sonnet-4-6");
}

#[tokio::test]
async fn complete_returns_error_on_nonzero_exit() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "error");

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let err = transport.complete(&req()).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("exit") && msg.contains("simulated failure"),
        "error should carry stderr context; got: {msg}"
    );
}

#[tokio::test]
async fn complete_returns_error_on_is_error_flag() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "is_error");

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let err = transport.complete(&req()).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("is_error") && msg.contains("upstream refused"),
        "error should preserve upstream refusal body; got: {msg}"
    );
}

#[tokio::test]
async fn complete_returns_error_on_garbage_stdout() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "garbage");

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let err = transport.complete(&req()).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("JSON parse"),
        "error should name JSON parse failure; got: {msg}"
    );
}

#[tokio::test]
async fn complete_times_out_when_subprocess_runs_long() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "sleep");
    env.set("MOCK_CLAUDE_SLEEP_SECS", "10");

    // Override the manifest's 30s timeout with a 200ms one so the test
    // returns fast.
    let mut m = manifest();
    if let Some(sp) = m.subprocess.as_mut() {
        sp.timeout_ms = 200;
    }
    let transport = ClaudeCodeCliTransport::from_manifest(m).expect("build transport");
    let err = transport.complete(&req()).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("timeout"),
        "error should name timeout; got: {msg}"
    );
}

#[tokio::test]
async fn complete_stream_emits_text_then_finish() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "ok");

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let mut stream = transport.complete_stream(&req()).await.expect("stream");
    use futures::StreamExt;
    let mut deltas = Vec::new();
    while let Some(d) = stream.next().await {
        deltas.push(d.expect("delta"));
    }
    // Expect: TextChunk, TokenUsage, FinishReason — in that order.
    assert!(matches!(
        deltas.first(),
        Some(crate::transport::ProviderResponseDelta::TextChunk(t)) if t == "hello from mock"
    ));
    assert!(deltas.iter().any(|d| matches!(
        d,
        crate::transport::ProviderResponseDelta::TokenUsage {
            tokens_in: 10,
            tokens_out: 3
        }
    )));
    assert!(matches!(
        deltas.last(),
        Some(crate::transport::ProviderResponseDelta::FinishReason(
            StopReason::EndTurn
        ))
    ));
}

#[tokio::test]
async fn complete_errors_when_binary_does_not_exist() {
    let _lock = ENV_LOCK.lock().await;
    let mut env = EnvGuard::new();
    env.set(
        "ARGO_CLAUDE_CODE_BINARY",
        "/nonexistent/path/definitely-not-here/claude",
    );

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    let err = transport.complete(&req()).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("spawn"),
        "error should name spawn failure; got: {msg}"
    );
}

#[tokio::test]
async fn complete_passes_model_flag_when_pinned() {
    let _lock = ENV_LOCK.lock().await;
    let argv_out = ArgvCapture::new("sonnet");
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "ok");
    env.set("MOCK_CLAUDE_ARGV_OUT", argv_out.path_str());

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    transport
        .complete(&req_with_model("claude-sonnet-4-6"))
        .await
        .expect("complete");

    let lines = argv_out.read_lines();
    // `--model` must be immediately followed by the pinned id, after the
    // manifest's default_args (`--print --output-format json`).
    let model_idx = lines
        .iter()
        .position(|a| a == "--model")
        .expect("driver must pass --model when the request pins a model");
    assert_eq!(
        lines.get(model_idx + 1).map(String::as_str),
        Some("claude-sonnet-4-6"),
        "--model must carry the pinned id; argv: {lines:?}"
    );
    // Hard floor: Haiku must never be selected in the dev routing path.
    assert!(
        !lines.iter().any(|a| a.contains("haiku")),
        "Haiku must never appear in the spawned args; argv: {lines:?}"
    );
}

#[tokio::test]
async fn complete_passes_opus_model_flag_for_complex_tier() {
    let _lock = ENV_LOCK.lock().await;
    let argv_out = ArgvCapture::new("opus");
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "ok");
    env.set("MOCK_CLAUDE_ARGV_OUT", argv_out.path_str());

    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    transport
        .complete(&req_with_model("claude-opus-4-8"))
        .await
        .expect("complete");

    let lines = argv_out.read_lines();
    assert!(
        lines.iter().any(|a| a == "claude-opus-4-8"),
        "complex tier must pin Opus 4.8; argv: {lines:?}"
    );
}

#[tokio::test]
async fn complete_omits_model_flag_when_unpinned() {
    let _lock = ENV_LOCK.lock().await;
    let argv_out = ArgvCapture::new("unpinned");
    let mut env = EnvGuard::new();
    env.set("ARGO_CLAUDE_CODE_BINARY", fixture_path().to_str().unwrap());
    env.set("MOCK_CLAUDE_MODE", "ok");
    env.set("MOCK_CLAUDE_ARGV_OUT", argv_out.path_str());

    // Empty model id = unpinned; the driver must preserve the CLI's
    // auto-route behaviour by NOT passing `--model` (backward compat).
    let transport = ClaudeCodeCliTransport::from_manifest(manifest()).expect("build transport");
    transport
        .complete(&req_with_model(""))
        .await
        .expect("complete");

    let lines = argv_out.read_lines();
    assert!(
        !lines.iter().any(|a| a == "--model"),
        "unpinned request must NOT pass --model; argv: {lines:?}"
    );
}
