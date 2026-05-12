//! `argo sandbox e2b list` / `argo sandbox e2b reap-idle` handler
//! (Stream `mission-e2b-reap-idle`).
//!
//! Reads `ARGO_E2B_API_KEY` from the environment and constructs the
//! same [`argo_executor_computer_use::e2b::E2bClient`] the CU
//! executor uses. Two surfaces:
//!
//! - `list` — JSON-line dump of every sandbox the configured API
//!   key owns (one row per line, matches existing `argo` list
//!   conventions).
//! - `reap-idle [--max-age N] [--dry-run]` — close sandboxes idle
//!   longer than `--max-age` seconds. Emits a single JSON object
//!   summarising `scanned` / `reaped` / `errors`.
//!
//! ## Tenancy warning
//!
//! E2B's API-key is the tenancy boundary. Operators sharing one
//! API key across multiple Argo workspaces (or with another E2B-using
//! application) WILL list and/or reap each other's sandboxes — the
//! reaper can't filter by Argo workspace. Run with `--dry-run` first
//! if in doubt; see `docs/skycot/runbook/e2b-reap-idle.md`.

use crate::cli::{E2bSandboxCmd, SandboxCmd};
use argo_executor_computer_use::e2b::{E2bCall, E2bClient};
use argo_executor_computer_use::e2b_reap::filter_idle;
use argo_executor_computer_use::e2b_reap_cron::{
    ARGO_E2B_API_KEY_ENV, ARGO_E2B_BASE_URL_ENV,
};
use argo_types::{ArgoError, ArgoResult};

const DEFAULT_BASE_URL: &str = "https://api.e2b.dev";
const REQUEST_TIMEOUT_SECS: u32 = 30;

pub async fn run(cmd: SandboxCmd) -> ArgoResult<()> {
    match cmd {
        SandboxCmd::E2b { cmd } => dispatch_e2b(cmd).await,
    }
}

async fn dispatch_e2b(cmd: E2bSandboxCmd) -> ArgoResult<()> {
    let client = build_client()?;
    match cmd {
        E2bSandboxCmd::List => list(&client).await,
        E2bSandboxCmd::ReapIdle { max_age, dry_run } => {
            if dry_run {
                reap_dry_run(&client, max_age).await
            } else {
                reap(&client, max_age).await
            }
        }
    }
}

fn build_client() -> ArgoResult<E2bClient> {
    let key = std::env::var(ARGO_E2B_API_KEY_ENV).map_err(|_| {
        ArgoError::Provider(format!(
            "{ARGO_E2B_API_KEY_ENV} unset; export an E2B API key before running `argo sandbox e2b`"
        ))
    })?;
    if key.is_empty() {
        return Err(ArgoError::Provider(format!("{ARGO_E2B_API_KEY_ENV} is empty")));
    }
    let base_url =
        std::env::var(ARGO_E2B_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
    E2bClient::new(base_url, key, REQUEST_TIMEOUT_SECS).map_err(Into::into)
}

async fn list(client: &E2bClient) -> ArgoResult<()> {
    let sandboxes = client.list_sandboxes().await?;
    for info in &sandboxes {
        let line = serde_json::to_string(info)
            .map_err(|e| ArgoError::Provider(format!("serialise sandbox row: {e}")))?;
        println!("{line}");
    }
    Ok(())
}

async fn reap(client: &E2bClient, max_age_secs: u64) -> ArgoResult<()> {
    let report = client.reap_idle(max_age_secs).await?;
    let payload = serde_json::to_string(&report)
        .map_err(|e| ArgoError::Provider(format!("serialise reap report: {e}")))?;
    println!("{payload}");
    Ok(())
}

async fn reap_dry_run(client: &E2bClient, max_age_secs: u64) -> ArgoResult<()> {
    let sandboxes = client.list_sandboxes().await?;
    let now = chrono::Utc::now();
    let candidates = filter_idle(&sandboxes, max_age_secs, now);
    let candidate_ids: Vec<&str> = candidates.iter().map(|s| s.id.as_str()).collect();
    let summary = serde_json::json!({
        "dry_run": true,
        "max_age_secs": max_age_secs,
        "scanned": sandboxes.len(),
        "would_reap": candidate_ids,
    });
    let payload = serde_json::to_string(&summary)
        .map_err(|e| ArgoError::Provider(format!("serialise dry-run summary: {e}")))?;
    println!("{payload}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_client_errors_when_api_key_unset() {
        let prior = std::env::var(ARGO_E2B_API_KEY_ENV).ok();
        unsafe {
            std::env::remove_var(ARGO_E2B_API_KEY_ENV);
        }
        let result = build_client();
        if let Some(v) = prior {
            unsafe { std::env::set_var(ARGO_E2B_API_KEY_ENV, v); }
        }
        assert!(matches!(result, Err(ArgoError::Provider(_))));
    }
}
