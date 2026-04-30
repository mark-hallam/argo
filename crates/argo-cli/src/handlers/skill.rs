//! `argo skill` — list / view / install / lock / unlock skills.
use crate::cli::SkillCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{PrincipalOverride, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock};
use std::sync::Arc;

pub async fn run(cmd: SkillCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    let skills = SkillStore::new(store);
    match cmd {
        SkillCmd::List { json } => list(&skills, json).await,
        SkillCmd::View { name, json } => view(&skills, &name, json).await,
        SkillCmd::Verify { name } => verify_hash(&skills, &name).await,
        SkillCmd::Install { source } => install(&skills, &source).await,
        SkillCmd::Lock { name } => set_lock(&skills, &name, Lock::UserProtected).await,
        SkillCmd::Unlock { name } => set_lock(&skills, &name, Lock::None).await,
        other => {
            println!(
                "argo skill: '{other:?}' requires the daemon's capability + judge bindings (separate PR)."
            );
            Ok(())
        }
    }
}

async fn list(skills: &SkillStore, json: bool) -> ArgoResult<()> {
    let active = skills.list_active().await?;
    if json {
        for s in &active {
            let line = serde_json::json!({
                "id": s.id.0.to_string(),
                "name": s.frontmatter.name,
                "version": s.frontmatter.version,
                "lock": format!("{:?}", s.frontmatter.lock),
                "hash": hex::encode(s.hash),
                "created_at": s.created_at.to_rfc3339(),
            });
            let serialized = serde_json::to_string(&line)
                .map_err(|e| ArgoError::Provider(format!("serialize skill: {e}")))?;
            println!("{serialized}");
        }
        return Ok(());
    }
    if active.is_empty() {
        println!("(no active skills)");
        return Ok(());
    }
    println!("{name:<28} {ver:<8} lock", name = "name", ver = "version");
    for s in active {
        println!(
            "{:<28} {:<8} {:?}",
            s.frontmatter.name, s.frontmatter.version, s.frontmatter.lock
        );
    }
    Ok(())
}

async fn view(skills: &SkillStore, name: &str, json: bool) -> ArgoResult<()> {
    match skills.find_active_by_name(name).await? {
        None => Err(ArgoError::Provider(format!(
            "no active skill named '{name}'"
        ))),
        Some(s) => {
            if json {
                let row = serde_json::json!({
                    "id": s.id.0.to_string(),
                    "name": s.frontmatter.name,
                    "version": s.frontmatter.version,
                    "description": s.frontmatter.description,
                    "lock": format!("{:?}", s.frontmatter.lock),
                    "hash": hex::encode(s.hash),
                    "body": s.body,
                    "created_at": s.created_at.to_rfc3339(),
                });
                let serialized = serde_json::to_string(&row)
                    .map_err(|e| ArgoError::Provider(format!("serialize skill: {e}")))?;
                println!("{serialized}");
                return Ok(());
            }
            println!("# {} (v{})", s.frontmatter.name, s.frontmatter.version);
            if !s.frontmatter.description.is_empty() {
                println!("{}", s.frontmatter.description);
            }
            println!("\nlock: {:?}", s.frontmatter.lock);
            println!("\n---\n");
            println!("{}", s.body);
            Ok(())
        }
    }
}

async fn verify_hash(skills: &SkillStore, name: &str) -> ArgoResult<()> {
    use sha2::{Digest, Sha256};
    let s = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let mut h = Sha256::new();
    h.update(s.body.as_bytes());
    let computed = format!("sha256-{}", hex::encode(h.finalize()));
    let stored = &s.frontmatter.provenance.hash;
    if &computed == stored {
        println!(
            "skill '{name}' v{} — provenance hash ok ({})",
            s.frontmatter.version, computed
        );
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "skill '{name}' v{} — provenance hash MISMATCH\n  stored:   {stored}\n  computed: {computed}",
            s.frontmatter.version
        )))
    }
}

async fn install(skills: &SkillStore, source: &str) -> ArgoResult<()> {
    let body = std::fs::read_to_string(source)
        .map_err(|e| ArgoError::Provider(format!("read {source}: {e}")))?;
    let installed = skills.install(&body, None).await?;
    println!(
        "installed {} v{} (id={})",
        installed.frontmatter.name, installed.frontmatter.version, installed.id.0
    );
    Ok(())
}

async fn set_lock(skills: &SkillStore, name: &str, lock: Lock) -> ArgoResult<()> {
    let s = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    skills
        .set_lock(s.id, lock, PrincipalOverride::Force)
        .await?;
    println!("skill '{name}' lock = {lock:?}");
    Ok(())
}
