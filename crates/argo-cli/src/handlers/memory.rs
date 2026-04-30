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
        MemoryCmd::List { json, file } => list(&memory, workspace, json, file.as_deref()).await,
        MemoryCmd::View { entry, json } => view(&memory, &entry, json).await,
        MemoryCmd::Append { file, body } => append(&memory, workspace, &file, &body).await,
        MemoryCmd::Verify { entry } => verify_hash(&memory, &entry).await,
        MemoryCmd::Edit { entry } => edit(&memory, &entry).await,
        MemoryCmd::Revert { entry } => revert(&memory, &entry).await,
        MemoryCmd::Forget { entry } => revert(&memory, &entry).await,
        MemoryCmd::Lock { entry } => set_lock(&memory, &entry, Lock::UserProtected).await,
        MemoryCmd::Unlock { entry } => set_lock(&memory, &entry, Lock::None).await,
    }
}

async fn append(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    file: &str,
    body: &str,
) -> ArgoResult<()> {
    let mf = parse_memory_file(file)?;
    let entry = memory.append(workspace, mf, body.to_string(), None).await?;
    println!(
        "appended {} to {}.md ({} bytes)",
        entry.id,
        file,
        body.len()
    );
    Ok(())
}

async fn list(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    json: bool,
    only: Option<&str>,
) -> ArgoResult<()> {
    let only_file = match only {
        None => None,
        Some(s) => Some(parse_memory_file(s)?),
    };
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
        if json {
            for e in &entries {
                let row = serde_json::json!({
                    "id": e.id.to_string(),
                    "file": format!("{:?}", e.file),
                    "bytes": e.body.len(),
                    "lock": format!("{:?}", e.lock),
                    "hash": hex::encode(e.hash),
                    "learned_in": e.learned_in.map(|s| s.0.to_string()),
                    "learned_at": e.learned_at.to_rfc3339(),
                });
                let serialized = serde_json::to_string(&row)
                    .map_err(|err| ArgoError::Provider(format!("serialize memory: {err}")))?;
                println!("{serialized}");
            }
            total += entries.len();
            continue;
        }
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
    if !json && total == 0 {
        println!("(no active memory entries)");
    }
    Ok(())
}

async fn view(memory: &MemoryStore, raw: &str, json: bool) -> ArgoResult<()> {
    let id = parse_uuid(raw)?;
    let entry = memory.get(id).await?;
    if json {
        let row = serde_json::json!({
            "id": entry.id.to_string(),
            "file": format!("{:?}", entry.file),
            "lock": format!("{:?}", entry.lock),
            "hash": hex::encode(entry.hash),
            "learned_in": entry.learned_in.map(|s| s.0.to_string()),
            "learned_at": entry.learned_at.to_rfc3339(),
            "body": entry.body,
        });
        let serialized = serde_json::to_string(&row)
            .map_err(|e| ArgoError::Provider(format!("serialize entry: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
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

async fn verify_hash(memory: &MemoryStore, raw: &str) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let id = parse_uuid(raw)?;
    let entry = memory.get(id).await?;
    let mut h = Sha256::new();
    h.update(entry.body.as_bytes());
    let computed: [u8; 32] = h.finalize().into();
    let stored_hex = hex::encode(entry.hash);
    let computed_hex = hex::encode(computed);
    if computed == entry.hash {
        println!(
            "memory entry {id} ({:?}) — hash ok ({computed_hex})",
            entry.file
        );
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "memory entry {id} ({:?}) — hash MISMATCH\n  stored:   {stored_hex}\n  computed: {computed_hex}",
            entry.file
        )))
    }
}

async fn edit(memory: &MemoryStore, raw: &str) -> ArgoResult<()> {
    let id = parse_uuid(raw)?;
    let prev = memory.get(id).await?;
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".into());
    let mut tmp = std::env::temp_dir();
    tmp.push(format!("argo-memory-{id}.md"));
    std::fs::write(&tmp, &prev.body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", tmp.display())))?;
    let status = std::process::Command::new(&editor)
        .arg(&tmp)
        .status()
        .map_err(|e| ArgoError::Provider(format!("spawn {editor}: {e}")))?;
    if !status.success() {
        return Err(ArgoError::Provider(format!(
            "{editor} exited non-zero ({status}); aborting edit"
        )));
    }
    let new_body = std::fs::read_to_string(&tmp)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", tmp.display())))?;
    let _ = std::fs::remove_file(&tmp);
    if new_body == prev.body {
        println!("memory entry {id}: body unchanged — nothing written");
        return Ok(());
    }
    let new = memory
        .supersede(id, new_body, None, PrincipalOverride::Force)
        .await?;
    println!(
        "edited: new active entry {} (file={:?}, supersedes {id})",
        new.id, new.file
    );
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

fn parse_memory_file(raw: &str) -> ArgoResult<MemoryFile> {
    match raw {
        "memory" => Ok(MemoryFile::Memory),
        "user" => Ok(MemoryFile::User),
        "soul" => Ok(MemoryFile::Soul),
        "agents" => Ok(MemoryFile::Agents),
        other => Err(ArgoError::Provider(format!(
            "unknown memory file '{other}': use one of memory|user|soul|agents"
        ))),
    }
}
