//! Resolve `ARGO_HOME` (or `[core].home` from `argo.toml`) to a
//! concrete path, with sane fallbacks.
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use std::path::{Path, PathBuf};

/// Locate the workspace's `argo.toml` (cwd) and resolve the
/// effective home directory. Order of precedence:
/// 1. `ARGO_HOME` env var
/// 2. `[core].home` in the workspace's `argo.toml`
/// 3. `~/.argo`
pub fn resolve_home(cwd: &Path) -> ArgoResult<PathBuf> {
    if let Some(env_home) = std::env::var_os("ARGO_HOME") {
        return Ok(PathBuf::from(env_home));
    }
    let cfg_path = cwd.join("argo.toml");
    let cfg = ArgoConfig::load(&cfg_path)?;
    Ok(cfg.core.resolved_home())
}

/// Resolve the SQLite database path under `home`.
pub fn db_path(home: &Path) -> PathBuf {
    home.join("sessions.db")
}

/// Ensure `dir` exists; creates it (and parents) if missing.
pub fn ensure_dir(dir: &Path) -> ArgoResult<()> {
    std::fs::create_dir_all(dir)
        .map_err(|e| ArgoError::Provider(format!("create_dir_all {}: {e}", dir.display())))
}

/// Resolve the workspace for the current cwd — the canonical path
/// every CLI handler and gateway endpoint uses, matching what
/// writers (`argo new`, `argo work`, `argo session replay`, …)
/// already do via `SessionStore::ensure_workspace`. Callers that
/// previously used `store.first_workspace()` would silently bind
/// to whichever workspace was inserted FIRST, which on a developer
/// machine with multiple historic workspaces is rarely the one the
/// caller wants (#717, #718).
pub async fn cwd_workspace(store: &SessionStore) -> ArgoResult<WorkspaceId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    Ok(workspace)
}
