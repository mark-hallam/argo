//! `argo audit by-kind [--since-minutes N] [--json]` — histogram
//! of audit rows grouped by `kind`. Mirrors the gateway's
//! `GET /audit/by-kind` (#570). Lives in its own file so
//! `handlers/audit.rs` stays under the 500-LOC budget.
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
                "SELECT kind, COUNT(*) FROM audit_record WHERE created_at >= ? \
                 GROUP BY kind ORDER BY kind",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record GROUP BY kind ORDER BY kind",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("audit_by_kind: {e}")))?;
    if json {
        let mut by_kind = serde_json::Map::new();
        for (k, n) in &rows {
            by_kind.insert(k.clone(), serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_kind": serde_json::Value::Object(by_kind),
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no audit rows in the last {m} minutes)"),
            None => println!("(no audit rows)"),
        }
        return Ok(());
    }
    let mut sorted = rows.clone();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
    let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
    println!(
        "audit by kind ({total} rows across {} kinds):",
        sorted.len()
    );
    for (k, n) in &sorted {
        println!("  {n:>8}  {k}");
    }
    Ok(())
}
