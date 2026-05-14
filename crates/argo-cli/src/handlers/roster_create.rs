//! `argo roster create` handler. Reads optional policy + channels
//! JSON files, builds a [`Roster`], persists via [`RosterStore`].

use super::roster_list::{open_roster_store, parse_tenant};
use argo_identity::{AutoApprovalPolicy, NotifyChannelRef, Roster};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct CreatedOut {
    id: String,
    name: String,
    tenant_id: String,
    policy_enabled: bool,
}

pub async fn run(
    tenant: &str,
    name: &str,
    policy_path: Option<PathBuf>,
    channels_path: Option<PathBuf>,
    budget_cap_usd: Option<f64>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_roster_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let policy = match policy_path.as_deref() {
        Some(p) => load_json::<AutoApprovalPolicy>(p, "policy")?,
        None => AutoApprovalPolicy::default(),
    };
    let channels = match channels_path.as_deref() {
        Some(p) => load_json::<Vec<NotifyChannelRef>>(p, "channels")?,
        None => Vec::new(),
    };
    let roster = Roster::new(tenant_id, name, policy, channels, budget_cap_usd);
    store.create(&roster).await?;
    let out = CreatedOut {
        id: roster.id.to_string(),
        name: roster.name.clone(),
        tenant_id: roster.tenant_id.to_string(),
        policy_enabled: roster.policy.enabled,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&out)
                .map_err(|e| ArgoError::Provider(format!("serialize created: {e}")))?
        );
    } else {
        println!(
            "created roster '{}' (id {}, policy_enabled = {})",
            out.name, out.id, out.policy_enabled
        );
    }
    Ok(())
}

fn load_json<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
    label: &str,
) -> ArgoResult<T> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {label} '{}': {e}", path.display())))?;
    serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {label} '{}': {e}", path.display())))
}
