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
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;

    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
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
        .create_session(tenant, ws, &target_model, &target_provider, Some(&title))
        .await?;

    let mut final_text = String::new();
    for text in &user_texts {
        let result = crate::handlers::chat::run_turn(
            &store,
            &cwd,
            new_id,
            Some(text),
            false,
            crate::handlers::chat::MAX_TOOL_ROUNDS,
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

/// Top-level handler for `argo session replay`. Calls
/// `replay_core` and prints a human-readable summary.
pub(crate) async fn run(
    raw: &str,
    model: Option<&str>,
    provider: Option<&str>,
) -> ArgoResult<()> {
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
