//! Bounded "safe" `argo doctor fix` actions — no operator confirmation required.

use super::doctor_fix::{FixContext, FixReport, POLICY_RELPATH, POLICY_TEMPLATE, STALE_AFTER};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

pub(super) fn run_safe(ctx: &FixContext, id: &'static str) -> ArgoResult<FixReport> {
    match id {
        "home_dir" => ensure_dir(id, "safe", &ctx.home, "created ARGO_HOME directory"),
        "manifest_dir" => ensure_dir(
            id,
            "safe",
            &ctx.cwd.join(".argo/manifests"),
            "created workspace manifest directory",
        ),
        "stale_lock" => remove_stale_lock(ctx),
        "workspace_policy" => ensure_policy(ctx),
        _ => Err(ArgoError::Schema(format!("unknown safe fix {id}"))),
    }
}

fn ensure_dir(
    id: &'static str,
    category: &'static str,
    path: &Path,
    detail: &str,
) -> ArgoResult<FixReport> {
    if path.is_dir() {
        return Ok(FixReport::noop(
            id,
            category,
            format!("{} already exists", path.display()),
            path,
        ));
    }
    if path.exists() {
        return Ok(FixReport::blocked(
            id,
            category,
            format!("{} exists but is not a directory", path.display()),
            "move the file aside and rerun doctor fix",
        )
        .with_path(path));
    }
    std::fs::create_dir_all(path)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", path.display())))?;
    Ok(FixReport::applied(id, category, detail, path))
}

fn remove_stale_lock(ctx: &FixContext) -> ArgoResult<FixReport> {
    let path = ctx.cwd.join("cron/.tick.lock");
    if !path.exists() {
        return Ok(FixReport::noop(
            "stale_lock",
            "safe",
            "cron lock file is absent",
            &path,
        ));
    }
    let stale = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age > STALE_AFTER);
    if !stale {
        return Ok(FixReport::noop(
            "stale_lock",
            "safe",
            "cron lock file is not stale",
            &path,
        ));
    }
    std::fs::remove_file(&path)
        .map_err(|e| ArgoError::Io(format!("remove {}: {e}", path.display())))?;
    Ok(FixReport::applied(
        "stale_lock",
        "safe",
        "removed stale cron lock file",
        &path,
    ))
}

fn ensure_policy(ctx: &FixContext) -> ArgoResult<FixReport> {
    let path = ctx.cwd.join(POLICY_RELPATH);
    if path.exists() {
        return Ok(FixReport::noop(
            "workspace_policy",
            "safe",
            "workspace policy already exists",
            &path,
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    }
    std::fs::write(&path, POLICY_TEMPLATE)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    Ok(FixReport::applied(
        "workspace_policy",
        "safe",
        "wrote default disabled workspace policy",
        &path,
    ))
}
