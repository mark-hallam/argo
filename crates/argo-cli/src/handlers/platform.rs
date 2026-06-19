//! `argo platform staff …` handlers (DEC-057 Phase 1).
//!
//! The deliberate, audited seeding path for Skycot's own operators. A grant is
//! given to an EXISTING user account (they must have signed up first); staff is
//! default-deny, so only rows created here authorize the account-level staff
//! surfaces. Writes to the same canonical store the gateway reads, so a grant
//! made here takes effect on the live `/api/v1/platform/*` routes.

use crate::cli::cli_platform_cmds::{PlatformCmd, StaffCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::platform_staff::{PlatformRole, PlatformStaff};
use argo_identity::platform_staff_store::{list_staff, remove_staff, upsert_staff};
use argo_identity::user_store::get_user_by_email;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::str::FromStr;

/// Top-level `argo platform` dispatch.
pub async fn run(cmd: PlatformCmd) -> ArgoResult<()> {
    match cmd {
        PlatformCmd::Staff { cmd } => run_staff(cmd).await,
    }
}

async fn run_staff(cmd: StaffCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        StaffCmd::Add { email, role, json } => add(&store, &email, &role, json).await,
        StaffCmd::List { json } => list(&store, json).await,
        StaffCmd::Remove { email, json } => remove(&store, &email, json).await,
    }
}

async fn add(store: &SessionStore, email: &str, role_str: &str, as_json: bool) -> ArgoResult<()> {
    let role = PlatformRole::from_str(role_str)
        .map_err(|e| ArgoError::Schema(format!("invalid --role: {e}")))?;
    let user = get_user_by_email(store, email).await?.ok_or_else(|| {
        ArgoError::NotFound(format!(
            "no user account for '{email}' — they must sign up first, then grant staff"
        ))
    })?;
    let grant = PlatformStaff::new(user.id, role, Some("cli".to_string()));
    upsert_staff(store, &grant).await?;
    if as_json {
        println!("{}", serde_json::to_string(&grant).unwrap_or_default());
    } else {
        println!(
            "granted platform-staff '{}' to {email} (user {})",
            role.as_str(),
            user.id
        );
    }
    Ok(())
}

async fn list(store: &SessionStore, as_json: bool) -> ArgoResult<()> {
    let staff = list_staff(store).await?;
    if as_json {
        println!("{}", serde_json::to_string(&staff).unwrap_or_default());
    } else if staff.is_empty() {
        println!("no platform-staff grants");
    } else {
        for s in &staff {
            println!(
                "{}  role={}  status={}  created_at={}",
                s.user_id,
                s.role.as_str(),
                s.status.as_str(),
                s.created_at
            );
        }
    }
    Ok(())
}

async fn remove(store: &SessionStore, email: &str, as_json: bool) -> ArgoResult<()> {
    let user = get_user_by_email(store, email)
        .await?
        .ok_or_else(|| ArgoError::NotFound(format!("no user account for '{email}'")))?;
    let removed = remove_staff(store, user.id).await?;
    if as_json {
        println!("{}", json!({ "email": email, "removed": removed }));
    } else {
        println!("revoked {removed} platform-staff grant(s) for {email}");
    }
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[cfg(test)]
#[path = "platform_tests.rs"]
mod tests;
