//! `argo audit tool-errors [--since-minutes M] [--json]` —
//! histogram of `tool_result` audit rows where `is_error=true`,
//! grouped by tool name. Mirrors the gateway's
//! `GET /audit/tool-errors` (#617). Operational signal for
//! spotting broken tool integrations at a glance — pairs with
//! `argo audit by-tool` (#614) which counts ALL invocations.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(since_minutes: Option<i64>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
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
                 WHERE kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
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
                 WHERE kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("audit_tool_errors: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(t, n)| serde_json::json!({"tool": t, "count": n}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"tool_errors": arr}))
                .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no tool_result errors in the last {m} minutes — clean run)"),
            None => println!("(no tool_result errors recorded — clean run)"),
        }
        return Ok(());
    }
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    println!(
        "tool errors ({total} failures across {} tools):",
        rows.len()
    );
    for (t, n) in &rows {
        println!("  {n:>8}  {t}");
    }
    Ok(())
}
