//! `argo memory` — list / view / revert / lock / unlock memory entries.
use crate::cli::MemoryCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock};
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
        MemoryCmd::Append { file, body } => append(&memory, workspace, &file, &body).await,
        MemoryCmd::Revert { entry } => revert(&memory, &entry).await,
        MemoryCmd::Forget { entry } => revert(&memory, &entry).await,
        MemoryCmd::Lock { entry } => set_lock(&memory, &entry, Lock::UserProtected).await,
        MemoryCmd::Unlock { entry } => set_lock(&memory, &entry, Lock::None).await,
        other => {
            println!(
                "argo memory: '{other:?}' requires the daemon's capability bindings (separate PR)."
            );
            Ok(())
        }
    }
}

async fn append(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    file: &str,
    body: &str,
) -> ArgoResult<()> {
    let mf = match file {
        "memory" => MemoryFile::Memory,
        "user" => MemoryFile::User,
        "soul" => MemoryFile::Soul,
        "agents" => MemoryFile::Agents,
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown memory file '{other}': use one of memory|user|soul|agents"
            )));
        }
    };
    let entry = memory.append(workspace, mf, body.to_string(), None).await?;
    println!(
        "appended {} to {}.md ({} bytes)",
        entry.id,
        file,
        body.len()
    );
    Ok(())
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
    let id = parse_uuid(raw)?;
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

async fn revert(memory: &MemoryStore, raw: &str) -> ArgoResult<()> {
    let id = parse_uuid(raw)?;
    let new = memory.revert(id, PrincipalOverride::Force).await?;
    println!(
        "reverted: new active entry {} (file={:?})",
        new.id, new.file
    );
    Ok(())
}

async fn set_lock(memory: &MemoryStore, raw: &str, lock: Lock) -> ArgoResult<()> {
    let id = parse_uuid(raw)?;
    memory.set_lock(id, lock, PrincipalOverride::Force).await?;
    println!("memory entry {id}: lock = {lock:?}");
    Ok(())
}

fn parse_uuid(raw: &str) -> ArgoResult<uuid::Uuid> {
    uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid uuid '{raw}': {e}")))
}
