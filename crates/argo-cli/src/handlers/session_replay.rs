//! `argo session replay <id> [--model <m>] [--provider <p>]` — re-
//! run the user prompts from a recorded session against a (possibly
//! different) model + provider. Extracted from session.rs to keep
//! that file under the 500-LOC budget.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

/// Outcome of one replay run, suitable for both CLI summaries and
/// JSON serialization (e.g. the gateway's HTTP response).
pub(crate) struct ReplayResult {
    pub source_id: SessionId,
    pub new_id: SessionId,
    pub provider: String,
    pub model: String,
    pub user_message_count: usize,
    pub final_text: String,
}

/// Pure replay core — does the work without any printing. Used by
/// both the CLI handler (which prints progress) and the gateway
/// handler (which serializes the result as JSON).
pub(crate) async fn replay_core(
    raw: &str,
    model: Option<&str>,
    provider: Option<&str>,
) -> ArgoResult<ReplayResult> {
    replay_core_with_events(raw, model, provider, None).await
}

/// Like `replay_core`, but pipes per-turn `RunEvent`s through an
/// `mpsc` sender. Used by the gateway's `replay/stream` endpoint.
pub(crate) async fn replay_core_with_events(
    raw: &str,
    model: Option<&str>,
    provider: Option<&str>,
    events: Option<&argo_agent_runner::RunEventSink>,
) -> ArgoResult<ReplayResult> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;

    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))?;
    let source = store
        .get_session(SessionId(id))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw}")))?;
    let target_model = model.unwrap_or(&source.model).to_string();
    let target_provider = provider.unwrap_or(&source.provider).to_string();

    let messages = store.list_messages(SessionId(id)).await?;
    let user_texts: Vec<String> = messages
        .iter()
        .filter(|m| m.role == "user")
        .filter_map(|m| extract_first_text(&m.content_json))
        .collect();
    if user_texts.is_empty() {
        return Err(ArgoError::Provider(format!(
            "session {raw} has no user-role text messages to replay"
        )));
    }

    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, ws) = store.ensure_workspace(name, &root).await?;
    let title = source
        .title
        .as_deref()
        .map(|t| format!("replay of {t}"))
        .unwrap_or_else(|| format!("replay of {}", source.id.0));
    let new_id = store
        .create_session(tenant, ws, &target_model, &target_provider, Some(&title), None)
        .await?;

    let mut final_text = String::new();
    for text in &user_texts {
        let result = crate::handlers::chat::run_turn_with_events(
            &store,
            &cwd,
            new_id,
            Some(text),
            false,
            crate::handlers::chat::MAX_TOOL_ROUNDS,
            events,
        )
        .await?;
        final_text = result.text;
    }
    Ok(ReplayResult {
        source_id: source.id,
        new_id,
        provider: target_provider,
        model: target_model,
        user_message_count: user_texts.len(),
        final_text,
    })
}

/// Top-level handler for `argo session replay`. With `stream`,
/// pipes per-turn `RunEvent`s to stdout as NDJSON (mirroring the
/// gateway's `replay/stream` chunked response). Otherwise calls
/// `replay_core` and prints a human-readable summary.
pub(crate) async fn run(
    raw: &str,
    model: Option<&str>,
    provider: Option<&str>,
    stream: bool,
) -> ArgoResult<()> {
    if stream {
        return run_streaming(raw, model, provider).await;
    }
    let result = replay_core(raw, model, provider).await?;
    println!(
        "session replay: {} → new session {} (provider={}, model={})",
        result.source_id.0, result.new_id.0, result.provider, result.model
    );
    println!("replayed {} user message(s)", result.user_message_count);
    let snippet_text = snippet(&result.final_text);
    if !snippet_text.is_empty() {
        println!("final reply: {snippet_text}");
    }
    println!(
        "\ninspect with `argo session show {}`.",
        result.new_id.0
    );
    Ok(())
}

/// Stream a replay's per-turn `RunEvent`s to stdout as NDJSON.
/// Each event lands as one JSON object on its own line; the
/// final line is `{"event":"done", source_id, new_id,
/// user_message_count}` (or `{"event":"error", "message": …}`
/// if the replay fails). Mirrors the gateway's
/// `POST /sessions/<id>/replay/stream` chunked response.
async fn run_streaming(
    raw: &str,
    model: Option<&str>,
    provider: Option<&str>,
) -> ArgoResult<()> {
    use std::io::Write;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
    let raw_owned = raw.to_string();
    let model_owned = model.map(String::from);
    let provider_owned = provider.map(String::from);
    let task = tokio::spawn(async move {
        replay_core_with_events(
            &raw_owned,
            model_owned.as_deref(),
            provider_owned.as_deref(),
            Some(&tx),
        )
        .await
    });
    let mut stdout = std::io::stdout().lock();
    while let Some(payload) = rx.recv().await {
        let line = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    }
    let summary = match task.await {
        Ok(Ok(r)) => serde_json::json!({
            "event": "done",
            "source_id": r.source_id.0.to_string(),
            "new_id": r.new_id.0.to_string(),
            "user_message_count": r.user_message_count,
        }),
        Ok(Err(e)) => serde_json::json!({"event":"error", "message": e.to_string()}),
        Err(e) => serde_json::json!({"event":"error", "message": format!("task: {e}")}),
    };
    if let Ok(s) = serde_json::to_string(&summary) {
        let _ = writeln!(stdout, "{s}");
        let _ = stdout.flush();
    }
    Ok(())
}

fn extract_first_text(content_json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(content_json).ok()?;
    parsed
        .as_array()?
        .iter()
        .find_map(|b| b.get("text").and_then(|t| t.as_str()).map(str::to_string))
}

fn snippet(s: &str) -> String {
    s.lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(80)
        .collect::<String>()
}
