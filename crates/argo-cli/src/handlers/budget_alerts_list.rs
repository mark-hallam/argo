//! `argo budget alerts list` — show recent budget push alerts.

use argo_types::{ArgoError, ArgoResult};

pub async fn run(
    worker: Option<&str>,
    since: Option<&str>,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let store = super::budget_watch::open_store().await?;
    let since = super::budget_watch::parse_since(since)?;
    let rows = super::budget_watch::list_alerts(&store, worker, since, limit).await?;
    if json {
        for row in &rows {
            println!(
                "{}",
                serde_json::to_string(row)
                    .map_err(|e| ArgoError::Provider(format!("serialize alert: {e}")))?
            );
        }
    } else if rows.is_empty() {
        println!("(no budget alerts)");
    } else {
        println!(
            "{:<20}  {:<8}  {:>9}  {:>10}  {:>10}  via",
            "fired_at", "worker", "threshold", "spend", "headroom"
        );
        for row in &rows {
            println!(
                "{:<20}  {:<8}  {:>8}%  ${:>9.2}  ${:>9.2}  {}",
                row.fired_at,
                row.worker_id,
                row.threshold,
                row.spend_amount_usd,
                row.headroom_amount_usd,
                row.notified_via
            );
        }
    }
    Ok(())
}
