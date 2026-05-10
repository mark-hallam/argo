//! Mutating subcommands for `argo team` — add / remove / rename / create.
//!
//! Split out from `handlers/team.rs` to keep that file under the
//! 500-LOC ceiling. Read-side rendering, dispatcher, and shared
//! helpers stay in the parent module.

use argo_store::{team_roster, SessionStore};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;

use super::team::{open_store, parse_tenant, print_json};

#[derive(Debug, Serialize)]
struct AddPayload<'a> {
    ok: bool,
    team_id: &'a str,
    worker_id: &'a str,
    role: Option<&'a str>,
}

pub(super) async fn add(
    tenant: &str,
    team_id: &str,
    worker_id: &str,
    role: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    store.ensure_team_roster_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    require_team(&store, tenant_id, team_id).await?;
    team_roster::add_team_member(store.pool(), team_id, worker_id, role).await?;
    let payload = AddPayload {
        ok: true,
        team_id,
        worker_id,
        role,
    };
    if json {
        print_json(&payload)?;
    } else {
        let role_str = role.unwrap_or("(none)");
        println!(
            "ok: added worker {worker_id} to team {team_id} (role: {role_str})"
        );
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct RemovePayload<'a> {
    ok: bool,
    team_id: &'a str,
    worker_id: &'a str,
    soft_deleted: bool,
}

pub(super) async fn remove(
    tenant: &str,
    team_id: &str,
    worker_id: &str,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    store.ensure_team_roster_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    require_team(&store, tenant_id, team_id).await?;
    let did = team_roster::remove_team_member(store.pool(), team_id, worker_id).await?;
    let payload = RemovePayload {
        ok: did,
        team_id,
        worker_id,
        soft_deleted: did,
    };
    if json {
        print_json(&payload)?;
    } else if did {
        println!("ok: soft-removed worker {worker_id} from team {team_id}");
    } else {
        println!(
            "no-op: worker {worker_id} was not an active member of team {team_id}"
        );
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct RenamePayload<'a> {
    ok: bool,
    team_id: &'a str,
    name: &'a str,
}

pub(super) async fn rename(
    tenant: &str,
    team_id: &str,
    new_name: &str,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    store.ensure_team_roster_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let updated = team_roster::rename_team(store.pool(), tenant_id, team_id, new_name).await?;
    if !updated {
        return Err(ArgoError::Provider(format!(
            "no team '{team_id}' in tenant '{tenant}'"
        )));
    }
    let payload = RenamePayload {
        ok: true,
        team_id,
        name: new_name,
    };
    if json {
        print_json(&payload)?;
    } else {
        println!("ok: renamed team {team_id} → {new_name}");
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct CreatePayload<'a> {
    ok: bool,
    team_id: &'a str,
    name: &'a str,
    parent: Option<&'a str>,
}

pub(super) async fn create(
    tenant: &str,
    name: &str,
    parent: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    store.ensure_team_roster_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let outcome = team_roster::create_team(store.pool(), tenant_id, name, parent).await?;
    let payload = CreatePayload {
        ok: true,
        team_id: &outcome.id,
        name: &outcome.name,
        parent: outcome.parent_id.as_deref(),
    };
    if json {
        print_json(&payload)?;
    } else {
        match parent {
            Some(p) => println!(
                "ok: created team {} ({}) under parent {}",
                outcome.name, outcome.id, p
            ),
            None => println!(
                "ok: created team {} ({}) at root",
                outcome.name, outcome.id
            ),
        }
    }
    Ok(())
}

async fn require_team(
    store: &SessionStore,
    tenant: TenantId,
    team_id: &str,
) -> ArgoResult<()> {
    let exists = team_roster::get_team(store.pool(), tenant, team_id).await?;
    if exists.is_none() {
        return Err(ArgoError::Provider(format!(
            "no team '{team_id}' in tenant '{tenant}'"
        )));
    }
    Ok(())
}
