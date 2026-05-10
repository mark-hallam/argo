//! `argo memory list` and `argo memory count` — read-side queries
//! over the active memory entries.

use super::memory::parse_memory_file;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, Lock};

/// `argo memory count [--file] [--json]` — print just the count
/// of active memory entries (across all four files by default,
/// or one file with --file). Default: one int per line; --json
/// emits `{count, file}`.
pub(crate) async fn count(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    only: Option<&str>,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let only_file = match only {
        None => None,
        Some(s) => Some(parse_memory_file(s)?),
    };
    let cutoff = since_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let mut total = 0_usize;
    let all = [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ];
    for file in all {
        if let Some(f) = only_file {
            if file != f {
                continue;
            }
        }
        let entries = memory.list_active(workspace, file).await?;
        let n = match cutoff {
            Some(c) => entries.iter().filter(|e| e.learned_at >= c).count(),
            None => entries.len(),
        };
        total += n;
    }
    if json {
        let payload = serde_json::json!({
            "count": total,
            "file": only.unwrap_or("all"),
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("{total}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn list(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    json: bool,
    format: &str,
    only: Option<&str>,
    sort_by: &str,
    limit: Option<u32>,
    since_minutes: Option<i64>,
    lock: Option<&str>,
    body_contains: Option<&str>,
    vertical: Option<&str>,
) -> ArgoResult<()> {
    let only_file = match only {
        None => None,
        Some(s) => Some(parse_memory_file(s)?),
    };
    let lock_filter = match lock {
        None => None,
        Some("none") => Some(Lock::None),
        Some("user-protected") => Some(Lock::UserProtected),
        Some("content-locked") => Some(Lock::ContentLocked),
        Some(other) => {
            return Err(ArgoError::Provider(format!(
                "unknown --lock '{other}' — use none|user-protected|content-locked"
            )));
        }
    };
    let cutoff = since_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let needle = body_contains.map(str::to_lowercase);
    let all = [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ];
    // Collect across files first so we can sort uniformly.
    // #686 — Rule 7: when --vertical is set, use the
    // vertical-namespaced lookup which excludes legacy
    // workspace-global rows + other verticals' rows. Without
    // --vertical, the legacy list-all behaviour is preserved.
    let mut all_entries: Vec<argo_memory::MemoryEntry> = Vec::new();
    for file in all {
        if let Some(f) = only_file {
            if file != f {
                continue;
            }
        }
        let entries = match vertical {
            Some(vid) => memory.list_for_vertical(workspace, file, vid).await?,
            None => memory.list_active(workspace, file).await?,
        };
        for e in entries {
            if let Some(c) = cutoff {
                if e.learned_at < c {
                    continue;
                }
            }
            if let Some(want) = lock_filter {
                if e.lock != want {
                    continue;
                }
            }
            if let Some(n) = needle.as_deref() {
                if !e.body.to_lowercase().contains(n) {
                    continue;
                }
            }
            all_entries.push(e);
        }
    }
    match sort_by {
        "learned-at" => {
            all_entries.sort_by_key(|e| std::cmp::Reverse(e.learned_at));
        }
        "size" => {
            all_entries.sort_by_key(|e| std::cmp::Reverse(e.body.len()));
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --sort-by '{other}' — use learned-at|size"
            )));
        }
    }
    if let Some(n) = limit {
        all_entries.truncate(n as usize);
    }
    let total = all_entries.len();
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            for e in &all_entries {
                let row = serde_json::json!({
                    "id": e.id.to_string(),
                    "file": format!("{:?}", e.file),
                    "bytes": e.body.len(),
                    "lock": format!("{:?}", e.lock),
                    "hash": hex::encode(e.hash),
                    "learned_in": e.learned_in.map(|s| s.0.to_string()),
                    "learned_at": e.learned_at.to_rfc3339(),
                    "vertical_id": e.vertical_id,
                });
                let serialized = serde_json::to_string(&row)
                    .map_err(|err| ArgoError::Provider(format!("serialize memory: {err}")))?;
                println!("{serialized}");
            }
            return Ok(());
        }
        "markdown" => {
            println!(
                "# Memory entries ({} match{})",
                total,
                if total == 1 { "" } else { "es" }
            );
            println!();
            if total == 0 {
                return Ok(());
            }
            println!("| file | id | bytes | lock | snippet |");
            println!("| --- | --- | ---: | --- | --- |");
            for e in &all_entries {
                let head = e
                    .body
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect::<String>()
                    .replace('|', "\\|");
                println!(
                    "| {:?} | `{}` | {} | {:?} | {head} |",
                    e.file,
                    e.id,
                    e.body.len(),
                    e.lock,
                );
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if total == 0 {
        println!("(no active memory entries)");
        return Ok(());
    }
    for e in all_entries {
        let head = e
            .body
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(60)
            .collect::<String>();
        println!(
            "  [{:?}] {} ({} bytes)  {head}",
            e.file,
            e.id,
            e.body.len()
        );
    }
    Ok(())
}
