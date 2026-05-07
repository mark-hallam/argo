//! `argo duty fire-now <id>` handler (Stream
//! `claude-worker-duty-scheduler`).
use crate::handlers::duty_list::{default_gate, open_duty_store, open_orchestrator};
use argo_orchestrator::DutyScheduler;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::sync::Arc;

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let duties = Arc::new(open_duty_store().await?);
    let orch = open_orchestrator().await?;
    let scheduler = DutyScheduler::new(duties, orch, default_gate());
    let session_id = scheduler.fire_now(id).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "session_id": session_id.to_string(),
                "decision": "manual_fire",
            }))
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!("fired {id} -> session {session_id}");
    }
    Ok(())
}
