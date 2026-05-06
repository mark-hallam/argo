//! `argo skill conflicts ...` handler (Stream
//! `claude-skill-conflict-resolution`). Drives
//! [`argo_skill::detect_conflicts`] / [`argo_skill::render_diff`]
//! and the lifecycle flip on resolve.
//!
//! ## Library wiring
//!
//! The H-4b two-tier lookup needs a SECOND SkillStore (the
//! library tier). This handler reads the library DB path from
//! `$ARGO_SKILL_LIBRARY_DB`. When the env var is unset, every
//! subcommand returns a 0-conflicts result (or the equivalent
//! "no library configured" error on `show`/`resolve`) so the
//! command degrades cleanly without a library wired.
use crate::cli::SkillConflictsCmd;
use argo_skill::{
    detect_conflicts, parse_keep_flag, render_diff, SkillConflict, SkillStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

/// Top-level dispatch.
pub async fn run(workspace: &SkillStore, cmd: SkillConflictsCmd) -> ArgoResult<()> {
    let library = open_library().await?;
    match cmd {
        SkillConflictsCmd::List { json } => list(workspace, library.as_ref(), json).await,
        SkillConflictsCmd::Show { name, json } => {
            show(workspace, library.as_ref(), &name, json).await
        }
        SkillConflictsCmd::Resolve { name, keep, json } => {
            resolve(workspace, library.as_ref(), &name, &keep, json).await
        }
    }
}

async fn open_library() -> ArgoResult<Option<SkillStore>> {
    let Some(path) = std::env::var_os("ARGO_SKILL_LIBRARY_DB") else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    let store = SessionStore::open(&path).await?;
    store.migrate().await?;
    Ok(Some(SkillStore::new(Arc::new(store))))
}

async fn list(workspace: &SkillStore, library: Option<&SkillStore>, json: bool) -> ArgoResult<()> {
    let conflicts = match library {
        Some(lib) => detect_conflicts(workspace, lib).await?,
        None => Vec::new(),
    };
    if json {
        let payload = serde_json::json!({
            "library_configured": library.is_some(),
            "count": conflicts.len(),
            "conflicts": conflicts.iter().map(summarize).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
        return Ok(());
    }
    if library.is_none() {
        println!("(no library configured — set ARGO_SKILL_LIBRARY_DB to enable)");
        return Ok(());
    }
    if conflicts.is_empty() {
        println!("(no conflicts)");
        return Ok(());
    }
    println!("workspace ↔ library conflicts ({}):", conflicts.len());
    for c in &conflicts {
        println!(
            "  {:<24} workspace@{} | library@{}",
            c.name,
            c.workspace.frontmatter.version,
            c.library.frontmatter.version,
        );
    }
    Ok(())
}

async fn show(
    workspace: &SkillStore,
    library: Option<&SkillStore>,
    name: &str,
    json: bool,
) -> ArgoResult<()> {
    let library = library.ok_or_else(|| {
        ArgoError::Provider(
            "no library configured — set ARGO_SKILL_LIBRARY_DB to enable conflicts subsystem"
                .into(),
        )
    })?;
    let conflicts = detect_conflicts(workspace, library).await?;
    let conflict = conflicts
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| ArgoError::Provider(format!("no conflict for skill name '{name}'")))?;
    let diff = render_diff(conflict);
    if json {
        let payload = serde_json::json!({
            "name": conflict.name,
            "workspace_version": conflict.workspace.frontmatter.version,
            "library_version": conflict.library.frontmatter.version,
            "diff": diff,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        print!("{diff}");
    }
    Ok(())
}

async fn resolve(
    workspace: &SkillStore,
    library: Option<&SkillStore>,
    name: &str,
    keep: &str,
    json: bool,
) -> ArgoResult<()> {
    let library = library.ok_or_else(|| {
        ArgoError::Provider(
            "no library configured — set ARGO_SKILL_LIBRARY_DB to enable conflicts subsystem"
                .into(),
        )
    })?;
    let keep_layer = parse_keep_flag(keep).ok_or_else(|| {
        ArgoError::Schema(format!(
            "--keep '{keep}' must be `workspace` or `library`"
        ))
    })?;
    let conflicts = detect_conflicts(workspace, library).await?;
    let conflict = conflicts
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| ArgoError::Provider(format!("no conflict for skill name '{name}'")))?;
    let loser = conflict.loser(keep_layer);
    let loser_store = match keep_layer {
        argo_skill::SkillLayer::Workspace => library,
        argo_skill::SkillLayer::Library => workspace,
    };
    loser_store
        .transition_lifecycle(loser.id, argo_skill::LifecycleState::Archived)
        .await?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "name": name,
            "kept_layer": format!("{keep_layer:?}").to_lowercase(),
            "archived_skill_id": loser.id.to_string(),
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!(
            "resolved '{name}': kept {:?} (workspace@{}); archived {:?} ({})",
            keep_layer,
            conflict.keeper(keep_layer).frontmatter.version,
            other_layer(keep_layer),
            loser.id,
        );
    }
    Ok(())
}

fn other_layer(layer: argo_skill::SkillLayer) -> argo_skill::SkillLayer {
    match layer {
        argo_skill::SkillLayer::Workspace => argo_skill::SkillLayer::Library,
        argo_skill::SkillLayer::Library => argo_skill::SkillLayer::Workspace,
    }
}

fn summarize(c: &SkillConflict) -> serde_json::Value {
    serde_json::json!({
        "name": c.name,
        "workspace": {
            "id": c.workspace.id.to_string(),
            "version": c.workspace.frontmatter.version,
        },
        "library": {
            "id": c.library.id.to_string(),
            "version": c.library.frontmatter.version,
        },
    })
}
