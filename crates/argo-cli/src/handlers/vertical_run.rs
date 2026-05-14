//! `argo vertical run <id> --workflow <name>` (#669) — one-shot
//! "from zero to running vertical" composite.
//!
//! Creates a fresh session bound to the vertical, auto-attaches
//! every skill the manifest declares (#653), then runs the
//! named workflow against that session via the #668 `--session`
//! plumbing. Emits one combined JSON document so scripting
//! clients don't have to parse multiple shell commands' output.
//!
//! This is the natural composition of:
//! - `argo new --vertical <id>`           (#651, #653 — session bind + skill attach)
//! - `argo vertical workflow … --session` (#659, #668 — workflow runner with audit binding)
//!
//! Reusing the same primitives means there's no new behaviour
//! to test in isolation — only the *composition* needs an
//! integration test.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run(
    vertical_id: &str,
    workflow_name: &str,
    title: Option<&str>,
    provider: &str,
    model: &str,
    json: bool,
) -> ArgoResult<()> {
    // 1. Resolve home + open store.
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let store = Arc::new(store);

    // 2. Resolve the vertical (fail loud on unknown id, same UX
    //    as `argo new --vertical X`).
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let vertical = registry.get(vertical_id).cloned().ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{vertical_id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;

    // 3. Create the session row bound to the vertical.
    let workspace_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let workspace_root = cwd.to_string_lossy();
    let (tenant, workspace) = store
        .ensure_workspace(workspace_name, &workspace_root)
        .await?;
    let session_id = store
        .create_session(tenant, workspace, model, provider, title, Some(vertical_id))
        .await?;

    // 4. Auto-attach the vertical's skill bundle (per-skill
    //    failures don't abort, same policy as `argo new`).
    let skills_attached =
        super::vertical_attach::install_and_attach_bundle(&store, &vertical, session_id)
            .await
            .unwrap_or(0);

    // 5. Run the workflow against this fresh session. Steps that
    //    need an LLM (`skill_invoke`, `prompt`) record
    //    `deferred_no_llm`; the composition still completes.
    let session_id_str = session_id.to_string();
    let report = super::vertical_workflow::run_collect(
        vertical_id,
        workflow_name,
        Some(&session_id_str),
    )
    .await?;

    // 6. Combined output. JSON mode is one document; text mode
    //    prints session metadata then the workflow report.
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "session_id": session_id_str,
                "vertical_id": vertical_id,
                "skills_attached": skills_attached,
                "workflow": report.to_json(),
            }))
            .unwrap_or_default()
        );
    } else {
        println!("created session {session_id_str}");
        println!("  vertical: {vertical_id}");
        if skills_attached > 0 {
            println!("  skills:   {skills_attached} attached from bundle");
        }
        println!();
        report.print_text();
    }

    if !report.ok {
        return Err(ArgoError::Provider(format!(
            "workflow '{workflow_name}': one or more steps failed or were denied/escalated"
        )));
    }
    Ok(())
}
