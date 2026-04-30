//! `argo skill` — list / view skills in the workspace store.
use crate::cli::SkillCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
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
        SkillCmd::List => list(&skills).await,
        SkillCmd::View { name } => view(&skills, &name).await,
        other => {
            println!(
                "argo skill: '{other:?}' requires the daemon's capability + judge bindings (separate PR)."
            );
            Ok(())
        }
    }
}

async fn list(skills: &SkillStore) -> ArgoResult<()> {
    let active = skills.list_active().await?;
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

async fn view(skills: &SkillStore, name: &str) -> ArgoResult<()> {
    match skills.find_active_by_name(name).await? {
        None => Err(ArgoError::Provider(format!(
            "no active skill named '{name}'"
        ))),
        Some(s) => {
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
