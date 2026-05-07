//! `argo duty pause <id> [--until <RFC3339>]` handler (Stream
//! `claude-worker-duty-scheduler`).
use crate::handlers::duty_list::open_duty_store;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration as CDur, Utc};
use serde_json::json;

pub async fn run(id: &str, until: Option<&str>, json: bool) -> ArgoResult<()> {
    let store = open_duty_store().await?;
    let until_dt = match until {
        Some(raw) => DateTime::parse_from_rfc3339(raw)
            .map_err(|e| ArgoError::Provider(format!("invalid --until '{raw}': {e}")))?
            .with_timezone(&Utc),
        None => Utc::now() + CDur::hours(24),
    };
    store.set_paused(id, Some(until_dt)).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "paused_until": until_dt.to_rfc3339(),
            }))
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!("paused {id} until {}", until_dt.to_rfc3339());
    }
    Ok(())
}
