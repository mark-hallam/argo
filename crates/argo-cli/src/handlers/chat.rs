//! `argo chat <id> [--user <text>]` — run one turn of the agent
//! loop against a recorded session.
//!
//! Dispatches by `manifest.provider.family`:
//!   - `anthropic` → AnthropicTransport
//!   - `openai-compatible` → OpenAiCompatTransport
//!   - `openai-responses` → OpenAiResponsesTransport
//!   - `bedrock` → not yet wired (sigv4 needs daemon plumbing)
//!
//! Calls the provider once, persists the assistant response. No
//! tool dispatch, no streaming, no router — just the happy-path
//! single-turn round-trip. Heavier surfaces (multi-turn loop, tool
//! calls, streaming, cost accounting, audit emission) belong with
//! the daemon-resident agent loop and will land via
//! `POST /sessions/<id>/runs`.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_provider::{
    AnthropicTransport, OpenAiCompatTransport, OpenAiResponsesTransport, ProviderFamily,
    ProviderManifest, ProviderRequest, Sampling, Transport,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId};
use chrono::Utc;
use std::sync::Arc;

pub async fn run(raw: &str, user: Option<&str>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let session = store
        .get_session(SessionId(id))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw}")))?;

    // Optionally append a fresh user message before the call —
    // saves one extra HTTP/CLI round-trip in the common case.
    if let Some(text) = user {
        let content = serde_json::json!([{"type": "text", "text": text}]);
        store
            .append_message(SessionId(id), MessageRole::User, &content.to_string())
            .await?;
    }

    let manifest = load_manifest(&cwd, &session.provider)?;
    let api_key_env = manifest.auth.api_key_env.clone();
    let api_key = std::env::var(&api_key_env)
        .map_err(|_| ArgoError::Provider(format!("{api_key_env} not set")))?;
    let transport: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Anthropic => {
            Box::new(AnthropicTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiCompatible => {
            Box::new(OpenAiCompatTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, &api_key)?)
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "argo chat doesn't yet drive provider family {other:?} \
                 (Bedrock needs sigv4 plumbing the daemon owns)"
            )));
        }
    };

    // Reload messages so the freshly-appended user input is included.
    let rows = store.list_messages(SessionId(id)).await?;
    let messages = rows
        .iter()
        .map(|r| message_from_row(r, SessionId(id)))
        .collect::<ArgoResult<Vec<_>>>()?;

    let req = ProviderRequest {
        model: session.model.clone(),
        messages,
        tools: Vec::new(),
        system: None,
        sampling: Sampling::default(),
    };
    eprintln!(
        "argo chat: → {} {} ({} message(s))",
        session.provider,
        session.model,
        rows.len()
    );
    let resp = transport.complete(&req).await?;

    let content = match (&resp.text, &resp.reasoning) {
        (Some(t), _) => serde_json::json!([{"type": "text", "text": t}]),
        (None, Some(r)) => serde_json::json!([{"type": "thinking", "thinking": r}]),
        (None, None) => serde_json::json!([]),
    };
    let assistant_id = store
        .append_message(SessionId(id), MessageRole::Assistant, &content.to_string())
        .await?;

    println!("argo chat: ← assistant message {assistant_id}");
    if let Some(t) = &resp.text {
        println!("\n{t}\n");
    }
    if let (Some(ti), Some(to)) = (resp.tokens_in, resp.tokens_out) {
        println!("[{ti} prompt + {to} completion tokens]");
    }
    Ok(())
}

fn load_manifest(cwd: &std::path::Path, provider_id: &str) -> ArgoResult<ProviderManifest> {
    let path = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{provider_id}.toml"));
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    ProviderManifest::from_toml_str(&body)
}

fn message_from_row(row: &argo_store::MessageRow, session_id: SessionId) -> ArgoResult<Message> {
    let id_bytes: [u8; 16] = row
        .id
        .as_slice()
        .try_into()
        .map_err(|_| ArgoError::Provider("message id is not 16 bytes".into()))?;
    let role = match row.role.as_str() {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        other => {
            return Err(ArgoError::Provider(format!("unknown role '{other}'")));
        }
    };
    let content: Vec<ContentBlock> = serde_json::from_str(&row.content_json).map_err(|e| {
        ArgoError::Provider(format!("parse content_json: {e}: {}", row.content_json))
    })?;
    let created_at = chrono::DateTime::from_timestamp_micros(row.created_at)
        .ok_or_else(|| ArgoError::Provider("created_at out of range".into()))?
        .with_timezone(&Utc);
    Ok(Message {
        id: MessageId(uuid::Uuid::from_bytes(id_bytes)),
        session_id,
        role,
        content,
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at,
        provider_meta: serde_json::Value::Null,
    })
}
