//! On-disk credential cache + auth-state writers/readers.

use super::connector_authorize_paths::{
    auth_state_path, cache_path, credential_value, now_secs, provider_err,
};
use super::connector_authorize_specs::{AuthState, ConnectorSpec};
use argo_types::ArgoResult;
use serde::Serialize;
use serde_json::Value;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub fn write_auth_state(workspace: &Path, spec: &ConnectorSpec, keys: &[&str]) -> ArgoResult<()> {
    let state = AuthState {
        kind: spec.kind.to_string(),
        auth_type: spec.auth,
        credential_keys: keys.iter().map(|s| (*s).to_string()).collect(),
        authorized_at_unix: now_secs(),
    };
    write_private_json(&auth_state_path(workspace, spec), &state)
}

pub fn read_access_token(workspace: &Path, spec: &ConnectorSpec) -> ArgoResult<String> {
    if let Some(token) = credential_value(workspace, spec.token_key) {
        return Ok(token);
    }
    let body = std::fs::read_to_string(cache_path(workspace, spec))
        .map_err(|e| provider_err(format!("read {} token cache: {e}", spec.kind)))?;
    let parsed: Value = serde_json::from_str(&body)
        .map_err(|e| provider_err(format!("parse {} token cache: {e}", spec.kind)))?;
    value_str(&parsed, "access_token")
}

pub(super) fn value_str(value: &Value, key: &str) -> ArgoResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| provider_err(format!("missing {key}")))
}

pub(super) fn write_private_json<T: Serialize>(path: &Path, value: &T) -> ArgoResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| provider_err(format!("create {}: {e}", parent.display())))?;
    }
    let body = serde_json::to_vec_pretty(value)
        .map_err(|e| provider_err(format!("serialize {}: {e}", path.display())))?;
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).truncate(true).write(true);
    #[cfg(unix)]
    opts.mode(0o600);
    let mut file = opts
        .open(path)
        .map_err(|e| provider_err(format!("open {}: {e}", path.display())))?;
    file.write_all(&body)
        .map_err(|e| provider_err(format!("write {}: {e}", path.display())))
}

pub(super) fn print_result(label: &str, payload: Value, json_out: bool) -> ArgoResult<()> {
    if json_out {
        let mut envelope = serde_json::Map::new();
        envelope.insert("ok".into(), Value::Bool(true));
        envelope.insert(label.into(), payload.clone());
        println!(
            "{}",
            serde_json::to_string_pretty(&Value::Object(envelope))
                .map_err(|e| provider_err(format!("serialize JSON: {e}")))?
        );
    } else if let Some(kind) = payload.get("kind").and_then(Value::as_str) {
        println!("{label}: {kind} ok");
    } else {
        println!("{label}: ok");
    }
    Ok(())
}
