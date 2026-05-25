//! End-to-end pin for live runner handoff-chain inheritance.
//!
//! When a worker turn is servicing an inbound handoff, `delegate_task`
//! must see that active handoff id on `ToolContext` and prepend the
//! delegated worker to the still-active escalation chain.

use argo_agent_runner::run_turn_with_events;
use argo_orchestrator::{
    EscalationChain, EscalationStep, Handoff, OrchestratorStore, SessionId as OrchSessionId,
};
use argo_store::{SessionCreate, SessionStore, StoreResult, WorkerId};
use argo_types::{SessionId, TenantId};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const PROVIDER: &str = "orchestrator";
const MODEL: &str = "claude-delegate-chain-smoke";
const API_KEY_ENV: &str = "ARGO_DELEGATE_CHAIN_SMOKE_KEY";
const HUMAN_FALLBACK: &str = "ops@example.test";

#[derive(Clone)]
struct DelegateSequence {
    calls: Arc<AtomicUsize>,
    callee: WorkerId,
}

impl DelegateSequence {
    fn new(callee: WorkerId) -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            callee,
        }
    }
}

impl Respond for DelegateSequence {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        match call {
            0 => tool_use(
                call,
                "delegate_task",
                serde_json::json!({
                    "worker_id": self.callee.to_string(),
                    "task": "verify the downstream work",
                }),
            ),
            1 => {
                let result = latest_tool_result(&body);
                assert!(
                    !result.trim().is_empty(),
                    "delegate_task should return the emitted handoff id"
                );
                text(call, "delegated")
            }
            other => panic!("unexpected provider call #{other}: {body}"),
        }
    }
}

struct Fixture {
    cwd: TempDir,
    _server: MockServer,
    store: Arc<SessionStore>,
    tenant: TenantId,
    session: SessionId,
    receiver: WorkerId,
    callee: WorkerId,
    sequence: DelegateSequence,
}

impl Fixture {
    fn orchestrator(&self) -> OrchestratorStore {
        OrchestratorStore::new(Arc::clone(&self.store))
    }

    fn orch_session_id(&self) -> OrchSessionId {
        OrchSessionId::from_string(self.session.to_string())
    }
}

async fn fixture(with_active_handoff: bool) -> StoreResult<Fixture> {
    let receiver = WorkerId::new();
    let callee = WorkerId::new();
    let sequence = DelegateSequence::new(callee);

    let server = MockServer::start().await;
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
    let (tenant, workspace) = store
        .ensure_workspace("delegate-chain", cwd.path().to_string_lossy().as_ref())
        .await?;
    let session = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: MODEL,
            provider: PROVIDER,
            title: Some("delegate-chain"),
            vertical_id: None,
            worker_id: Some(receiver),
        })
        .await?;

    let f = Fixture {
        cwd,
        _server: server,
        store,
        tenant,
        session,
        receiver,
        callee,
        sequence,
    };
    if with_active_handoff {
        insert_active_handoff(&f).await?;
    }
    Ok(f)
}

async fn insert_active_handoff(f: &Fixture) -> StoreResult<()> {
    let mut active = Handoff::new(
        f.orch_session_id(),
        WorkerId::new(),
        EscalationStep::Worker(f.receiver),
        "complete the parent task",
        serde_json::json!({}),
    );
    active.set_escalation_chain(
        &EscalationChain::new(vec![
            EscalationStep::Worker(f.receiver),
            EscalationStep::Human(HUMAN_FALLBACK.to_string()),
        ]),
        0,
    );
    f.orchestrator().insert_handoff(f.tenant, &active).await
}

#[tokio::test]
async fn delegate_task_inherits_active_inbound_handoff_chain() {
    let f = fixture(true).await.expect("fixture");

    let result = run_turn_with_events(
        &f.store,
        f.cwd.path(),
        f.session,
        Some("delegate from active handoff"),
        false,
        2,
        None,
    )
    .await
    .expect("run turn");

    assert_eq!(result.text, "delegated");
    assert_eq!(f.sequence.calls.load(Ordering::SeqCst), 2);

    let emitted = emitted_handoff(&f).await;
    assert_eq!(
        emitted.payload.get("escalation_chain"),
        Some(&serde_json::json!([
            format!("worker:{}", f.callee),
            format!("worker:{}", f.receiver),
            format!("human:{HUMAN_FALLBACK}"),
        ]))
    );
    assert_eq!(
        emitted.payload.get("escalation_index"),
        Some(&serde_json::json!(0))
    );
}

#[tokio::test]
async fn delegate_task_without_inbound_handoff_leaves_chain_unset() {
    let f = fixture(false).await.expect("fixture");

    let result = run_turn_with_events(
        &f.store,
        f.cwd.path(),
        f.session,
        Some("delegate without active handoff"),
        false,
        2,
        None,
    )
    .await
    .expect("run turn");

    assert_eq!(result.text, "delegated");
    assert_eq!(f.sequence.calls.load(Ordering::SeqCst), 2);

    let emitted = emitted_handoff(&f).await;
    assert!(emitted.payload.get("escalation_chain").is_none());
    assert!(emitted.payload.get("escalation_index").is_none());
}

async fn emitted_handoff(f: &Fixture) -> Handoff {
    f.orchestrator()
        .list_handoffs_for_session(f.tenant, &f.orch_session_id())
        .await
        .expect("list handoffs")
        .into_iter()
        .find(|handoff| handoff.to == EscalationStep::Worker(f.callee))
        .expect("delegated handoff")
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
display_name = "Delegate chain smoke"
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
