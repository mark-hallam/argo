//! `argo chat <id> [--user <text>]` — run multi-turn agent loop
//! against a recorded session.
//!
//! Dispatches by `manifest.provider.family`:
//!   - `anthropic` → AnthropicTransport
//!   - `openai-compatible` → OpenAiCompatTransport
//!   - `openai-responses` → OpenAiResponsesTransport
//!   - `bedrock` → not yet wired (sigv4 needs daemon plumbing)
//!
//! When the response carries `tool_calls`, each is dispatched
//! through `argo_tool::ToolRegistry` (capability-checked via
//! `CapabilityEngine`), the result is appended as a tool-role
//! message, and the loop runs another provider turn. Capped at
//! `MAX_TOOL_ROUNDS` to bound runaway loops. Streaming
//! (`--stream`) stays single-turn for now; multi-turn streaming
//! lands once the wire layer can accumulate tool-call deltas
//! through the agent loop.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_capability::CapabilityEngine;
use argo_provider::{
    AnthropicTransport, OpenAiCompatTransport, OpenAiResponsesTransport, ProviderFamily,
    ProviderManifest, ProviderRequest, ProviderResponseDelta, Sampling,
    ToolSchema as ProviderToolSchema, Transport,
};
use argo_store::SessionStore;
use argo_tool::{builtin_tools, ToolContext, ToolRegistry};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, Principal, SessionId,
    ToolCallId, ToolCallRequest, ToolCallResult,
};
use chrono::Utc;
use futures::StreamExt;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Hard cap on tool-dispatch rounds within a single `run_turn` to
/// stop runaway loops. Generous enough for legitimate planning
/// chains; small enough that an accidental loop fails fast.
const MAX_TOOL_ROUNDS: usize = 8;

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
/// assistant text is flushed to stdout as it arrives — streaming
/// stays single-turn for now (no tool dispatch). Non-streaming
/// runs the full tool-dispatch loop, capped at
/// [`MAX_TOOL_ROUNDS`] rounds.
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

    if stream {
        return run_turn_stream(store, transport.as_ref(), session_id, &session.model).await;
    }
    run_turn_with_tools(store, transport.as_ref(), cwd, session_id, &session.model).await
}

/// Build a tool registry + capability engine that grants the
/// local OS user the `owner` role. `argo chat` runs against the
/// invoking user's workspace, so a permissive grant is correct
/// here — the daemon path will narrow this once tenants land.
fn build_dispatch_ctx(cwd: &Path) -> ArgoResult<(ToolRegistry, CapabilityEngine, Principal)> {
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd.to_path_buf()) {
        reg.register(t);
    }
    let engine = CapabilityEngine::with_defaults();
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "argo".into());
    let principal = Principal::Local { os_user };
    engine.bind(principal.clone(), "owner".into())?;
    Ok((reg, engine, principal))
}

fn provider_schemas(reg: &ToolRegistry) -> Vec<ProviderToolSchema> {
    reg.schemas()
        .into_iter()
        .map(|s| ProviderToolSchema {
            name: s.name,
            description: s.description,
            parameters: s.parameters,
        })
        .collect()
}

async fn run_turn_with_tools(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    cwd: &Path,
    session_id: SessionId,
    model: &str,
) -> ArgoResult<TurnResult> {
    let (registry, engine, principal) = build_dispatch_ctx(cwd)?;
    let tools = provider_schemas(&registry);

    let mut last_text = String::new();
    let mut last_tokens_in: Option<u32> = None;
    let mut last_tokens_out: Option<u32> = None;
    let mut last_assistant_id = Uuid::nil();

    for round in 0..MAX_TOOL_ROUNDS {
        let rows = store.list_messages(session_id).await?;
        let messages = rows
            .iter()
            .map(|r| message_from_row(r, session_id))
            .collect::<ArgoResult<Vec<_>>>()?;

        let req = ProviderRequest {
            model: model.to_string(),
            messages,
            tools: tools.clone(),
            system: None,
            sampling: Sampling::default(),
        };
        let resp = transport.complete(&req).await?;
        last_text = resp.text.clone().unwrap_or_default();
        last_tokens_in = resp.tokens_in;
        last_tokens_out = resp.tokens_out;

        let assistant_content = build_assistant_content(&last_text, &resp.tool_calls);
        last_assistant_id = store
            .append_message(
                session_id,
                MessageRole::Assistant,
                &assistant_content.to_string(),
            )
            .await?;

        if resp.tool_calls.is_empty() {
            return Ok(TurnResult {
                assistant_id: last_assistant_id,
                text: last_text,
                tokens_in: last_tokens_in,
                tokens_out: last_tokens_out,
            });
        }

        for call in &resp.tool_calls {
            let result = dispatch_one(&registry, &engine, &principal, cwd, call).await;
            let tool_msg = serde_json::json!([ContentBlock::ToolResult(result)]);
            store
                .append_message(session_id, MessageRole::Tool, &tool_msg.to_string())
                .await?;
        }

        if round + 1 == MAX_TOOL_ROUNDS {
            return Err(ArgoError::Provider(format!(
                "tool-dispatch loop hit MAX_TOOL_ROUNDS={MAX_TOOL_ROUNDS}; aborting"
            )));
        }
    }
    Ok(TurnResult {
        assistant_id: last_assistant_id,
        text: last_text,
        tokens_in: last_tokens_in,
        tokens_out: last_tokens_out,
    })
}

