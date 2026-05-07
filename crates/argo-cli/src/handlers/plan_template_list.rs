//! `argo plan template list` — enumerate Dreams-distilled
//! plan templates. Filter by `--grade` (pending / promoted /
//! rejected) or pass `--all`.
use argo_store::learned_plan_template::{list, LearnedPlanTemplateFilters, TrustGrade};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

const HARD_LIMIT_CAP: u32 = 500;

pub async fn run(grade: Option<&str>, limit: u32, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let parsed_grade = match grade {
        Some(label) => Some(parse_grade(label)?),
        None => None,
    };
    let capped_limit = limit.clamp(1, HARD_LIMIT_CAP);
    let filters = LearnedPlanTemplateFilters {
        trust_grade: parsed_grade,
    };
    let rows = list(store.pool(), &filters, capped_limit).await?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "count": rows.len(),
            "filters": {
                "grade": grade,
                "limit": capped_limit,
            },
            "rows": rows,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no learned plan templates match the filters)");
    } else {
        println!(
            "{:<14} {:<10} {:<6} {:<26} description",
            "id-prefix", "grade", "size", "name"
        );
        for r in &rows {
            let id_prefix: String = r.id.chars().take(12).collect();
            let name_prefix: String = r.name.chars().take(24).collect();
            println!(
                "{:<14} {:<10} {:<6} {:<26} {}",
                id_prefix,
                r.trust_grade.as_str(),
                r.cohort_size,
                name_prefix,
                r.description
            );
        }
    }
    Ok(())
}

fn parse_grade(label: &str) -> ArgoResult<TrustGrade> {
    match label {
        "pending" => Ok(TrustGrade::Pending),
        "promoted" => Ok(TrustGrade::Promoted),
        "rejected" => Ok(TrustGrade::Rejected),
        other => Err(ArgoError::Provider(format!(
            "unknown --grade '{other}' (expected: pending, promoted, rejected)"
        ))),
    }
}
