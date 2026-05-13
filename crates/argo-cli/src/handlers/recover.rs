//! `argo recover <session-id>` — LLM-assisted post-mortem.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::recover_prompt::{build_prompt, chain_hash_for, find_failing_step, RecoverySuggestions};
use crate::home::{db_path, ensure_dir, resolve_home};

const SCHEMA_SQL: &str = include_str!("../../../argo-store/migrations/28__recovery_cache.sql");

/// Pluggable Anthropic caller — same shape as
/// `argo_genesis::AnthropicCaller`. Exposed for tests.
#[async_trait]
pub trait AnthropicCaller: Send + Sync {
    /// Send the prompt; return the raw model response.
    async fn call(&self, prompt: String) -> ArgoResult<String>;
}

pub async fn run(session_id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    ensure_recovery_schema(&store).await?;

    let session_uuid = Uuid::parse_str(session_id)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{session_id}': {e}")))?;
    let sid = SessionId(session_uuid);
    let rows = store.list_audit_for_session(sid).await?;
    if rows.is_empty() {
        return Err(ArgoError::Provider(format!(
            "session '{session_id}' has no audit rows"
        )));
    }

    let chain_hash = chain_hash_for(&rows);
    let cached = read_cache(&store, session_id, &chain_hash).await?;
    let (suggestions, from_cache) = if let Some(s) = cached {
        (s, true)
    } else {
        let caller = build_caller()?;
        let prompt = build_prompt(None, None, &rows);
        let raw = caller.call(prompt).await?;
        let parsed: RecoverySuggestions = serde_json::from_str(&raw)
            .map_err(|e| ArgoError::Provider(format!("parse llm response: {e}")))?;
        parsed
            .validate()
            .map_err(|e| ArgoError::Schema(format!("invalid llm response: {e}")))?;
        let failing_kind = find_failing_step(&rows)
            .map(|r| r.kind.clone())
            .unwrap_or_default();
        write_cache(&store, session_id, &chain_hash, &parsed, &failing_kind).await?;
        (parsed, false)
    };

    if json {
        let envelope = serde_json::json!({
            "session_id": session_id,
            "chain_hash": chain_hash,
            "from_cache": from_cache,
            "suggestions": suggestions,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("recover: session {session_id}");
        if from_cache {
            println!("  (from cache, chain_hash={chain_hash})");
        }
        println!("\nlikely cause:\n  {}", suggestions.likely_cause);
        println!("\nranked fixes:");
        let mut sorted = suggestions.fixes.clone();
        sorted.sort_by_key(|f| f.rank);
        for fix in &sorted {
            println!("  [{}] {}", fix.rank, fix.title);
            println!("      command: {}", fix.command);
            println!("      why: {}", fix.rationale);
        }
        println!("\n(advisory only — operator decides whether to apply.)");
    }
    Ok(())
}

async fn ensure_recovery_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("recovery schema: {e}")))?;
    }
    Ok(())
}

async fn read_cache(
    store: &SessionStore,
    session_id: &str,
    chain_hash: &str,
) -> ArgoResult<Option<RecoverySuggestions>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT suggestions_json FROM recovery_cache \
         WHERE session_id = ? AND chain_hash = ?",
    )
    .bind(session_id)
    .bind(chain_hash)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("read cache: {e}")))?;
    let Some((json,)) = row else {
        return Ok(None);
    };
    let parsed: RecoverySuggestions = serde_json::from_str(&json)
        .map_err(|e| ArgoError::Provider(format!("decode cached suggestions: {e}")))?;
    Ok(Some(parsed))
}

async fn write_cache(
    store: &SessionStore,
    session_id: &str,
    chain_hash: &str,
    suggestions: &RecoverySuggestions,
    failure_kind: &str,
) -> ArgoResult<()> {
    let json = serde_json::to_string(suggestions)
        .map_err(|e| ArgoError::Provider(format!("encode suggestions: {e}")))?;
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT OR REPLACE INTO recovery_cache \
         (session_id, chain_hash, suggestions_json, failure_kind, cached_at_micros) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(chain_hash)
    .bind(json)
    .bind(failure_kind)
    .bind(now)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("write cache: {e}")))?;
    Ok(())
}

fn build_caller() -> ArgoResult<Box<dyn AnthropicCaller>> {
    let key = argo_config::cred::get("ANTHROPIC_API_KEY").map_err(|_| {
        ArgoError::Provider(
            "ANTHROPIC_API_KEY unset; cannot run recover (cache hits work without the key)".into(),
        )
    })?;
    Ok(Box::new(EnvAnthropicCaller { api_key: key }))
}

struct EnvAnthropicCaller {
    api_key: String,
}

#[async_trait]
impl AnthropicCaller for EnvAnthropicCaller {
    async fn call(&self, prompt: String) -> ArgoResult<String> {
        let body = serde_json::json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 2048,
            "messages": [{"role": "user", "content": prompt}],
        });
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("send: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!("http {status}: {txt}")));
        }
        let parsed: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("decode: {e}")))?;
        let text = parsed
            .pointer("/content/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArgoError::Provider("response missing /content/0/text".into()))?
            .to_string();
        Ok(text)
    }
}