pub(crate) fn build_assistant_content(
    text: &str,
    tool_calls: &[ToolCallRequest],
) -> serde_json::Value {
    let mut blocks: Vec<ContentBlock> = Vec::new();
    if !text.is_empty() {
        blocks.push(ContentBlock::Text {
            text: text.to_string(),
        });
    }
    for tc in tool_calls {
        blocks.push(ContentBlock::ToolUse(tc.clone()));
    }
    serde_json::json!(blocks)
}

async fn dispatch_one(
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    call: &ToolCallRequest,
) -> ToolCallResult {
    let started = std::time::Instant::now();
    let mut ctx = ToolContext::new(principal.clone(), cwd.to_path_buf());
    ctx.call_id = call.id;
    match registry
        .dispatch(&call.name, call.arguments.clone(), &ctx, engine)
        .await
    {
        Ok(r) => r,
        Err(e) => argo_tool::error_text(
            call.id,
            format!("dispatch '{}' failed: {e}", call.name),
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        ),
    }
}

async fn run_turn_stream(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    session_id: SessionId,
    model: &str,
) -> ArgoResult<TurnResult> {
    let rows = store.list_messages(session_id).await?;
    let messages = rows
        .iter()
        .map(|r| message_from_row(r, session_id))
        .collect::<ArgoResult<Vec<_>>>()?;

    let req = ProviderRequest {
        model: model.to_string(),
        messages,
        tools: Vec::new(),
        system: None,
        sampling: Sampling::default(),
    };
    let (text, tokens_in, tokens_out) = run_stream(transport, &req).await?;
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
    let raw_content: Vec<ContentBlock> = serde_json::from_str(&row.content_json).map_err(|e| {
        ArgoError::Provider(format!("parse content_json: {e}: {}", row.content_json))
    })?;
    let (content, tool_calls, tool_call_id) = lift_tool_blocks(raw_content);
    let created_at = chrono::DateTime::from_timestamp_micros(row.created_at)
        .ok_or_else(|| ArgoError::Provider("created_at out of range".into()))?
        .with_timezone(&Utc);
    Ok(Message {
        id: MessageId(uuid::Uuid::from_bytes(id_bytes)),
        session_id,
        role,
        content,
        tool_calls,
        tool_call_id,
        reasoning: None,
        created_at,
        provider_meta: serde_json::Value::Null,
    })
}

/// Split a `Vec<ContentBlock>` (the on-disk form) into the
/// shape the wire layer expects: plain content blocks stay on
/// `Message.content`, `ToolUse` blocks get hoisted to
/// `Message.tool_calls`, and the first `ToolResult`'s id is
/// hoisted to `Message.tool_call_id`. Tool-result blocks remain
/// in `content` so wire layers (Anthropic `push_tool_result`)
/// can read the body.
pub(crate) fn lift_tool_blocks(
    blocks: Vec<ContentBlock>,
) -> (Vec<ContentBlock>, Vec<ToolCallRequest>, Option<ToolCallId>) {
    let mut content = Vec::with_capacity(blocks.len());
    let mut tool_calls = Vec::new();
    let mut tool_call_id: Option<ToolCallId> = None;
    for b in blocks {
        match b {
            ContentBlock::ToolUse(tc) => tool_calls.push(tc),
            ContentBlock::ToolResult(r) => {
                if tool_call_id.is_none() {
                    tool_call_id = Some(r.id);
                }
                content.push(ContentBlock::ToolResult(r));
            }
            other => content.push(other),
        }
    }
    (content, tool_calls, tool_call_id)
}
