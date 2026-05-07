//! `argo plan template promote <id>` — flip a Dreams-distilled
//! learned plan template's TrustGrade from Pending to Promoted.
//! TemplatePlanner consults promoted rows ahead of the shipped
//! defaults on the next `argo plan generate` call.
use argo_store::learned_plan_template::{get, promote, TrustGrade};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let existing = get(store.pool(), id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no learned plan template with id '{id}'")))?;

    if existing.trust_grade == TrustGrade::Promoted {
        if json {
            let envelope = serde_json::json!({
                "ok": true,
                "id": id,
                "already_promoted": true,
            });
            println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
        } else {
            println!("learned-plan-template {id} is already promoted");
        }
        return Ok(());
    }

    promote(store.pool(), id).await?;
    let after = get(store.pool(), id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no learned plan template with id '{id}' after promote")))?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "id": id,
            "row": after,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("learned-plan-template {id} promoted ({})", after.name);
        if let Some(p) = after.promoted_at {
            println!("  promoted_at : {p}");
        }
    }
    Ok(())
}
