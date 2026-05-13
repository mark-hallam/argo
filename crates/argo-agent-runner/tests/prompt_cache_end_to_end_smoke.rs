//! End-to-end prompt-cache smoke for the affinity-enabled session path.

use argo_agent_runner::run_turn_with_events;
use argo_store::{PromptCacheAffinity, SessionCreate, SessionStore, WorkerId};
use argo_telemetry::metrics_prompt_cache::{snapshot, PromptCacheStats};
use argo_types::SessionId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[derive(Clone)]
struct AnthropicSequence {
    calls: Arc<AtomicUsize>,
    creation_tokens: u32,
    read_tokens_after_first: u32,
}

impl Respond for AnthropicSequence {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let (cache_creation, cache_read) = if call == 0 {
            (self.creation_tokens, 0)
        } else {
            (0, self.read_tokens_after_first)
        };
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": format!("msg_{call}"),
            "model": "claude-haiku-4-5",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": format!("ok-{call}")}],
            "usage": {
                "input_tokens": 11,
                "output_tokens": 3,
                "cache_creation_input_tokens": cache_creation,
                "cache_read_input_tokens": cache_read
            }
        }))
    }
}

struct Fixture {
    cwd: TempDir,
    _server: MockServer,
    store: Arc<SessionStore>,
    tenant: argo_types::TenantId,
    workspace: argo_types::WorkspaceId,
    provider: String,
    model: String,
    worker: WorkerId,
}

async fn fixture(
    name: &str,
    affinity: Option<Arc<PromptCacheAffinity>>,
    read_tokens_after_first: u32,
) -> Fixture {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(AnthropicSequence {
            calls: Arc::new(AtomicUsize::new(0)),
            creation_tokens: 29,
            read_tokens_after_first,
        })
        .mount(&server)
        .await;

    let cwd = TempDir::new().unwrap();
    let provider = format!("anthropic-cache-{name}");
    let model = format!("claude-haiku-4-5-{name}");
    write_manifest(cwd.path(), &provider, &server.uri());
    let api_key_env = format!("ARGO_PROMPT_CACHE_E2E_KEY_{}", name.to_ascii_uppercase());
    std::env::set_var(api_key_env, "sk-ant-test");

    let mut store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    if let Some(affinity) = affinity {
        store = store.with_affinity(affinity);
    }
    let workspace_path = cwd.path().to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(name, &workspace_path).await.unwrap();

    Fixture {
        cwd,
        _server: server,
        store: Arc::new(store),
        tenant,
        workspace,
        provider,
        model,
        worker: WorkerId(uuid::Uuid::now_v7()),
    }
}

fn write_manifest(cwd: &std::path::Path, provider: &str, base_url: &str) {
    let dir = cwd.join("manifests/providers");
    std::fs::create_dir_all(&dir).unwrap();
    let env_key = format!(
        "ARGO_PROMPT_CACHE_E2E_KEY_{}",
        provider
            .trim_start_matches("anthropic-cache-")
            .to_ascii_uppercase()
    );
    std::fs::write(
        dir.join(format!("{provider}.toml")),
        format!(
            r#"
[provider]
id = "{provider}"
family = "anthropic"
display_name = "Anthropic cache smoke"
base_url = "{base_url}"

[auth]
api_key_env = "{env_key}"
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

[quirks]
prompt-caching-breakpoints = ["system", "tools"]

[[models]]
id = "claude-haiku-4-5"
context_window = 200000
"#
        ),
    )
    .unwrap();
}

async fn mint_session(f: &Fixture) -> SessionId {
    f.store
        .create_session_with_worker(SessionCreate {
            tenant_id: f.tenant,
            workspace_id: f.workspace,
            model: &f.model,
            provider: &f.provider,
            title: Some("prompt-cache-smoke"),
            vertical_id: None,
            worker_id: Some(f.worker),
        })
        .await
        .unwrap()
}

async fn run_turn(f: &Fixture, sid: SessionId, text: &str) {
    run_turn_with_events(&f.store, f.cwd.path(), sid, Some(text), false, 1, None)
        .await
        .unwrap();
}

fn stats(provider: &str, model: &str) -> PromptCacheStats {
    snapshot()
        .into_iter()
        .find(|s| s.provider == provider && s.model == model)
        .unwrap_or(PromptCacheStats {
            provider: provider.to_string(),
            model: model.to_string(),
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            uncached_input_tokens: 0,
            hit_rate: 0.0,
        })
}

#[tokio::test]
async fn affinity_enabled_two_turn_replay_records_cache_read() {
    let f = fixture(
        "enabled",
        Some(Arc::new(PromptCacheAffinity::enabled(Duration::from_secs(
            270,
        )))),
        17,
    )
    .await;

    let sid1 = mint_session(&f).await;
    run_turn(&f, sid1, "first turn").await;
    let after_first = stats(&f.provider, &f.model);
    assert!(after_first.cache_creation_input_tokens > 0);
    assert_eq!(after_first.cache_read_input_tokens, 0);

    let sid2 = mint_session(&f).await;
    assert_eq!(sid1, sid2);
    run_turn(&f, sid2, "second turn").await;
    let after_second = stats(&f.provider, &f.model);
    assert!(after_second.cache_read_input_tokens > after_first.cache_read_input_tokens);
}

#[tokio::test]
async fn affinity_disabled_two_turns_record_no_cache_read() {
    let f = fixture("disabled", None, 0).await;

    let sid1 = mint_session(&f).await;
    run_turn(&f, sid1, "first turn").await;
    let sid2 = mint_session(&f).await;
    assert_ne!(sid1, sid2);
    run_turn(&f, sid2, "second turn").await;

    let after_second = stats(&f.provider, &f.model);
    assert!(after_second.cache_creation_input_tokens > 0);
    assert_eq!(after_second.cache_read_input_tokens, 0);
}
