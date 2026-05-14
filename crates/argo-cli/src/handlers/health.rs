//! `argo health` — meta-command that runs every read-only
//! verification and exits non-zero on the first failure. Wraps the
//! existing CLI surface so the per-command UX stays canonical.
use crate::home::{cwd_workspace, db_path, ensure_dir, resolve_home};
use argo_audit::verify_all as audit_verify_all;
use argo_memory::{MemoryFile, MemoryStore};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run(json: bool, format: &str) -> ArgoResult<()> {
    let payload = collect().await?;
    let ok = payload["ok"].as_bool().unwrap_or(false);
    let failures: Vec<&str> = payload["failures"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            println!(
                "{}",
                serde_json::to_string(&payload)
                    .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
            );
            if ok {
                return Ok(());
            }
            return Err(ArgoError::Provider(format!(
                "argo health: {} check(s) failed",
                failures.len()
            )));
        }
        "markdown" => {
            print_markdown(&payload);
            if ok {
                return Ok(());
            }
            return Err(ArgoError::Provider(format!(
                "argo health: {} check(s) failed",
                failures.len()
            )));
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    let audit = &payload["checks"]["audit"];
    let skill = &payload["checks"]["skill"];
    let memory = &payload["checks"]["memory"];
    eprint!("[argo health]  audit chain ...     ");
    if audit["ok"].as_bool().unwrap_or(false) {
        eprintln!("ok ({} rows)", audit["rows"].as_u64().unwrap_or(0));
    } else if let Some(b) = audit["breaks"].as_u64() {
        eprintln!("FAIL ({b} break(s))");
    } else {
        eprintln!("FAIL ({})", audit["error"].as_str().unwrap_or("?"));
    }
    eprint!("[argo health]  skill provenance ... ");
    if skill["ok"].as_bool().unwrap_or(false) {
        eprintln!("ok ({} skill(s))", skill["count"].as_u64().unwrap_or(0));
    } else {
        eprintln!("FAIL ({})", skill["error"].as_str().unwrap_or("?"));
    }
    eprint!("[argo health]  memory provenance .. ");
    if memory["ok"].as_bool().unwrap_or(false) {
        eprintln!("ok ({} entries)", memory["count"].as_u64().unwrap_or(0));
    } else {
        eprintln!("FAIL ({})", memory["error"].as_str().unwrap_or("?"));
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

/// Markdown render of the health payload — one section per
/// check with a ✓ / FAIL marker. Matches the doctor markdown
/// shape so the two reports compose cleanly side-by-side.
fn print_markdown(payload: &serde_json::Value) {
    let ok = payload["ok"].as_bool().unwrap_or(false);
    let audit = &payload["checks"]["audit"];
    let skill = &payload["checks"]["skill"];
    let memory = &payload["checks"]["memory"];
    println!("# argo health");
    println!();
    println!(
        "**Status**: {}",
        if ok { "all checks ok ✓" } else { "**FAILED**" }
    );
    println!();
    println!("## Audit chain");
    println!();
    if audit["ok"].as_bool().unwrap_or(false) {
        println!("- ✓ ok ({} rows)", audit["rows"].as_u64().unwrap_or(0));
    } else if let Some(b) = audit["breaks"].as_u64() {
        println!("- **FAIL** ({b} break(s))");
    } else {
        println!(
            "- **FAIL** ({})",
            audit["error"].as_str().unwrap_or("?")
        );
    }
    println!();
    println!("## Skill provenance");
    println!();
    if skill["ok"].as_bool().unwrap_or(false) {
        println!(
            "- ✓ ok ({} skill(s))",
            skill["count"].as_u64().unwrap_or(0)
        );
    } else {
        println!(
            "- **FAIL** ({})",
            skill["error"].as_str().unwrap_or("?")
        );
    }
    println!();
    println!("## Memory provenance");
    println!();
    if memory["ok"].as_bool().unwrap_or(false) {
        println!(
            "- ✓ ok ({} entries)",
            memory["count"].as_u64().unwrap_or(0)
        );
    } else {
        println!(
            "- **FAIL** ({})",
            memory["error"].as_str().unwrap_or("?")
        );
    }
}

/// Collect every health check into a single JSON document.
/// Shared between the CLI handler (which prints a human report)
/// and the gateway's `GET /health/full` endpoint.
pub(crate) async fn collect() -> ArgoResult<serde_json::Value> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;

    let mut failures: Vec<&'static str> = Vec::new();
    let audit = match audit_verify_all(&store).await {
        Ok(r) if r.ok => serde_json::json!({"ok": true, "rows": r.rows}),
        Ok(r) => {
            failures.push("audit");
            serde_json::json!({"ok": false, "breaks": r.breaks.len()})
        }
        Err(e) => {
            failures.push("audit");
            serde_json::json!({"ok": false, "error": e.to_string()})
        }
    };
    let skill = match verify_skills(&store).await {
        Ok(n) => serde_json::json!({"ok": true, "count": n}),
        Err(e) => {
            failures.push("skill");
            serde_json::json!({"ok": false, "error": e.to_string()})
        }
    };
    let memory = match verify_memory(&store).await {
        Ok(n) => serde_json::json!({"ok": true, "count": n}),
        Err(e) => {
            failures.push("memory");
            serde_json::json!({"ok": false, "error": e.to_string()})
        }
    };
    Ok(serde_json::json!({
        "ok": failures.is_empty(),
        "checks": {"audit": audit, "skill": skill, "memory": memory},
        "failures": failures,
    }))
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
    // #718: cwd-resolved workspace (matches every writer's path).
    let workspace = cwd_workspace(store).await?;
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
