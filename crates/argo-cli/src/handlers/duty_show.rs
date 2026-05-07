//! `argo duty show <id>` handler (Stream
//! `claude-worker-duty-scheduler`).
use crate::handlers::duty_list::open_duty_store;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let store = open_duty_store().await?;
    let duty = store
        .get(id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("duty '{id}' not found")))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&duty)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        println!("id              {}", duty.id);
        println!("tenant_id       {}", duty.tenant_id);
        println!("worker_id       {}", duty.worker_id);
        println!("cron            {}", duty.cron);
        println!("entry_action    {}", duty.entry_action);
        println!("description     {}", duty.description);
        println!("enabled         {}", duty.enabled);
        println!(
            "paused_until    {}",
            duty.paused_until
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "(none)".into())
        );
        println!(
            "last_fired_at   {}",
            duty.last_fired_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "(never)".into())
        );
        println!("created_at      {}", duty.created_at.to_rfc3339());
    }
    Ok(())
}
