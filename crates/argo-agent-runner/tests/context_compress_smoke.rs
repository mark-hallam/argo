//! End-to-end smoke for context compression/truncation in the turn loop.

use argo_agent_runner::run_turn_with_events;
use argo_store::{SessionCreate, SessionStore};
use argo_types::{ContentBlock, MessageRole};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, Respond, ResponseTemplate,
};

#[derive(Clone, Default)]
struct Capture {
    count: Arc<Mutex<usize>>,
    body: Arc<Mutex<String>>,
}

impl Respond for Capture {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        *self.count.lock().unwrap() = body["messages"].as_array().map_or(0, Vec::len);
        *self.body.lock().unwrap() = body.to_string();
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_context_smoke",
            "model": "claude-context-smoke",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "ok"}],
            "usage": {"input_tokens": 11, "output_tokens": 3}
        }))
    }
}

struct Fixture {
    cwd: TempDir,
    _server: MockServer,
    store: Arc<SessionStore>,
    capture: Capture,
}

async fn fixture() -> Fixture {
    let server = MockServer::start().await;
    let capture = Capture::default();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(capture.clone())
        .mount(&server)
        .await;
    let cwd = TempDir::new().unwrap();
    write_files(cwd.path(), &server.uri());
    std::env::set_var("ARGO_CONTEXT_SMOKE_KEY", "sk-ant-test");
    std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    Fixture {
        cwd,
        _server: server,
        store,
        capture,
    }
}

fn write_files(cwd: &std::path::Path, base_url: &str) {
    let dir = cwd.join("manifests/providers");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        cwd.join("argo.toml"),
        "[runtime.context_compress]\nenabled=true\ntrigger_pct=1\nkeep_last_turns=1\nsummary_max_tokens=20\n",
    ).unwrap();
    std::fs::write(
        dir.join("anthropic-context-smoke.toml"),
        format!(
            "[provider]\nid=\"anthropic-context-smoke\"\nfamily=\"anthropic\"\n\
             display_name=\"Anthropic context smoke\"\nbase_url=\"{base_url}\"\n\
             [auth]\napi_key_env=\"ARGO_CONTEXT_SMOKE_KEY\"\nheader_name=\"x-api-key\"\n\
             header_format=\"{{key}}\"\n[transport]\nstreaming=\"sse\"\n\
             chat_path=\"/v1/messages\"\nconnect_timeout_ms=1000\nread_timeout_ms=5000\n\
             max_retries=0\ninitial_backoff_ms=1\nmax_backoff_ms=1\n[tool_calls]\n\
             format=\"anthropic_tool_use\"\n[[models]]\nid=\"claude-context-smoke\"\n\
             context_window=100\n"
        ),
    )
    .unwrap();
}

async fn append_text(f: &Fixture, sid: argo_types::SessionId, role: MessageRole, text: String) {
    let body = serde_json::to_string(&vec![ContentBlock::Text { text }]).unwrap();
    f.store.append_message(sid, role, &body).await.unwrap();
}

#[tokio::test]
async fn long_context_is_compressed_then_truncated_before_provider_call() {
    let f = fixture().await;
    let (tenant, workspace) = f
        .store
        .ensure_workspace("context-smoke", f.cwd.path().to_string_lossy().as_ref())
        .await
        .unwrap();
    let sid = f
        .store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: "claude-context-smoke",
            provider: "anthropic-context-smoke",
            title: Some("context-smoke"),
            vertical_id: None,
            worker_id: None,
        })
        .await
        .unwrap();
    for i in 0..8 {
        append_text(
            &f,
            sid,
            MessageRole::User,
            format!("old user {i} {}", "a".repeat(160)),
        )
        .await;
        append_text(
            &f,
            sid,
            MessageRole::Assistant,
            format!("old assistant {i} {}", "b".repeat(160)),
        )
        .await;
    }
    let latest = "latest user turn ".to_string() + &"z".repeat(1200);
    append_text(&f, sid, MessageRole::User, latest).await;
    f.store.flush_pending_writes().await.unwrap();

    run_turn_with_events(&f.store, f.cwd.path(), sid, None, false, 1, None)
        .await
        .unwrap();

    let audit = f.store.list_audit_for_session(sid).await.unwrap();
    assert!(audit.iter().any(|r| r.kind == "runtime.context_compressed"));
    assert!(audit.iter().any(|r| {
        r.kind == "http.request" && r.payload_json.contains("runtime.context_truncated")
    }));
    assert!(*f.capture.count.lock().unwrap() < 19);
    assert!(f.capture.body.lock().unwrap().contains("truncated"));
}
