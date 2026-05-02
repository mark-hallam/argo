//! `argo db` — schema migration ops.
use crate::cli::DbCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::ArgoResult;

pub async fn run(cmd: DbCmd) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| argo_types::ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    match cmd {
        DbCmd::Status { json, format } => {
            let v = store.schema_version().await?;
            let stats = store.db_stats().await?;
            let meta = std::fs::metadata(&db).ok();
            let file_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let last_modified_micros = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_micros() as i64)
                .unwrap_or(0);
            let pick = if json { "json" } else { format.as_str() };
            match pick {
                "json" => {
                    let payload = serde_json::json!({
                        "schema_version": v,
                        "db_path": db.display().to_string(),
                        "file_bytes": file_bytes,
                        "last_modified_micros": last_modified_micros,
                        "rows": {
                            "sessions":          stats.sessions,
                            "messages":          stats.messages,
                            "tool_calls":        stats.tool_calls,
                            "audit_records":     stats.audit_rows,
                            "routing_decisions": stats.routing_decisions,
                            "judge_records":     stats.judge_records,
                            "memory_entries":    stats.memory_entries,
                            "skills":            stats.skills,
                            "forge_runs":        stats.forge_runs,
                            "spend_ledger":      stats.spend_rows,
                        },
                    });
                    println!("{}", serde_json::to_string(&payload).unwrap_or_default());
                }
                "markdown" => {
                    println!("# Database status");
                    println!();
                    println!("- **Schema**: v{v}");
                    println!("- **Path**: `{}`", db.display());
                    println!("- **Size**: {file_bytes} bytes");
                    println!();
                    println!("## Rows per table");
                    println!();
                    println!("| table | rows |");
                    println!("| --- | ---: |");
                    println!("| `sessions` | {} |", stats.sessions);
                    println!("| `messages` | {} |", stats.messages);
                    println!("| `tool_calls` | {} |", stats.tool_calls);
                    println!("| `audit_record` | {} |", stats.audit_rows);
                    println!("| `routing_decisions` | {} |", stats.routing_decisions);
                    println!("| `judge_record` | {} |", stats.judge_records);
                    println!("| `memory_entries` | {} |", stats.memory_entries);
                    println!("| `skills` | {} |", stats.skills);
                    println!("| `forge_run` | {} |", stats.forge_runs);
                    println!("| `spend_ledger` | {} |", stats.spend_rows);
                }
                "text" => {
                    println!("schema_version: {v}");
                    println!("db_path:        {}", db.display());
                    println!("file size:      {file_bytes} bytes");
                    println!("rows:");
                    println!("  sessions:           {:>8}", stats.sessions);
                    println!("  messages:           {:>8}", stats.messages);
                    println!("  tool_calls:         {:>8}", stats.tool_calls);
                    println!("  audit_records:      {:>8}", stats.audit_rows);
                    println!("  routing_decisions:  {:>8}", stats.routing_decisions);
                    println!("  judge_records:      {:>8}", stats.judge_records);
                    println!("  memory_entries:     {:>8}", stats.memory_entries);
                    println!("  skills:             {:>8}", stats.skills);
                    println!("  forge_runs:         {:>8}", stats.forge_runs);
                    println!("  spend_ledger:       {:>8}", stats.spend_rows);
                }
                other => {
                    return Err(argo_types::ArgoError::Provider(format!(
                        "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
                    )));
                }
            }
        }
        DbCmd::MigrateUp => {
            let before = store.schema_version().await?;
            store.migrate().await?;
            let after = store.schema_version().await?;
            println!("migrated: {before} -> {after}");
        }
        DbCmd::MigrateDown { to } => {
            let before = store.schema_version().await?;
            store.migrate_down_to(to).await?;
            let after = store.schema_version().await?;
            println!("rolled back: {before} -> {after}");
        }
        DbCmd::Prune {
            older_than_days,
            force,
        } => {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days);
            let cutoff_micros = cutoff.timestamp_micros();
            if !force {
                let count = store.count_sessions_before(cutoff_micros).await?;
                println!(
                    "dry run: would delete {count} session(s) older than {} ({}d ago).",
                    cutoff, older_than_days
                );
                println!("re-run with --force to actually drop them.");
            } else {
                let n = store.prune_sessions_before(cutoff_micros).await?;
                println!(
                    "pruned {n} session(s) older than {} ({}d ago); cascaded children dropped.",
                    cutoff, older_than_days
                );
                println!("(run `argo db vacuum` afterwards to reclaim file space)");
            }
        }
        DbCmd::Vacuum => {
            let before = std::fs::metadata(&db).map(|m| m.len()).unwrap_or(0);
            store.vacuum().await?;
            let after = std::fs::metadata(&db).map(|m| m.len()).unwrap_or(0);
            let saved = before.saturating_sub(after);
            println!(
                "vacuum: {} -> {} bytes ({} reclaimed)",
                before, after, saved
            );
        }
        DbCmd::Version { json } => {
            let v = store.schema_version().await?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"version": v}))
                        .unwrap_or_default()
                );
            } else {
                println!("{v}");
            }
        }
        DbCmd::Integrity { json } => {
            let pool = store.pool();
            let rows: Vec<(String,)> = sqlx::query_as("PRAGMA integrity_check")
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    argo_types::ArgoError::Provider(format!("integrity_check: {e}"))
                })?;
            let lines: Vec<String> = rows.into_iter().map(|(s,)| s).collect();
            let ok = lines.len() == 1 && lines.first().is_some_and(|s| s == "ok");
            if json {
                let payload = serde_json::json!({
                    "ok": ok,
                    "check": lines,
                });
                println!("{}", serde_json::to_string(&payload).unwrap_or_default());
            } else if ok {
                println!("ok");
            } else {
                println!("FAIL");
                for l in &lines {
                    println!("  {l}");
                }
            }
        }
    }
    Ok(())
}
