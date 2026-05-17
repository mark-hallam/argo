//! Workspace + credential path helpers shared across the connector handlers.

use super::connector_authorize_specs::ConnectorSpec;
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn spec_for(kind: &str) -> ArgoResult<&'static ConnectorSpec> {
    super::connector_authorize_specs::CONNECTORS
        .iter()
        .find(|spec| spec.kind == kind)
        .ok_or_else(|| provider_err(format!("unknown connector '{kind}'")))
}

pub fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| provider_err(format!("current_dir: {e}")))
}

pub fn connector_dir(workspace: &Path, spec: &ConnectorSpec) -> PathBuf {
    workspace.join(".argo").join("connectors").join(spec.kind)
}

pub fn auth_state_path(workspace: &Path, spec: &ConnectorSpec) -> PathBuf {
    connector_dir(workspace, spec).join("auth.json")
}

pub fn cache_path(workspace: &Path, spec: &ConnectorSpec) -> PathBuf {
    let name = match spec.kind {
        "gmail" | "gcal" => "access_token.json",
        _ => "token.json",
    };
    connector_dir(workspace, spec).join(name)
}

pub fn credential_value(workspace: &Path, key: &str) -> Option<String> {
    argo_config::cred::get_from_workspace(workspace, key)
        .ok()
        .filter(|v| !v.trim().is_empty())
}

pub fn base_url(workspace: &Path, spec: &ConnectorSpec) -> String {
    spec.base_url_key
        .and_then(|key| credential_value(workspace, key))
        .unwrap_or_else(|| spec.default_base_url.to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn provider_err(message: impl Into<String>) -> ArgoError {
    ArgoError::Provider(message.into())
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
