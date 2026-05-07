//! `argo duty resume <id>` handler (Stream
//! `claude-worker-duty-scheduler`).
use crate::handlers::duty_list::open_duty_store;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let store = open_duty_store().await?;
    store.set_paused(id, None).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "paused_until": serde_json::Value::Null,
            }))
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!("resumed {id}");
    }
    Ok(())
}
