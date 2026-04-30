//! `argo session replay <id> [--model <m>] [--provider <p>]` — re-
//! run the user prompts from a recorded session against a (possibly
//! different) model + provider. Extracted from session.rs to keep
//! that file under the 500-LOC budget.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

/// Top-level handler for `argo session replay`.
///
/// Creates a fresh session row and pumps each user-role message
/// from the source through `chat::run_turn` against the target
/// `(provider, model)`. Assistant / tool messages from the source
/// are skipped — the new model regenerates them.
pub(crate) async fn run(
    raw: &str,
    model: Option<&str>,
    provider: Option<&str>,
) -> ArgoResult<()> {
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
    let target_model = model.unwrap_or(&source.model);
    let target_provider = provider.unwrap_or(&source.provider);

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
        .create_session(
            tenant,
            ws,
            target_model,
            target_provider,
            Some(title.as_str()),
        )
        .await?;

    println!(
        "session replay: {} → new session {} (provider={target_provider}, model={target_model})",
        source.id.0, new_id.0
    );
    println!("replaying {} user message(s)…\n", user_texts.len());
    for (i, text) in user_texts.iter().enumerate() {
        println!("[turn {}] > {}", i + 1, snippet(text));
        let result = crate::handlers::chat::run_turn(
            &store,
            &cwd,
            new_id,
            Some(text),
            false,
            crate::handlers::chat::MAX_TOOL_ROUNDS,
        )
        .await?;
        let s = snippet(&result.text);
        if !s.is_empty() {
            println!("[turn {}] < {s}", i + 1);
        }
    }
    println!(
        "\nreplay complete. inspect with `argo session show {}`.",
        new_id.0
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
