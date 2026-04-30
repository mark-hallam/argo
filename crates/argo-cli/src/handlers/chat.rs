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
    ProviderManifest, ProviderRequest, ProviderResponseDelta, Sampling, Transport,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId};
use chrono::Utc;
use futures::StreamExt;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Outcome of one agent-loop turn. Consumed by both the CLI
/// handler (which prints) and the gateway's
/// `POST /sessions/<id>/runs` endpoint (which serializes to JSON).
pub struct TurnResult {
    /// Persisted assistant message id.
    pub assistant_id: Uuid,
    /// Full assistant text.
    pub text: String,
    /// Provider-reported input tokens, if any.
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens, if any.
    pub tokens_out: Option<u32>,
}

pub async fn run(raw: &str, user: Option<&str>, stream: bool, interactive: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;

    if interactive {
        return run_interactive(&store, &cwd, SessionId(id), user, stream).await;
    }

    eprintln!("argo chat: → session {id} (stream={stream})");
    let result = run_turn(&store, &cwd, SessionId(id), user, stream).await?;
    println!("\nargo chat: ← assistant message {}", result.assistant_id);
    if !stream && !result.text.is_empty() {
        println!("\n{}\n", result.text);
    }
    if let (Some(ti), Some(to)) = (result.tokens_in, result.tokens_out) {
        println!("[{ti} prompt + {to} completion tokens]");
    }
    Ok(())
}

async fn run_interactive(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    seed: Option<&str>,
    stream: bool,
) -> ArgoResult<()> {
    use std::io::BufRead;
    eprintln!(
        "argo chat: interactive mode against session {} (stream={stream})",
        session_id.0
    );
    eprintln!("type a line to send, Ctrl-D / EOF to exit\n");

    // First turn uses the optional --user seed (if any) so callers
    // can prime the REPL from the command line.
    if let Some(text) = seed {
        eprintln!("> {text}");
        let result = run_turn(store, cwd, session_id, Some(text), stream).await?;
        print_result(&result, stream);
    }

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        eprint!("> ");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let line = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(e)) => return Err(ArgoError::Provider(format!("stdin: {e}"))),
            None => {
                eprintln!("\nargo chat: EOF — bye");
                return Ok(());
            }
        };
        if line.is_empty() {
            continue;
        }
        let result = run_turn(store, cwd, session_id, Some(&line), stream).await?;
        print_result(&result, stream);
    }
}

fn print_result(result: &TurnResult, stream: bool) {
    println!("\nargo chat: ← assistant message {}", result.assistant_id);
    if !stream && !result.text.is_empty() {
        println!("\n{}\n", result.text);
    }
    if let (Some(ti), Some(to)) = (result.tokens_in, result.tokens_out) {
        println!("[{ti} prompt + {to} completion tokens]");
    }
}

/// One agent-loop turn against an open SessionStore.
///
/// Reusable from both the CLI handler and the gateway's
/// `POST /sessions/<id>/runs` endpoint. When `stream = true` the
/// assistant text is flushed to stdout as it arrives (no-op when
/// stdout is captured / piped).
pub async fn run_turn(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
) -> ArgoResult<TurnResult> {
    let session = store
        .get_session(session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {}", session_id.0)))?;

    if let Some(text) = user {
        let content = serde_json::json!([{"type": "text", "text": text}]);
        store
            .append_message(session_id, MessageRole::User, &content.to_string())
            .await?;
    }

    let manifest = load_manifest(cwd, &session.provider)?;
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

    let rows = store.list_messages(session_id).await?;
    let messages = rows
        .iter()
        .map(|r| message_from_row(r, session_id))
        .collect::<ArgoResult<Vec<_>>>()?;

    let req = ProviderRequest {
        model: session.model.clone(),
        messages,
        tools: Vec::new(),
        system: None,
        sampling: Sampling::default(),
    };
    let (text, tokens_in, tokens_out) = if stream {
        run_stream(transport.as_ref(), &req).await?
    } else {
        let resp = transport.complete(&req).await?;
        (
            resp.text.unwrap_or_default(),
            resp.tokens_in,
            resp.tokens_out,
        )
    };
    let content = if text.is_empty() {
        serde_json::json!([])
    } else {
        serde_json::json!([{"type": "text", "text": text}])
    };
    let assistant_id = store
        .append_message(session_id, MessageRole::Assistant, &content.to_string())
        .await?;
    Ok(TurnResult {
        assistant_id,
        text,
        tokens_in,
        tokens_out,
    })
}

async fn run_stream(
    transport: &dyn Transport,
    req: &ProviderRequest,
) -> ArgoResult<(String, Option<u32>, Option<u32>)> {
    let mut stream = transport.complete_stream(req).await?;
    let mut text = String::new();
    let mut tokens_in: Option<u32> = None;
    let mut tokens_out: Option<u32> = None;
    let mut stdout = std::io::stdout();
    while let Some(delta) = stream.next().await {
        match delta? {
            ProviderResponseDelta::TextChunk(s) => {
                text.push_str(&s);
                let _ = stdout.write_all(s.as_bytes());
                let _ = stdout.flush();
            }
            ProviderResponseDelta::TokenUsage {
                tokens_in: ti,
                tokens_out: to,
            } => {
                tokens_in = Some(ti);
                tokens_out = Some(to);
            }
            ProviderResponseDelta::FinishReason(_)
            | ProviderResponseDelta::ReasoningChunk(_)
            | ProviderResponseDelta::ToolCallDelta { .. } => {
                // No-op for now — single-turn happy path. Reasoning
                // / tool-call streams will be wired with the agent
                // loop's full surface.
            }
        }
    }
    Ok((text, tokens_in, tokens_out))
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
