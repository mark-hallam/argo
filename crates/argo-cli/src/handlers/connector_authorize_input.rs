//! Operator-input helpers: hidden stdin reads + CLI arg / credential fallback.

use super::connector_authorize_paths::{credential_value, provider_err};
use argo_types::ArgoResult;
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

/// Read an OAuth token from stdin with terminal echo suppressed. Delegates to
/// the shared [`super::secret_input::read_hidden_value`] so the echo-suppression
/// (unix `stty` / Windows Console API) lives in ONE place — the Windows arm is
/// no longer a silent no-op (win-cli-console-echo-suppression-secrets).
pub(super) fn read_hidden_value(label: &str) -> ArgoResult<String> {
    super::secret_input::read_hidden_value(label)
}
