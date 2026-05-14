//! `argo audit by-tool [--since-minutes M] [--json]` — histogram
//! of `tool_call` + `tool_result` audit rows grouped by tool name
//! (extracted from `payload_json`'s `$.tool` field via SQLite's
//! `json_extract`). Mirrors the gateway's `GET /audit/by-tool`
//! (#613). Pairs with `argo audit by-kind` (#570) — by-tool gives
//! the next level of detail under tool calls.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(since_minutes: Option<i64>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let rows: Vec<(String, i64)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("audit_by_tool: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(t, n)| serde_json::json!({"tool": t, "count": n}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"by_tool": arr})).unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no tool_call/tool_result audit rows in the last {m} minutes)"),
            None => println!("(no tool_call/tool_result audit rows)"),
        }
        return Ok(());
    }
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    println!(
        "audit by tool ({total} rows across {} tools):",
        rows.len()
    );
    for (t, n) in &rows {
        println!("  {n:>8}  {t}");
    }
    Ok(())
}
