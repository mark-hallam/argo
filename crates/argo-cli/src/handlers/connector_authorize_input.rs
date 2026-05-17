//! Operator-input helpers: hidden stdin reads + CLI arg / credential fallback.

use super::connector_authorize_paths::{credential_value, provider_err};
use argo_types::ArgoResult;
use std::io::Write;
use std::path::Path;

pub(super) struct OAuthArgs {
    pub(super) client_id: Option<String>,
    pub(super) client_secret: Option<String>,
    pub(super) scopes: Vec<String>,
    pub(super) redirect_port: u16,
    pub(super) no_browser: bool,
    pub(super) auth_url: Option<String>,
    pub(super) token_url: Option<String>,
}

pub(super) fn resolve_arg_or_credential(
    workspace: &Path,
    arg: Option<String>,
    key: &str,
) -> ArgoResult<String> {
    arg.filter(|s| !s.trim().is_empty())
        .or_else(|| credential_value(workspace, key))
        .ok_or_else(|| {
            provider_err(format!(
                "missing {key}; pass --client-id/--client-secret or run `argo cred set {key}`"
            ))
        })
}

pub(super) fn read_hidden_value(label: &str) -> ArgoResult<String> {
    eprint!("value for {label}: ");
    std::io::stderr()
        .flush()
        .map_err(|e| provider_err(format!("stderr flush: {e}")))?;
    let echo_disabled = disable_echo();
    let mut value = String::new();
    let result = std::io::stdin()
        .read_line(&mut value)
        .map_err(|e| provider_err(format!("read token from stdin: {e}")));
    if echo_disabled {
        let _ = enable_echo();
        eprintln!();
    }
    result?;
    Ok(value.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(unix)]
fn disable_echo() -> bool {
    std::process::Command::new("stty")
        .arg("-echo")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn disable_echo() -> bool {
    false
}

#[cfg(unix)]
fn enable_echo() -> bool {
    std::process::Command::new("stty")
        .arg("echo")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn enable_echo() -> bool {
    false
}
