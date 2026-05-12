//! `argo audit archive` / `list-archives` / `verify-archive` (#228).
//!
//! Wraps `argo_audit::archive::*`, resolves `<archive-dir>` from CLI
//! arg or `<ARGO_HOME>/audit-archive/`, opens the workspace SQLite,
//! and renders text or `--json` summaries.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::archive::{
    archive_window, list_archives, verify_archive, ArchiveBundle, ArchiveConfig,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

fn pe(e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(e.to_string())
}

fn resolve_archive_dir(arg: Option<&str>) -> ArgoResult<PathBuf> {
    if let Some(s) = arg {
        return Ok(PathBuf::from(s));
    }
    let cwd = std::env::current_dir().map_err(|e| pe(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    Ok(home.join("audit-archive"))
}

async fn open_store() -> ArgoResult<Arc<SessionStore>> {
    let cwd = std::env::current_dir().map_err(|e| pe(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    Ok(store)
}

/// `argo audit archive`.
pub async fn run_archive(
    retention_days: u32,
    prune: bool,
    hard_keep_days: Option<u32>,
    archive_dir: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let dir = resolve_archive_dir(archive_dir)?;
    let mut cfg = ArchiveConfig::new(dir.clone());
    cfg.retention_days = retention_days;
    cfg.prune_after_archive = prune;
    cfg.hard_keep_days = hard_keep_days;
    let store = open_store().await?;
    let bundle = archive_window(&store, &cfg).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "archive": {
                    "ok": true,
                    "id": bundle.id,
                    "filename": bundle.filename,
                    "row_count": bundle.row_count,
                    "range_start": bundle.range_start,
                    "range_end": bundle.range_end,
                    "cutoff": bundle.cutoff,
                    "sha256": bundle.sha256,
                    "signed_by": bundle.signed_by,
                    "archive_dir": dir.display().to_string(),
                    "pruned": prune,
                }
            }))
            .map_err(pe)?
        );
    } else {
        println!(
            "archived {} row(s) → {}/{} (cutoff {}, sha256 {}, hmac {}{})",
            bundle.row_count,
            dir.display(),
            bundle.filename,
            bundle.cutoff.format("%Y-%m-%d"),
            &bundle.sha256[..16],
            &bundle.signed_by[..16.min(bundle.signed_by.len())],
            if prune { ", pruned online rows" } else { "" }
        );
    }
    Ok(())
}

/// `argo audit list-archives`.
pub async fn run_list(archive_dir: Option<&str>, json: bool) -> ArgoResult<()> {
    let dir = resolve_archive_dir(archive_dir)?;
    let entries = list_archives(&dir)?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "archives": entries,
                "archive_dir": dir.display().to_string(),
            }))
            .map_err(pe)?
        );
    } else if entries.is_empty() {
        println!("(no archives in {})", dir.display());
    } else {
        for b in &entries {
            println!(
                "{}  {}  rows={}  range={}..{}",
                b.id,
                b.filename,
                b.row_count,
                b.range_start.format("%Y-%m-%d"),
                b.range_end.format("%Y-%m-%d"),
            );
        }
    }
    Ok(())
}

/// `argo audit verify-archive <bundle_id>`.
pub async fn run_verify(bundle_id: &str, archive_dir: Option<&str>, json: bool) -> ArgoResult<()> {
    let dir = resolve_archive_dir(archive_dir)?;
    let entries = list_archives(&dir)?;
    let bundle: ArchiveBundle = entries
        .into_iter()
        .find(|b| b.id == bundle_id)
        .ok_or_else(|| pe(format!("no bundle {bundle_id} in {}", dir.display())))?;
    let outcome = verify_archive(&bundle, &dir);
    let ok = outcome.is_ok();
    let detail = outcome.as_ref().err().map(|e| e.to_string());
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "verify": {
                    "ok": ok,
                    "id": bundle.id,
                    "filename": bundle.filename,
                    "detail": detail,
                }
            }))
            .map_err(pe)?
        );
    } else if ok {
        println!("OK {} ({})", bundle.id, bundle.filename);
    } else {
        println!(
            "FAIL {} ({}): {}",
            bundle.id,
            bundle.filename,
            detail.unwrap_or_default()
        );
    }
    outcome
}
