//! `argo memory` — list / view / revert / lock / unlock memory entries.
use crate::cli::MemoryCmd;
use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock};
use std::sync::Arc;

pub(crate) use super::memory_query::{count, list};

pub async fn run(cmd: MemoryCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    // #717 / #718: cwd-resolved workspace (matches every writer's path).
    let workspace = cwd_workspace(&store).await?;
    let memory = MemoryStore::new(Arc::clone(&store));
    match cmd {
        MemoryCmd::SharedTeam { cmd } => super::memory_shared_team::run(&memory, cmd).await,
        MemoryCmd::List {
            json,
            format,
            file,
            sort_by,
            limit,
            since_minutes,
            lock,
            body_contains,
            vertical,
        } => {
            list(
                &memory,
                workspace,
                json,
                &format,
                file.as_deref(),
                &sort_by,
                limit,
                since_minutes,
                lock.as_deref(),
                body_contains.as_deref(),
                vertical.as_deref(),
            )
            .await
        }
        MemoryCmd::Count {
            file,
            since_minutes,
            json,
        } => count(&memory, workspace, file.as_deref(), since_minutes, json).await,
        MemoryCmd::View { entry, json } => view(&memory, &entry, json).await,
        MemoryCmd::Append {
            file,
            body,
            vertical,
            session,
        } => {
            append(
                &memory,
                workspace,
                &file,
                &body,
                vertical.as_deref(),
                session.as_deref(),
            )
            .await
        }
        MemoryCmd::Verify { entry, all } => {
            if all {
                verify_all(&memory, workspace).await
            } else {
                let raw = entry.unwrap_or_default();
                verify_hash(&memory, &raw).await
            }
        }
        MemoryCmd::Edit { entry } => edit(&memory, &entry).await,
        MemoryCmd::Revert { entry } => revert(&memory, &entry).await,
        MemoryCmd::Forget { entry } => revert(&memory, &entry).await,
        MemoryCmd::Lock { entry } => set_lock(&memory, &entry, Lock::UserProtected).await,
        MemoryCmd::Unlock { entry } => set_lock(&memory, &entry, Lock::None).await,
        MemoryCmd::DreamNow { worker, scope, json } => super::memory_dream_now::run(&worker, &scope, json).await,
    }
}

async fn append(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    file: &str,
    body: &str,
    vertical: Option<&str>,
    session: Option<&str>,
) -> ArgoResult<()> {
    let mf = parse_memory_file(file)?;
    // #697: --session attribution. Required when the row will
    // later be promoted to a learned skill (the audit row needs
    // a session to attribute to). Operators can still omit it
    // for workspace-global writes.
    let learned_in = match session {
        None => None,
        Some(s) => Some(argo_types::SessionId(uuid::Uuid::parse_str(s).map_err(
            |e| {
                argo_types::ArgoError::Provider(format!(
                    "invalid --session uuid '{s}': {e}"
                ))
            },
        )?)),
    };
    let entry = match vertical {
        Some(vid) => {
            // #686 — Rule 7: vertical-aware write. Tags the row
            // with `vertical_id` so vertical-scoped reads can
            // filter to it.
            memory
                .append_for_vertical(workspace, mf, body.to_string(), learned_in, vid)
                .await?
        }
        None => memory.append(workspace, mf, body.to_string(), learned_in).await?,
    };
    let scope = entry
        .vertical_id
        .as_deref()
        .map(|v| format!(" [vertical={v}]"))
        .unwrap_or_default();
    let learned = entry
        .learned_in
        .map(|sid| format!(" [session={}]", sid.0))
        .unwrap_or_default();
    println!(
        "appended {} to {}.md ({} bytes){scope}{learned}",
        entry.id,
        file,
        body.len()
    );
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

async fn verify_all(memory: &MemoryStore, workspace: argo_types::WorkspaceId) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let files = [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ];
    let mut total = 0_usize;
    let mut failures: Vec<String> = Vec::new();
    for file in files {
        let entries = memory.list_active(workspace, file).await?;
        for e in entries {
            total += 1;
            let mut h = Sha256::new();
            h.update(e.body.as_bytes());
            let computed: [u8; 32] = h.finalize().into();
            if computed == e.hash {
                println!("  ok       {} ({:?})", e.id, file);
            } else {
                failures.push(format!(
                    "{} ({:?}) stored={} computed={}",
                    e.id,
                    file,
                    hex::encode(e.hash),
                    hex::encode(computed)
                ));
                println!("  MISMATCH {} ({:?})", e.id, file);
            }
        }
    }
    if total == 0 {
        println!("(no active memory entries)");
        return Ok(());
    }
    if failures.is_empty() {
        println!("all {total} memory entries verified — body hashes match");
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "{} of {} memory entries failed verification:\n  {}",
            failures.len(),
            total,
            failures.join("\n  ")
        )))
    }
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
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", tmp.display())))?;
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
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", tmp.display())))?;
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
        .map_err(|e| ArgoError::Schema(format!("invalid uuid '{raw}': {e}")))
}

pub(super) fn parse_memory_file(raw: &str) -> ArgoResult<MemoryFile> {
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
