//! `argo forge runs by-skill [--since-minutes N] [--json]` —
//! histogram of forge_run rows grouped by skill_name. Mirrors
//! the gateway's `GET /forge/runs/by-skill` (#598).
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
                "SELECT skill_name, COUNT(*) FROM forge_run WHERE started_at >= ? \
                 GROUP BY skill_name ORDER BY skill_name",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT skill_name, COUNT(*) FROM forge_run \
                 GROUP BY skill_name ORDER BY skill_name",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("forge_runs_by_skill: {e}")))?;
    if json {
        let mut by_skill = serde_json::Map::new();
        for (s, n) in &rows {
            by_skill.insert(s.clone(), serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_skill": serde_json::Value::Object(by_skill),
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no forge runs in the last {m} minutes)"),
            None => println!("(no forge runs)"),
        }
        return Ok(());
    }
    let mut sorted = rows.clone();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
    let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
    println!(
        "forge runs by skill ({total} runs across {} skills):",
        sorted.len()
    );
    for (s, n) in &sorted {
        println!("  {n:>8}  {s}");
    }
    Ok(())
}
