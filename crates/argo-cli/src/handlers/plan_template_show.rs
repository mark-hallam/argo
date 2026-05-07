//! `argo plan template show <id>` — fetch one learned-plan-template
//! row by id and render its YAML body.
use argo_store::learned_plan_template::get;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let row = get(store.pool(), id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no learned plan template with id '{id}'")))?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "row": row,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("learned-plan-template {id}");
        println!("  name        : {}", row.name);
        println!("  description : {}", row.description);
        println!("  grade       : {}", row.trust_grade.as_str());
        println!("  parent_goal : {}", row.parent_goal);
        println!("  keywords    : {}", row.keywords_csv);
        println!("  cohort_size : {}", row.cohort_size);
        println!("  created_at  : {}", row.created_at);
        if let Some(p) = row.promoted_at {
            println!("  promoted_at : {p}");
        }
        println!("---");
        print!("{}", row.yaml);
    }
    Ok(())
}
