//! Confirmation-tier `argo doctor fix` actions — require `--auto` before applying.

use super::doctor_fix::{FixContext, FixReport};
use crate::home::db_path;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

pub(super) async fn run_confirmation(
    ctx: &FixContext,
    id: &'static str,
) -> ArgoResult<FixReport> {
    match id {
        "migrations" => run_migrations(ctx).await,
        "vertical_required_fields" => patch_verticals(ctx),
        _ => Err(ArgoError::Schema(format!("unknown confirmation fix {id}"))),
    }
}

async fn run_migrations(ctx: &FixContext) -> ArgoResult<FixReport> {
    std::fs::create_dir_all(&ctx.home)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", ctx.home.display())))?;
    let db = db_path(&ctx.home);
    let existed = db.exists();
    let store = SessionStore::open(&db).await?;
    let before = store.schema_version().await.unwrap_or(0);
    store.migrate().await?;
    let after = store.schema_version().await.unwrap_or(before);
    Ok(FixReport::new(
        "migrations",
        "confirmation",
        if !existed || after > before {
            "applied"
        } else {
            "noop"
        },
        format!("schema version {before} -> {after}"),
        Some(&db),
        None,
    ))
}

fn patch_verticals(ctx: &FixContext) -> ArgoResult<FixReport> {
    let dir = ctx.cwd.join("verticals");
    if !dir.exists() {
        return Ok(FixReport::noop(
            "vertical_required_fields",
            "confirmation",
            "verticals directory is absent",
            &dir,
        ));
    }
    let mut patched = 0_u64;
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| ArgoError::Io(format!("read vertical entry: {e}")))?;
        let manifest = entry.path().join("vertical.toml");
        if entry.path().is_dir()
            && manifest.exists()
            && patch_one_vertical(&manifest, &entry.file_name().to_string_lossy())?
        {
            patched += 1;
        }
    }
    let status = if patched == 0 { "noop" } else { "applied" };
    let detail = if patched == 0 {
        "no vertical manifests needed required-field patches".to_string()
    } else {
        format!("patched {patched} vertical manifest(s) with required fields")
    };
    Ok(FixReport::new(
        "vertical_required_fields",
        "confirmation",
        status,
        detail,
        Some(&dir),
        None,
    ))
}

fn patch_one_vertical(path: &Path, fallback_id: &str) -> ArgoResult<bool> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let mut value: toml::Value = toml::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
    let root = value
        .as_table_mut()
        .ok_or_else(|| ArgoError::Provider(format!("{} is not a TOML table", path.display())))?;
    let mut changed = insert_string(root, "schema_version", "1");
    let vertical = root
        .entry("vertical".to_string())
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .ok_or_else(|| {
            ArgoError::Provider(format!("{} [vertical] is not a table", path.display()))
        })?;
    let id = vertical
        .get("id")
        .and_then(toml::Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(fallback_id)
        .to_string();
    changed |= insert_string(vertical, "id", &id);
    changed |= insert_string(vertical, "name", &id.replace('-', " "));
    changed |= insert_string(vertical, "version", "0.1.0");
    if changed {
        let out = toml::to_string_pretty(&value)
            .map_err(|e| ArgoError::Provider(format!("serialize {}: {e}", path.display())))?;
        std::fs::write(path, out)
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
    }
    Ok(changed)
}

fn insert_string(table: &mut toml::value::Table, key: &str, value: &str) -> bool {
    if table
        .get(key)
        .and_then(toml::Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
    {
        return false;
    }
    table.insert(key.to_string(), toml::Value::String(value.to_string()));
    true
}
