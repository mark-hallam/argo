//! `argo handoff-auctions show` handler.

use argo_orchestrator::{auction::micros_to_rfc3339, AuctionStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let session_store = SessionStore::open(&db_path(&home)).await?;
    session_store.migrate().await?;
    let store = AuctionStore::new(session_store.pool().clone());
    let auction = store.get(id).await?.ok_or_else(|| {
        ArgoError::Provider(format!("auction '{id}' not found"))
    })?;

    if json {
        let bids: Vec<serde_json::Value> = auction
            .bids
            .iter()
            .map(|b| {
                serde_json::json!({
                    "worker_id": b.worker_id,
                    "score": b.score,
                    "eta_secs": b.eta_secs,
                    "submitted_at_micros": b.submitted_at_micros,
                    "submitted_at": micros_to_rfc3339(b.submitted_at_micros),
                })
            })
            .collect();
        let envelope = serde_json::json!({
            "id": auction.task.id,
            "description": auction.task.description,
            "required_skills": auction.task.required_skills,
            "status": auction.status.as_str(),
            "opened_at": micros_to_rfc3339(auction.opened_at_micros),
            "closed_at": auction.closed_at_micros.map(micros_to_rfc3339),
            "winner": auction.winner.as_ref().map(|w| {
                serde_json::json!({
                    "worker_id": w.worker_id,
                    "score": w.score,
                    "eta_secs": w.eta_secs,
                })
            }),
            "bids": bids,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("auction: {}", auction.task.id);
        println!("status:  {}", auction.status.as_str());
        println!("opened:  {}", micros_to_rfc3339(auction.opened_at_micros));
        if let Some(c) = auction.closed_at_micros {
            println!("closed:  {}", micros_to_rfc3339(c));
        }
        println!("description: {}", auction.task.description);
        if !auction.task.required_skills.is_empty() {
            println!("required_skills: {}", auction.task.required_skills.join(", "));
        }
        if let Some(w) = &auction.winner {
            println!(
                "winner: {} (score={:.2}, eta={}s)",
                w.worker_id, w.score, w.eta_secs
            );
        } else {
            println!("winner: (none)");
        }
        println!("bids ({}):", auction.bids.len());
        for b in &auction.bids {
            println!(
                "  {} score={:.2} eta={}s @ {}",
                b.worker_id,
                b.score,
                b.eta_secs,
                micros_to_rfc3339(b.submitted_at_micros)
            );
        }
    }
    Ok(())
}
