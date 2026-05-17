//! `argo connector authorize` plus shared connector auth helpers.
//!
//! Sibling modules:
//! - [`connector_authorize_specs`][s] — connector table + auth types.
//! - [`connector_authorize_paths`][p] — workspace + credential paths.
//! - [`connector_authorize_state`][st] — on-disk auth-state writer/reader.
//! - [`connector_authorize_oauth`][o] — OAuth URL / code exchange / callback.
//! - [`connector_authorize_input`][i] — hidden stdin + arg/credential fallback.
//!
//! [s]: super::connector_authorize_specs
//! [p]: super::connector_authorize_paths
//! [st]: super::connector_authorize_state
//! [o]: super::connector_authorize_oauth
//! [i]: super::connector_authorize_input

use super::connector_authorize_input::{read_hidden_value, resolve_arg_or_credential, OAuthArgs};
use super::connector_authorize_oauth::{
    build_authorize_url, capture_oauth_code, exchange_code, new_state, open_browser,
};
use super::connector_authorize_specs::OAuthTokenCache;
use super::connector_authorize_state::{print_result, value_str, write_private_json};
use crate::cli::ConnectorCmd;
use argo_types::ArgoResult;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::Path;

pub use super::connector_authorize_paths::{
    auth_state_path, base_url, cache_path, connector_dir, credential_value, now_secs, provider_err,
    spec_for, workspace_dir,
};
pub use super::connector_authorize_specs::{AuthKind, ConnectorSpec, OAuthSpec, CONNECTORS};
pub use super::connector_authorize_state::{read_access_token, write_auth_state};

pub async fn run(cmd: ConnectorCmd) -> ArgoResult<()> {
    match cmd {
        ConnectorCmd::List {
            installed_only,
            json,
        } => crate::handlers::connector_list::run(installed_only, json).await,
        ConnectorCmd::Show { kind, json } => {
            crate::handlers::connector_show::run(&kind, json).await
        }
        ConnectorCmd::Authorize {
            kind,
            token,
            client_id,
            client_secret,
            scopes,
            redirect_port,
            no_browser,
            auth_url,
            token_url,
            json,
        } => {
            authorize(
                &kind,
                token,
                OAuthArgs {
                    client_id,
                    client_secret,
                    scopes,
                    redirect_port,
                    no_browser,
                    auth_url,
                    token_url,
                },
                json,
            )
            .await
        }
        ConnectorCmd::ListStatus { json } => {
            crate::handlers::connector_list_status::run(json).await
        }
        ConnectorCmd::Revoke { kind, json } => {
            crate::handlers::connector_revoke::run(&kind, json).await
        }
        ConnectorCmd::Test { kind, json } => {
            crate::handlers::connector_test::run(&kind, json).await
        }
    }
}

async fn authorize(
    kind: &str,
    token: Option<String>,
    oauth_args: OAuthArgs,
    json_out: bool,
) -> ArgoResult<()> {
    let spec = spec_for(kind)?;
    match spec.auth {
        AuthKind::Token => authorize_token(spec, token, json_out).await,
        AuthKind::OAuth => authorize_oauth(spec, oauth_args, json_out).await,
    }
}

async fn authorize_token(
    spec: &'static ConnectorSpec,
    token: Option<String>,
    json_out: bool,
) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let token = match token {
        Some(token) => token,
        None => read_hidden_value(&format!("{} token", spec.kind))?,
    };
    if token.trim().is_empty() {
        return Err(provider_err("token must not be empty"));
    }
    let outcome = crate::handlers::connector_test::validate_with_token(spec, &token).await?;
    argo_config::cred::set(&workspace, spec.token_key, &token)?;
    write_auth_state(&workspace, spec, &[spec.token_key])?;
    print_result(
        "connector_authorize",
        json!({"kind": spec.kind, "auth_type": spec.auth, "validated": outcome.ok, "credential_key": spec.token_key}),
        json_out,
    )
}

async fn authorize_oauth(
    spec: &'static ConnectorSpec,
    args: OAuthArgs,
    json_out: bool,
) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let oauth = spec
        .oauth
        .ok_or_else(|| provider_err("missing OAuth spec"))?;
    let client_id = resolve_arg_or_credential(&workspace, args.client_id, oauth.client_id_key)?;
    let client_secret =
        resolve_arg_or_credential(&workspace, args.client_secret, oauth.client_secret_key)?;
    let listener = TcpListener::bind(("127.0.0.1", args.redirect_port))
        .map_err(|e| provider_err(format!("bind OAuth callback: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| provider_err(format!("callback addr: {e}")))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let state = new_state();
    let scopes = if args.scopes.is_empty() {
        oauth.scopes.iter().map(|s| (*s).to_string()).collect()
    } else {
        args.scopes
    };
    let auth_url = build_authorize_url(
        args.auth_url.as_deref().unwrap_or(oauth.auth_url),
        &client_id,
        &redirect_uri,
        &state,
        &scopes,
    );
    if args.no_browser {
        eprintln!("open this URL to authorize {}: {auth_url}", spec.kind);
    } else {
        open_browser(&auth_url)?;
    }
    let code = capture_oauth_code(listener, state).await?;
    let token = exchange_code(
        args.token_url.as_deref().unwrap_or(oauth.token_url),
        &client_id,
        &client_secret,
        &redirect_uri,
        &code,
    )
    .await?;
    persist_oauth(&workspace, spec, oauth, &client_id, &client_secret, &token)?;
    print_result(
        "connector_authorize",
        json!({"kind": spec.kind, "auth_type": spec.auth, "credential_key": spec.token_key, "token_cache": cache_path(&workspace, spec)}),
        json_out,
    )
}

fn persist_oauth(
    workspace: &Path,
    spec: &ConnectorSpec,
    oauth: OAuthSpec,
    client_id: &str,
    client_secret: &str,
    token: &Value,
) -> ArgoResult<()> {
    let access_token = value_str(token, "access_token")?;
    let refresh_token = token
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::to_string);
    let expires_at = now_secs()
        + token
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(3600)
            .saturating_sub(60);
    let instance_url = token
        .get("instance_url")
        .and_then(Value::as_str)
        .map(|s| s.trim_end_matches('/').to_string());
    argo_config::cred::set(workspace, spec.token_key, &access_token)?;
    argo_config::cred::set(workspace, oauth.client_id_key, client_id)?;
    argo_config::cred::set(workspace, oauth.client_secret_key, client_secret)?;
    if let (Some(key), Some(value)) = (oauth.refresh_key, refresh_token.as_deref()) {
        argo_config::cred::set(workspace, key, value)?;
    }
    if let (Some(key), Some(value)) = (oauth.instance_url_key, instance_url.as_deref()) {
        argo_config::cred::set(workspace, key, value)?;
    }
    let cache = OAuthTokenCache {
        access_token,
        refresh_token,
        expires_at_unix: Some(expires_at),
        expires_at: Some(expires_at as i64),
        instance_url,
        token_type: token
            .get("token_type")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    write_private_json(&cache_path(workspace, spec), &cache)?;
    let mut keys = vec![spec.token_key, oauth.client_id_key, oauth.client_secret_key];
    if let Some(key) = oauth.refresh_key {
        keys.push(key);
    }
    if let Some(key) = oauth.instance_url_key {
        keys.push(key);
    }
    write_auth_state(workspace, spec, &keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_lookup_knows_slack() {
        let spec = spec_for("slack").unwrap();
        assert_eq!(spec.auth, AuthKind::Token);
        assert_eq!(spec.token_key, "SLACK_BOT_TOKEN");
    }
}
