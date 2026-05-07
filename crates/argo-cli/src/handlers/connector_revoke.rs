//! `argo connector revoke`.

use super::connector_authorize::{connector_dir, provider_err, spec_for, workspace_dir};
use argo_types::ArgoResult;
use serde::Serialize;
use serde_json::json;
use std::io::ErrorKind;
use std::path::Path;

#[derive(Debug, Serialize)]
struct RevokeOutcome { kind: String, removed_items: usize }

pub async fn run(kind: &str, json_out: bool) -> ArgoResult<()> {
    let spec = spec_for(kind)?; let workspace = workspace_dir()?; let removed = revoke_in_workspace(&workspace, spec)?; let outcome = RevokeOutcome { kind: spec.kind.to_string(), removed_items: removed };
    if json_out { println!("{}", serde_json::to_string_pretty(&json!({"ok": true, "connector_revoke": outcome})).map_err(|e| provider_err(format!("serialize JSON: {e}")))?); } else { println!("connector_revoke: {} removed {} item(s)", spec.kind, removed); }
    Ok(())
}

fn revoke_in_workspace(workspace: &Path, spec: &super::connector_authorize::ConnectorSpec) -> ArgoResult<usize> { let mut removed = remove_connector_dir(workspace, spec)?; for key in credential_keys(spec) { if argo_config::cred::clear(workspace, key)? { removed += 1; } } Ok(removed) }
fn remove_connector_dir(workspace: &Path, spec: &super::connector_authorize::ConnectorSpec) -> ArgoResult<usize> { let dir = connector_dir(workspace, spec); match std::fs::remove_dir_all(&dir) { Ok(()) => Ok(1), Err(e) if e.kind() == ErrorKind::NotFound => Ok(0), Err(e) => Err(provider_err(format!("remove {}: {e}", dir.display()))) } }
fn credential_keys(spec: &super::connector_authorize::ConnectorSpec) -> Vec<&'static str> {
    let mut keys = vec![spec.token_key];
    if let Some(oauth) = spec.oauth {
        keys.push(oauth.client_id_key);
        keys.push(oauth.client_secret_key);
        keys.extend(oauth.refresh_key);
        keys.extend(oauth.instance_url_key);
    }
    keys
}

#[cfg(test)]
mod tests { use super::*; use super::super::connector_authorize::{connector_dir, spec_for}; #[test] fn revoke_removes_connector_dir_and_credential() { let dir = tempfile::tempdir().unwrap(); let spec = spec_for("slack").unwrap(); std::fs::create_dir_all(connector_dir(dir.path(), spec)).unwrap(); argo_config::cred::set(dir.path(), "SLACK_BOT_TOKEN", "tok").unwrap(); let removed = revoke_in_workspace(dir.path(), spec).unwrap(); assert_eq!(removed, 2); assert!(!connector_dir(dir.path(), spec).exists()); assert!(argo_config::cred::load(dir.path()).unwrap().is_empty()); } #[test] fn oauth_revoke_includes_refresh_credentials() { let spec = spec_for("gmail").unwrap(); let keys = credential_keys(spec); assert!(keys.contains(&"GMAIL_ACCESS_TOKEN")); assert!(keys.contains(&"ARGO_GMAIL_REFRESH_TOKEN")); } }
