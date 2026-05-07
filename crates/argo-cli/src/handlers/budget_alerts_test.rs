//! `argo budget alerts test` — fire a synthetic budget alert.

use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;

pub async fn run(worker: &str, threshold: u8, json: bool) -> ArgoResult<()> {
    if !matches!(threshold, 80 | 100) {
        return Err(ArgoError::Provider(
            "budget alert test threshold must be 80 or 100".into(),
        ));
    }
    let store = super::budget_watch::open_store().await?;
    let now = Utc::now();
    let candidate = super::budget_watch::AlertCandidate {
        worker_id: worker.to_string(),
        threshold,
        spend_amount: 0,
        month_spend_amount: 0,
        cap_amount: 0,
        headroom_amount: 0,
        synthetic: true,
    };
    let row = super::budget_watch::fire_alert(&store, candidate, now).await?;
    if json {
        let body = serde_json::to_string(&serde_json::json!({
            "ok": true,
            "synthetic": true,
            "alert": row,
        }))
        .map_err(|e| ArgoError::Provider(format!("serialize synthetic alert: {e}")))?;
        println!("{body}");
    } else {
        println!(
            "synthetic budget alert worker={} threshold={} via={}",
            row.worker_id, row.threshold, row.notified_via
        );
    }
    Ok(())
}
