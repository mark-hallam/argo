//! Stream `claude-worker-instance-policy` — `argo worker update`
//! cli handler.
//!
//! Reads a JSON `argo_identity::AutoApprovalPolicy` from the
//! file path the operator passes via
//! `--auto-approval-json <path>` and persists it to the
//! worker's `auto_approval_json` column.
//!
//! Lives in its own file so `worker_create.rs` stays under the
//! per-file 500-LOC budget.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::AutoApprovalPolicy;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// Update one worker's auto-approval policy from a JSON file.
/// Resolves the workspace's session store, validates the JSON
/// against `AutoApprovalPolicy`, and writes the new value to
/// `worker.auto_approval_json` for the row whose id matches.
pub async fn update_auto_approval(
    worker_id: &str,
    json_path: &Path,
    json: bool,
) -> ArgoResult<()> {
    let policy = read_policy_file(json_path)?;
    let (store, _cwd) = open_store().await?;
    let serialised = serde_json::to_string(&policy).map_err(|e| {
        ArgoError::Provider(format!("serialise auto_approval policy: {e}"))
    })?;
    let result = sqlx::query(
        "UPDATE worker SET auto_approval_json = ? WHERE id = ?",
    )
    .bind(&serialised)
    .bind(worker_id)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("update worker policy: {e}")))?;
    if result.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!(
            "no worker with id '{worker_id}' in this workspace"
        )));
    }
    print_outcome(worker_id, &policy, json);
    Ok(())
}

fn read_policy_file(path: &Path) -> ArgoResult<AutoApprovalPolicy> {
    let body = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Provider(format!(
            "could not read auto-approval JSON at {}: {e}",
            path.display()
        ))
    })?;
    serde_json::from_str(&body).map_err(|e| {
        ArgoError::Provider(format!(
            "auto-approval JSON at {} is not a valid AutoApprovalPolicy: {e}",
            path.display()
        ))
    })
}

async fn open_store() -> ArgoResult<(SessionStore, PathBuf)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok((store, home))
}

fn print_outcome(worker_id: &str, policy: &AutoApprovalPolicy, json: bool) {
    if json {
        let payload = serde_json::json!({
            "worker_id": worker_id,
            "auto_approval": policy,
            "ok": true,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("worker {worker_id}: auto_approval updated");
        println!("  enabled: {}", policy.enabled);
        if let Some(cap) = policy.max_usd_micros_per_action {
            println!("  per_action_cap: ${:.2}", cap as f64 / 1_000_000.0);
        }
        if let Some(cap) = policy.daily_budget_usd_micros {
            println!("  daily_cap:      ${:.2}", cap as f64 / 1_000_000.0);
        }
        if !policy.time_windows.is_empty() {
            println!("  time_windows:   {} configured", policy.time_windows.len());
        }
        if !policy.wake_action_types.is_empty() {
            println!(
                "  wake_actions:   [{}]",
                policy.wake_action_types.join(", ")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_policy_file_parses_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.json");
        std::fs::write(
            &path,
            r#"{ "enabled": true, "max_usd_micros_per_action": 500000 }"#,
        )
        .unwrap();
        let policy = read_policy_file(&path).expect("policy parses");
        assert!(policy.enabled);
        assert_eq!(policy.max_usd_micros_per_action, Some(500_000));
    }

    #[test]
    fn read_policy_file_errors_on_missing_path() {
        let path = Path::new("/tmp/skycot-does-not-exist-policy.json");
        let err = read_policy_file(path).expect_err("missing path");
        assert!(err.to_string().contains("could not read"));
    }

    #[test]
    fn read_policy_file_errors_on_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "{ not valid }").unwrap();
        let err = read_policy_file(&path).expect_err("invalid json");
        assert!(err.to_string().contains("not a valid AutoApprovalPolicy"));
    }
}
