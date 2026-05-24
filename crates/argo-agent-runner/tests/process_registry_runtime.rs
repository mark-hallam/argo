//! End-to-end pin for the agent-runner background process registry.
//!
//! The `terminal { background: true }` tool returns a process id in
//! one turn; later `process` calls in the same session must see and
//! manage that id. A different tenant must not see it.

use argo_agent_runner::run_turn_with_events;
use argo_store::{SessionStore, StoreResult};
use argo_types::SessionId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const PROVIDER: &str = "anthropic-process-registry";
const MODEL: &str = "claude-process-registry-smoke";
const API_KEY_ENV: &str = "ARGO_PROCESS_REGISTRY_E2E_KEY";

#[derive(Clone, Default)]
struct ProcessRegistrySequence {
    calls: Arc<AtomicUsize>,
    pid: Arc<Mutex<Option<String>>>,
}

impl ProcessRegistrySequence {
    fn stored_pid(&self) -> String {
        self.pid
            .lock()
            .unwrap()
            .as_ref()
            .expect("pid should have been captured")
            .clone()
    }

    fn remember_pid(&self, body: &serde_json::Value) -> String {
        let result = latest_tool_result(body);
        let pid = find_process_id(&result).expect("terminal result should include pid");
        *self.pid.lock().unwrap() = Some(pid.clone());
        pid
    }
}

impl Respond for ProcessRegistrySequence {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        let body_text = body.to_string();
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        match call {
            0 => tool_use(
                call,
                "terminal",
                serde_json::json!({"command": "sleep 20", "background": true}),
            ),
            1 => {
                let pid = self.remember_pid(&body);
                text(call, &format!("spawned {pid}"))
            }
            2 => tool_use(call, "process", serde_json::json!({"action": "list"})),
            3 => {
                let pid = self.stored_pid();
                let result = latest_tool_result(&body);
                assert!(
                    result.trim() == "[]" && !result.contains(&pid),
                    "tenant B must not see tenant A pid {pid}; tool result: {result}"
                );
                text(call, "tenant b saw no processes")
            }
            4 => tool_use(call, "process", serde_json::json!({"action": "list"})),
            5 => {
                let pid = self.stored_pid();
                let result = latest_tool_result(&body);
                assert!(
                    result.contains(&pid) && result.contains("sleep 20"),
                    "tenant A list must include pid {pid}; tool result: {result}"
                );
                tool_use(
                    call,
                    "process",
                    serde_json::json!({"action": "kill", "pid": pid}),
                )
            }
            6 => {
                let pid = self.stored_pid();
                let result = latest_tool_result(&body);
                assert!(
                    result.contains(&format!("killed: {pid}")),
                    "tenant A kill result must confirm pid {pid}; tool result: {result}"
                );
                text(call, "cleaned")
            }
            other => panic!("unexpected provider call #{other}: {body_text}"),
        }
    }
}

struct Fixture {
    cwd: TempDir,
    _server: MockServer,
    store: Arc<SessionStore>,
    session_a: SessionId,
    session_b: SessionId,
    sequence: ProcessRegistrySequence,
}

async fn fixture() -> StoreResult<Fixture> {
    let server = MockServer::start().await;
    let sequence = ProcessRegistrySequence::default();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(sequence.clone())
        .mount(&server)
        .await;

    let cwd = TempDir::new().expect("tempdir");
    write_manifest(cwd.path(), &server.uri());
    std::env::set_var(API_KEY_ENV, "sk-ant-test");
    std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");

    let store = Arc::new(SessionStore::open(":memory:").await?);
    store.migrate().await?;
    let (tenant_a, workspace_a) = store
        .ensure_workspace("tenant-a", &cwd.path().join("tenant-a").to_string_lossy())
        .await?;
    let (tenant_b, workspace_b) = store
        .ensure_workspace("tenant-b", &cwd.path().join("tenant-b").to_string_lossy())
        .await?;
    let session_a = store
        .create_session(tenant_a, workspace_a, MODEL, PROVIDER, Some("a"), None)
        .await?;
    let session_b = store
        .create_session(tenant_b, workspace_b, MODEL, PROVIDER, Some("b"), None)
        .await?;

    Ok(Fixture {
        cwd,
        _server: server,
        store,
        session_a,
        session_b,
        sequence,
    })
}

#[tokio::test]
async fn background_processes_survive_turns_and_stay_tenant_scoped() {
    let f = fixture().await.expect("fixture");

    run_turn_with_events(
        &f.store,
        f.cwd.path(),
        f.session_a,
        Some("spawn background process"),
        false,
        2,
        None,
    )
    .await
    .expect("tenant A spawn");

    run_turn_with_events(
        &f.store,
        f.cwd.path(),
        f.session_b,
        Some("list background processes"),
        false,
        2,
        None,
    )
    .await
    .expect("tenant B list");

    let result = run_turn_with_events(
        &f.store,
        f.cwd.path(),
        f.session_a,
        Some("list and kill background process"),
        false,
        3,
        None,
    )
    .await
    .expect("tenant A list + kill");

    assert_eq!(result.text, "cleaned");
    assert_eq!(f.sequence.calls.load(Ordering::SeqCst), 7);
}

fn write_manifest(cwd: &std::path::Path, base_url: &str) {
    let dir = cwd.join("manifests/providers");
    std::fs::create_dir_all(&dir).expect("manifest dir");
    std::fs::write(
        dir.join(format!("{PROVIDER}.toml")),
        format!(
            r#"
[provider]
id = "{PROVIDER}"
family = "anthropic"
display_name = "Process registry smoke"
base_url = "{base_url}"

[auth]
api_key_env = "{API_KEY_ENV}"
header_name = "x-api-key"
header_format = "{{key}}"

[transport]
streaming = "sse"
chat_path = "/v1/messages"
connect_timeout_ms = 1000
read_timeout_ms = 5000
max_retries = 0
initial_backoff_ms = 1
max_backoff_ms = 1

[tool_calls]
format = "anthropic_tool_use"

[[models]]
id = "{MODEL}"
context_window = 200000
"#
        ),
    )
    .expect("write manifest");
}

fn tool_use(call: usize, name: &str, input: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": format!("msg_{call}"),
        "model": MODEL,
        "stop_reason": "tool_use",
        "content": [{
            "type": "tool_use",
            "id": format!("toolu_{call}"),
            "name": name,
            "input": input,
        }],
        "usage": {"input_tokens": 11, "output_tokens": 3}
    }))
}

fn text(call: usize, value: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": format!("msg_{call}"),
        "model": MODEL,
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": value}],
        "usage": {"input_tokens": 11, "output_tokens": 3}
    }))
}

fn find_process_id(text: &str) -> Option<String> {
    let rest = text.split("process_id: ").nth(1)?;
    let pid = rest
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
        .collect::<String>();
    (!pid.is_empty()).then_some(pid)
}

fn latest_tool_result(body: &serde_json::Value) -> String {
    body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|message| message["content"].as_array().into_iter().flatten())
        .filter(|block| block["type"].as_str() == Some("tool_result"))
        .filter_map(|block| block["content"].as_str())
        .next_back()
        .expect("request should contain a tool_result block")
        .to_string()
}
