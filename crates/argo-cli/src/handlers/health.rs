//! `argo health` — meta-command that runs every read-only
//! verification and exits non-zero on the first failure. Wraps the
//! existing CLI surface so the per-command UX stays canonical.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::verify_all as audit_verify_all;
use argo_memory::{MemoryFile, MemoryStore};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run(json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;

    let mut failures: Vec<&'static str> = Vec::new();

    if !json {
        eprint!("[argo health]  audit chain ...     ");
    }
    let audit_status = match audit_verify_all(&store).await {
        Ok(r) if r.ok => {
            if !json {
                eprintln!("ok ({} rows)", r.rows);
            }
            serde_json::json!({"ok": true, "rows": r.rows})
        }
        Ok(r) => {
            if !json {
                eprintln!("FAIL ({} break(s))", r.breaks.len());
            }
            failures.push("audit");
            serde_json::json!({"ok": false, "breaks": r.breaks.len()})
        }
        Err(e) => {
            if !json {
                eprintln!("FAIL ({e})");
            }
            failures.push("audit");
            serde_json::json!({"ok": false, "error": e.to_string()})
        }
    };

    if !json {
        eprint!("[argo health]  skill provenance ... ");
    }
    let skill_status = match verify_skills(&store).await {
        Ok(n) => {
            if !json {
                eprintln!("ok ({n} skill(s))");
            }
            serde_json::json!({"ok": true, "count": n})
        }
        Err(e) => {
            if !json {
                eprintln!("FAIL ({e})");
            }
            failures.push("skill");
            serde_json::json!({"ok": false, "error": e.to_string()})
        }
    };

    if !json {
        eprint!("[argo health]  memory provenance .. ");
    }
    let memory_status = match verify_memory(&store).await {
        Ok(n) => {
            if !json {
                eprintln!("ok ({n} entries)");
            }
            serde_json::json!({"ok": true, "count": n})
        }
        Err(e) => {
            if !json {
                eprintln!("FAIL ({e})");
            }
            failures.push("memory");
            serde_json::json!({"ok": false, "error": e.to_string()})
        }
    };

    if json {
        let payload = serde_json::json!({
            "ok": failures.is_empty(),
            "checks": {
                "audit": audit_status,
                "skill": skill_status,
                "memory": memory_status,
            },
            "failures": failures,
        });
        println!(
            "{}",
            serde_json::to_string(&payload)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
        if failures.is_empty() {
            return Ok(());
        }
        return Err(ArgoError::Provider(format!(
            "argo health: {} check(s) failed",
            failures.len()
        )));
    }

    if failures.is_empty() {
        println!("argo health: all checks ok");
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "argo health: {} check(s) failed: {}",
            failures.len(),
            failures.join(", ")
        )))
    }
}

async fn verify_skills(store: &Arc<SessionStore>) -> ArgoResult<usize> {
    use sha2::{Digest, Sha256};
    let skills = SkillStore::new(Arc::clone(store));
    let active = skills.list_active().await?;
    let mut bad = Vec::new();
    for s in &active {
        let mut h = Sha256::new();
        h.update(s.body.as_bytes());
        let computed = format!("sha256-{}", hex::encode(h.finalize()));
        if computed != s.frontmatter.provenance.hash {
            bad.push(s.frontmatter.name.clone());
        }
    }
    if bad.is_empty() {
        Ok(active.len())
    } else {
        Err(ArgoError::Provider(format!("hash mismatch: {bad:?}")))
    }
}

async fn verify_memory(store: &Arc<SessionStore>) -> ArgoResult<usize> {
    use sha2::{Digest, Sha256};
    let workspace = match store.first_workspace().await? {
        Some(w) => w,
        None => return Ok(0),
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let mut total = 0_usize;
    let mut bad = 0_usize;
    for f in [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ] {
        for entry in memory.list_active(workspace, f).await? {
            total += 1;
            let mut h = Sha256::new();
            h.update(entry.body.as_bytes());
            let computed: [u8; 32] = h.finalize().into();
            if computed != entry.hash {
                bad += 1;
            }
        }
    }
    if bad == 0 {
        Ok(total)
    } else {
        Err(ArgoError::Provider(format!("{bad}/{total} hash mismatch")))
    }
}
