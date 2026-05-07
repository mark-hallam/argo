//! `argo connector list-status`.

use super::connector_authorize::{auth_state_path, cache_path, credential_value, provider_err, workspace_dir, AuthKind, CONNECTORS};
use argo_types::ArgoResult;
use serde::Serialize;
use serde_json::json;
use std::path::Path;

#[derive(Debug, Serialize)]
struct ConnectorStatus { kind: String, auth_type: AuthKind, authorized: bool, credential_key: String, source: String }

pub async fn run(json_out: bool) -> ArgoResult<()> {
    let workspace = workspace_dir()?; let rows = statuses(&workspace);
    if json_out { println!("{}", serde_json::to_string_pretty(&json!({"ok": true, "connector_list_status": {"connectors": rows}})).map_err(|e| provider_err(format!("serialize JSON: {e}")))?); } else { for row in rows { let status = if row.authorized { "authorized" } else { "needs_auth" }; println!("{:<12} {:<10} {:<12} {}", row.kind, auth_label(row.auth_type), status, row.source); } }
    Ok(())
}

fn statuses(workspace: &Path) -> Vec<ConnectorStatus> { CONNECTORS.iter().map(|spec| { let (authorized, source) = status_source(workspace, spec); ConnectorStatus { kind: spec.kind.to_string(), auth_type: spec.auth, authorized, credential_key: spec.token_key.to_string(), source } }).collect() }
fn status_source(workspace: &Path, spec: &super::connector_authorize::ConnectorSpec) -> (bool, String) {
    if credential_value(workspace, spec.token_key).is_some() {
        (true, format!("credential:{}", spec.token_key))
    } else if auth_state_path(workspace, spec).exists() {
        (true, auth_state_path(workspace, spec).display().to_string())
    } else if cache_path(workspace, spec).exists() {
        (true, cache_path(workspace, spec).display().to_string())
    } else {
        (false, "missing".into())
    }
}
fn auth_label(auth: AuthKind) -> &'static str { match auth { AuthKind::OAuth => "oauth", AuthKind::Token => "token" } }

#[cfg(test)]
mod tests { use super::*; use super::super::connector_authorize::{spec_for, write_auth_state}; #[test] fn status_uses_credentials_when_present() { let dir = tempfile::tempdir().unwrap(); argo_config::cred::set(dir.path(), "SLACK_BOT_TOKEN", "tok").unwrap(); let spec = spec_for("slack").unwrap(); let (ok, source) = status_source(dir.path(), spec); assert!(ok); assert_eq!(source, "credential:SLACK_BOT_TOKEN"); } #[test] fn status_uses_auth_state_file() { let dir = tempfile::tempdir().unwrap(); let spec = spec_for("slack").unwrap(); write_auth_state(dir.path(), spec, &["SLACK_BOT_TOKEN"]).unwrap(); let (ok, source) = status_source(dir.path(), spec); assert!(ok); assert!(source.ends_with(".argo/connectors/slack/auth.json")); } }
