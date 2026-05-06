//! `argo classify list` handler — print stored classification tags.

use argo_identity::{ClassificationRules, DataClass};
use argo_store::classification::{list_tags, micros_to_rfc3339};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use std::str::FromStr;

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(
    class: Option<&str>,
    kind: Option<&str>,
    worker: Option<&str>,
    expired: bool,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    if let Some(c) = class {
        DataClass::from_str(c).map_err(|e| ArgoError::Provider(format!("--class: {e}")))?;
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let mut rows = list_tags(store.pool(), class, kind, worker, limit).await?;
    if expired {
        let rules = ClassificationRules::from_workspace(&cwd);
        let now = Utc::now().timestamp_micros();
        rows.retain(|r| {
            DataClass::from_str(&r.class)
                .map(|c| rules.is_expired(c, r.tagged_at_micros, now))
                .unwrap_or(false)
        });
    }
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "record_kind": r.record_kind,
                    "record_id": r.record_id,
                    "class": r.class,
                    "tagged_at_micros": r.tagged_at_micros,
                    "tagged_at": micros_to_rfc3339(r.tagged_at_micros),
                    "tagged_by": r.tagged_by,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no classification tags)");
    } else {
        println!(
            "{:<14} {:<24} {:<10} {:<28} tagged_by",
            "kind", "record_id", "class", "tagged_at"
        );
        for r in &rows {
            let tagged_by = r.tagged_by.as_deref().unwrap_or("-");
            println!(
                "{:<14} {:<24} {:<10} {:<28} {}",
                r.record_kind,
                r.record_id,
                r.class,
                micros_to_rfc3339(r.tagged_at_micros),
                tagged_by,
            );
        }
    }
    Ok(())
}
