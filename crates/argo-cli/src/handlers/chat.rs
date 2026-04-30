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
use super::chat_helpers::{
    build_assistant_content, emit_event, message_from_row, price_for, record_turn_spend,
    RunEventSink, SpendCtx,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::CapabilityEngine;
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, ProviderRequest, Sampling,
    ToolSchema as ProviderToolSchema, Transport,
};
use argo_store::SessionStore;
use argo_tool::{builtin_tools, ToolContext, ToolRegistry};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, MessageRole, Principal, SessionId, ToolCallRequest,
    ToolCallResult,
};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Default cap on tool-dispatch rounds within a single `run_turn`
/// when callers don't pass an explicit `max_rounds`. Generous
/// enough for legitimate planning chains; small enough that an
/// accidental loop fails fast.
pub const MAX_TOOL_ROUNDS: usize = 8;

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

pub async fn run(
    raw: &str,
    user: Option<&str>,
    stream: bool,
    interactive: bool,
    max_tool_rounds: Option<usize>,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;

    let rounds = max_tool_rounds.unwrap_or(MAX_TOOL_ROUNDS);
    if interactive {
        return run_interactive(&store, &cwd, SessionId(id), user, stream, rounds).await;
    }

    eprintln!("argo chat: → session {id} (stream={stream})");
    let result = run_turn(&store, &cwd, SessionId(id), user, stream, rounds).await?;
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
    max_rounds: usize,
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
        let result = run_turn(store, cwd, session_id, Some(text), stream, max_rounds).await?;
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
        let result = run_turn(store, cwd, session_id, Some(&line), stream, max_rounds).await?;
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
    max_rounds: usize,
) -> ArgoResult<TurnResult> {
    run_turn_with_events(store, cwd, session_id, user, stream, max_rounds, None).await
}

/// Like `run_turn`, but with an optional `events` sink. The gateway
/// streaming endpoint passes a real `UnboundedSender`; the CLI +
/// non-streaming gateway pass `None`. Each event mirrors the audit
/// emission that just landed.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_events(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
    events: Option<&RunEventSink>,
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
    let pricing = price_for(&manifest, &session.model);
    let provider_id = manifest.provider.id.clone();
    let transport: Box<dyn Transport> = build_transport(manifest).await?;

    let audit = AuditSink::new(store.clone());
    let turn_start = serde_json::json!({"provider": session.provider, "model": session.model});
    audit
        .write(Some(session_id), AuditKind::TurnStart, turn_start.clone())
        .await?;
    emit_event(events, "TurnStart", turn_start);
    let spend_ctx = SpendCtx {
        tenant_id: session.tenant_id,
        provider: provider_id.as_str(),
        model: session.model.as_str(),
        pricing,
    };
    let result = run_turn_with_tools(
        store,
        transport.as_ref(),
        cwd,
        session_id,
        &session.model,
        &audit,
        &spend_ctx,
        max_rounds,
        stream,
        events,
    )
    .await;
    let kind = if result.is_ok() {
        AuditKind::TurnEnd
    } else {
        AuditKind::Halt
    };
    let kind_label = if result.is_ok() { "TurnEnd" } else { "Halt" };
    let _ = audit
        .write(Some(session_id), kind, serde_json::json!({}))
        .await;
    emit_event(events, kind_label, serde_json::json!({}));
    result
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

#[allow(clippy::too_many_arguments)]
async fn run_turn_with_tools(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    cwd: &Path,
    session_id: SessionId,
    model: &str,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    max_rounds: usize,
    stream: bool,
    events: Option<&RunEventSink>,
) -> ArgoResult<TurnResult> {
    let (registry, engine, principal) = build_dispatch_ctx(cwd)?;
    let tools = provider_schemas(&registry);

    let mut last_text = String::new();
    let mut last_tokens_in: Option<u32> = None;
    let mut last_tokens_out: Option<u32> = None;
    let mut last_assistant_id = Uuid::nil();

    for round in 0..max_rounds {
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
        let resp = if stream {
            super::chat_helpers::stream_one_round(transport, &req).await?
        } else {
            let r = transport.complete(&req).await?;
            super::chat_helpers::RoundResp {
                text: r.text.unwrap_or_default(),
                tool_calls: r.tool_calls,
                tokens_in: r.tokens_in,
                tokens_out: r.tokens_out,
            }
        };
        last_text = resp.text.clone();
        last_tokens_in = resp.tokens_in;
        last_tokens_out = resp.tokens_out;

        let assistant_content = build_assistant_content(&resp.text, &resp.tool_calls);
        last_assistant_id = store
            .append_message(
                session_id,
                MessageRole::Assistant,
                &assistant_content.to_string(),
            )
            .await?;
        let assistant_payload = serde_json::json!({
            "message_id": last_assistant_id,
            "round": round,
            "tool_calls": resp.tool_calls.len(),
            "tokens_in": last_tokens_in,
            "tokens_out": last_tokens_out,
        });
        audit
            .write(
                Some(session_id),
                AuditKind::AssistantMessage,
                assistant_payload.clone(),
            )
            .await?;
        emit_event(events, "AssistantMessage", assistant_payload);
        record_turn_spend(store, session_id, spend, last_tokens_in, last_tokens_out).await?;

        if resp.tool_calls.is_empty() {
            return Ok(TurnResult {
                assistant_id: last_assistant_id,
                text: last_text,
                tokens_in: last_tokens_in,
                tokens_out: last_tokens_out,
            });
        }

        for call in &resp.tool_calls {
            let call_payload = serde_json::json!({"tool": call.name, "call_id": call.id});
            audit
                .write(Some(session_id), AuditKind::ToolCall, call_payload.clone())
                .await?;
            emit_event(events, "ToolCall", call_payload);
            let result = dispatch_one(&registry, &engine, &principal, cwd, call).await;
            let result_payload = serde_json::json!({
                "tool": call.name,
                "call_id": call.id,
                "is_error": result.is_error,
                "duration_ms": result.duration_ms,
            });
            audit
                .write(
                    Some(session_id),
                    AuditKind::ToolResult,
                    result_payload.clone(),
                )
                .await?;
            emit_event(events, "ToolResult", result_payload);
            let tool_msg = serde_json::json!([ContentBlock::ToolResult(result)]);
            store
                .append_message(session_id, MessageRole::Tool, &tool_msg.to_string())
                .await?;
        }

        if round + 1 == max_rounds {
            return Err(ArgoError::Provider(format!(
                "tool-dispatch loop hit max_rounds={max_rounds}; aborting"
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


/// Resolve a `ProviderManifest` to a live `Transport`. Anthropic /
/// OpenAI families read the API key from `manifest.auth.api_key_env`;
/// Bedrock loads AWS credentials via `aws_config::load_from_env()`
/// (honours the standard `AWS_*` env vars + `~/.aws` profile).
async fn build_transport(manifest: ProviderManifest) -> ArgoResult<Box<dyn Transport>> {
    if matches!(manifest.provider.family, ProviderFamily::Bedrock) {
        let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        return Ok(Box::new(BedrockTransport::from_manifest(manifest, &cfg)?));
    }
    let api_key_env = manifest.auth.api_key_env.clone();
    let api_key = std::env::var(&api_key_env)
        .map_err(|_| ArgoError::Provider(format!("{api_key_env} not set")))?;
    let t: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Anthropic => {
            Box::new(AnthropicTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiCompatible => {
            Box::new(OpenAiCompatTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, &api_key)?)
        }
        // Already handled above; reachable only if a new family is
        // added without wiring it through. Fail loud.
        ProviderFamily::Bedrock => unreachable!("bedrock handled above"),
    };
    Ok(t)
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


