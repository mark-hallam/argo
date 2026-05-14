//! `argo handoff-auctions list` handler.

use argo_orchestrator::{auction::micros_to_rfc3339, AuctionStatus, AuctionStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(status: Option<&str>, limit: u32, json: bool) -> ArgoResult<()> {
    let parsed = status
        .map(|s| {
            let v = AuctionStatus::from_str_lossy(s);
            if v == AuctionStatus::Open && s != "open" {
                return Err(ArgoError::Provider(format!(
                    "--status must be one of open/closed/cancelled/no_bidders (got '{s}')"
                )));
            }
            Ok(v)
        })
        .transpose()?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let session_store = SessionStore::open(&db_path(&home)).await?;
    session_store.migrate().await?;
    let store = AuctionStore::new(session_store.pool().clone());
    let rows = store.list(parsed, limit).await?;

    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.task.id,
                    "description": a.task.description,
                    "required_skills": a.task.required_skills,
                    "status": a.status.as_str(),
                    "opened_at_micros": a.opened_at_micros,
                    "opened_at": micros_to_rfc3339(a.opened_at_micros),
                    "closed_at_micros": a.closed_at_micros,
                    "winner_worker_id": a.winner.as_ref().map(|w| w.worker_id.clone()),
                    "winner_score": a.winner.as_ref().map(|w| w.score),
                    "winner_eta_secs": a.winner.as_ref().map(|w| w.eta_secs),
                    "bid_count": a.bids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no auctions)");
    } else {
        println!(
            "{:<32} {:<12} {:<28} {:<14} winner",
            "id", "status", "opened_at", "bids"
        );
        for a in &rows {
            let winner = a
                .winner
                .as_ref()
                .map(|w| format!("{} ({:.1}/{:?}s)", w.worker_id, w.score, w.eta_secs))
                .unwrap_or_else(|| "-".to_string());
            println!(
                "{:<32} {:<12} {:<28} {:<14} {winner}",
                a.task.id,
                a.status.as_str(),
                micros_to_rfc3339(a.opened_at_micros),
                a.bids.len(),
            );
        }
    }
    Ok(())
}
