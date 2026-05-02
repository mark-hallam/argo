//! `argo new` — create a fresh session row in the local store.
//!
//! Without the live agent loop wired (that's the daemon's job),
//! this command does the minimal thing: register a workspace if
//! needed and insert one row in the `session` table. The new
//! session is then visible to every read-side surface (`argo
//! session list/show/export`, `GET /sessions`, the TUI).
//!
//! Useful for:
//!   - prototyping the read-side surface against a real session
//!   - external clients that want to hand-record a conversation
//!     via `POST /sessions/<id>/messages` later
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(
    provider: &str,
    model: &str,
    title: Option<&str>,
    vertical_id: Option<&str>,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(name, &root).await?;

    // Validate the vertical exists in the discovery dir before
    // we write the row — fast-fail with a friendly error rather
    // than orphaning the session pointing at nothing (#651).
    // Hold on to the loaded vertical for the auto-attach below
    // (#653).
    let loaded_vertical = if let Some(vid) = vertical_id {
        let dir = crate::handlers::vertical::resolve_verticals_dir();
        let (registry, _errors) = argo_vertical::load_dir(&dir)?;
        match registry.get(vid) {
            Some(v) => Some(v.clone()),
            None => {
                return Err(ArgoError::Provider(format!(
                    "vertical '{vid}' not found in {} — run `argo vertical list` to see what's loaded",
                    dir.display()
                )));
            }
        }
    } else {
        None
    };

    let id = store
        .create_session(tenant, workspace, model, provider, title, vertical_id)
        .await?;

    // Vertical-bound session → auto-install + attach the
    // declared skill bundle (#653). Per-skill errors are logged
    // by the helper but don't block session creation; the
    // session is already recorded and useful even if one skill
    // failed to attach.
    let attached = if let Some(v) = loaded_vertical.as_ref() {
        let store_arc = std::sync::Arc::new(store);
        super::vertical_attach::install_and_attach_bundle(&store_arc, v, id)
            .await
            .unwrap_or(0)
    } else {
        0
    };

    println!("created session {id}");
    println!("  provider:  {provider}");
    println!("  model:     {model}");
    if let Some(t) = title {
        println!("  title:     {t}");
    }
    if let Some(v) = vertical_id {
        println!("  vertical:  {v}");
        if attached > 0 {
            println!("  skills:    {attached} attached from vertical bundle");
        }
    }
    println!();
    println!("next: `argo session show {id}` or POST messages via the gateway:");
    println!("  POST /sessions/{id}/messages {{\"role\":\"user\", \"content\":[...]}}");
    Ok(())
}
