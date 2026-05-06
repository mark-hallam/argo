//! `argo roster add-member` handler. Resolves the roster ref +
//! parses a worker uuid, then upserts the membership row.

use super::roster_list::{open_roster_store, parse_tenant, resolve_roster};
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct AddedOut {
    roster_id: String,
    roster_name: String,
    worker_id: String,
    member_count: usize,
}

pub async fn run(
    tenant: &str,
    roster_ref: &str,
    worker_ref: &str,
    json: bool,
) -> ArgoResult<()> {
    let store = open_roster_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let mut roster = resolve_roster(&store, tenant_id, roster_ref).await?;
    let worker_id: WorkerId = worker_ref
        .parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{worker_ref}': {e}")))?;
    store.add_member(roster.id, worker_id).await?;
    roster.member_worker_ids = store.list_members(roster.id).await?;
    let out = AddedOut {
        roster_id: roster.id.to_string(),
        roster_name: roster.name.clone(),
        worker_id: worker_id.to_string(),
        member_count: roster.member_worker_ids.len(),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&out)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!(
            "added worker {} to roster '{}' ({} member(s))",
            out.worker_id, out.roster_name, out.member_count
        );
    }
    Ok(())
}
