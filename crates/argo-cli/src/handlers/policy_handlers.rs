//! `argo policy …` command handlers (Stream `claude-policy-as-code`).
//!
//! The 4-arm dispatcher + per-command implementations live in
//! one file because each handler is short (10-40 LOC) and they
//! share the same loader + json-print helpers. Splitting into 4
//! sibling files would be ceremony without value at this size.
use crate::cli::cli_policy_cmds::PolicyCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::policy::{load_policy_dir, parse_policy_yaml, PolicyError, PolicyFile};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Default workspace policy directory.
pub(crate) const POLICY_DIR: &str = "policies";

pub(crate) async fn run(cmd: PolicyCmd) -> ArgoResult<()> {
    match cmd {
        PolicyCmd::List { dir, json } => list(dir, json).await,
        PolicyCmd::Show { path, json } => show(&path, json),
        PolicyCmd::Validate { path, json } => validate(&path, json),
        PolicyCmd::Apply { path, json } => apply(&path, json).await,
    }
}

async fn list(dir: Option<PathBuf>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    if let Some(dir) = dir {
        return list_dir(&dir, json);
    }
    list_registered(&cwd, json).await
}

fn list_dir(dir: &Path, json: bool) -> ArgoResult<()> {
    let files = load_policy_dir(dir).map_err(map_policy_error)?;
    if json {
        let payload = json!({
            "scope": "directory",
            "dir": dir.display().to_string(),
            "files": files.iter().map(file_summary).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    println!(
        "policy files in {} ({} total)",
        dir.display(),
        files.len()
    );
    for file in &files {
        println!("  {} — {} rules", file.name, file.rules.len());
    }
    Ok(())
}

async fn list_registered(cwd: &Path, json: bool) -> ArgoResult<()> {
    let store = open_workspace_store(cwd).await?;
    let pool = store.pool();
    let workspace_id = first_workspace_id(&store).await?;
    let rows: Vec<(Vec<u8>, String, String, i64)> = sqlx::query_as(
        "SELECT workspace_id, name, body_sha256, applied_at_micros \
         FROM policy_files WHERE workspace_id = ? ORDER BY applied_at_micros DESC",
    )
    .bind(&workspace_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("policy_files query: {e}")))?;
    if json {
        let payload = json!({
            "scope": "registered",
            "count": rows.len(),
            "files": rows.iter().map(|(_, name, sha, applied)| json!({
                "name": name,
                "body_sha256": sha,
                "applied_at_micros": applied,
            })).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    println!("registered policy files ({})", rows.len());
    for (_, name, sha, applied) in &rows {
        println!("  {name:<30} sha256={}…  applied_at={applied}", &sha[..12]);
    }
    Ok(())
}

fn show(path: &Path, json: bool) -> ArgoResult<()> {
    let file = parse_path(path)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    println!("name:        {}", file.name);
    println!(
        "applies_to:  {}",
        file.applies_to
            .iter()
            .map(applies_to_label)
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("rules:");
    for (i, rule) in file.rules.iter().enumerate() {
        println!("  [{i}] action_kind={}", rule.action_kind);
        if let Some(req) = rule.require_signatures {
            println!("       require_signatures: {req}");
        }
        if let Some(qh) = &rule.quiet_hours {
            println!("       quiet_hours: {} → {} ({:?})", qh.start, qh.end, qh.behavior);
        }
        if let Some(cap) = rule.spend_cap_per_action_usd {
            println!("       spend_cap_per_action_usd: ${cap:.2}");
        }
        if rule.audit_chain_required.unwrap_or(false) {
            println!("       audit_chain_required: true");
        }
    }
    Ok(())
}

fn validate(path: &Path, json: bool) -> ArgoResult<()> {
    let result = parse_path(path);
    if json {
        let payload = match &result {
            Ok(file) => json!({
                "ok": true,
                "name": file.name,
                "rule_count": file.rules.len(),
            }),
            Err(e) => json!({
                "ok": false,
                "error": e.to_string(),
            }),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        if result.is_err() {
            std::process::exit(1);
        }
        return Ok(());
    }
    match result {
        Ok(file) => {
            println!("ok: {} ({} rules)", file.name, file.rules.len());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

async fn apply(path: &Path, json: bool) -> ArgoResult<()> {
    let file = parse_path(path)?;
    let body = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Provider(format!(
            "policy apply: read source {}: {e}",
            path.display()
        ))
    })?;
    let body_sha256 = sha256_hex(&body);
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let policy_dir = cwd.join(POLICY_DIR);
    std::fs::create_dir_all(&policy_dir).map_err(|e| {
        ArgoError::Provider(format!(
            "policy apply: create {}: {e}",
            policy_dir.display()
        ))
    })?;
    let dest = policy_dir.join(format!("{}.yaml", file.name));
    std::fs::write(&dest, &body)
        .map_err(|e| ArgoError::Provider(format!("policy apply: write {}: {e}", dest.display())))?;
    let store = open_workspace_store(&cwd).await?;
    let workspace_id = first_workspace_id(&store).await?;
    let now_micros = chrono::Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO policy_files (workspace_id, name, body_sha256, applied_at_micros) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(workspace_id, name) \
         DO UPDATE SET body_sha256 = excluded.body_sha256, applied_at_micros = excluded.applied_at_micros",
    )
    .bind(&workspace_id)
    .bind(&file.name)
    .bind(&body_sha256)
    .bind(now_micros)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("policy_files upsert: {e}")))?;
    if json {
        let payload = json!({
            "ok": true,
            "name": file.name,
            "dest": dest.display().to_string(),
            "body_sha256": body_sha256,
            "applied_at_micros": now_micros,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("Applied policy '{}' → {}", file.name, dest.display());
        println!("  sha256: {body_sha256}");
    }
    Ok(())
}

fn parse_path(path: &Path) -> ArgoResult<PolicyFile> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("policy: read {}: {e}", path.display())))?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("policy")
        .to_string();
    parse_policy_yaml(&name, &body).map_err(map_policy_error)
}

fn map_policy_error(e: PolicyError) -> ArgoError {
    ArgoError::Provider(e.to_string())
}

fn applies_to_label(a: &argo_identity::policy::PolicyApplies) -> String {
    match a {
        argo_identity::policy::PolicyApplies::Role(r) => format!("role={r}"),
        argo_identity::policy::PolicyApplies::Tenant(t) => format!("tenant={t}"),
    }
}

fn file_summary(file: &PolicyFile) -> serde_json::Value {
    json!({
        "name": file.name,
        "applies_to_count": file.applies_to.len(),
        "rule_count": file.rules.len(),
    })
}

fn sha256_hex(body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    hex::encode(hasher.finalize())
}

async fn open_workspace_store(cwd: &Path) -> ArgoResult<SessionStore> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn first_workspace_id(store: &SessionStore) -> ArgoResult<Vec<u8>> {
    let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT id FROM workspace LIMIT 1")
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workspace lookup: {e}")))?;
    row.map(|r| r.0).ok_or_else(|| {
        ArgoError::Provider("no workspace found; run `argo workspace init` first".into())
    })
}

#[cfg(test)]
#[path = "policy_handlers_tests.rs"]
mod tests;
