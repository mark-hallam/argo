//! `argo memory` — list / view memory entries.
use crate::cli::MemoryCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run(cmd: MemoryCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    let workspace = store
        .first_workspace()
        .await?
        .ok_or_else(|| ArgoError::Provider("no workspace registered yet".into()))?;
    let memory = MemoryStore::new(Arc::clone(&store));
    match cmd {
        MemoryCmd::List => list(&memory, workspace).await,
        MemoryCmd::View { entry } => view(&memory, &entry).await,
        other => {
            println!(
                "argo memory: '{other:?}' requires the daemon's capability bindings (separate PR)."
            );
            Ok(())
        }
    }
}

async fn list(memory: &MemoryStore, workspace: argo_types::WorkspaceId) -> ArgoResult<()> {
    let mut total = 0_usize;
    for file in [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ] {
        let entries = memory.list_active(workspace, file).await?;
        if entries.is_empty() {
            continue;
        }
        total += entries.len();
        println!("# {}.md", file.filename().trim_end_matches(".md"));
        for e in entries {
            let head = e
                .body
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();
            println!("  {} ({} bytes)  {head}", e.id, e.body.len());
        }
    }
    if total == 0 {
        println!("(no active memory entries)");
    }
    Ok(())
}

async fn view(memory: &MemoryStore, raw: &str) -> ArgoResult<()> {
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid uuid '{raw}': {e}")))?;
    let entry = memory.get(id).await?;
    println!(
        "entry {} ({:?}, lock={:?})",
        entry.id, entry.file, entry.lock
    );
    println!(
        "learned_in: {}",
        entry
            .learned_in
            .map(|s| s.0.to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );
    println!("\n---\n");
    println!("{}", entry.body);
    Ok(())
}
