//! `argo handoff-auctions cancel` handler.

use argo_orchestrator::{AuctionStatus, AuctionStore};
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
    let mut auction = store.get(id).await?.ok_or_else(|| {
        ArgoError::Provider(format!("auction '{id}' not found"))
    })?;
    let was_open = matches!(auction.status, AuctionStatus::Open);
    auction.cancel();
    if was_open {
        store.update_status(&auction).await?;
    }
    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "id": auction.task.id,
            "status": auction.status.as_str(),
            "was_open": was_open,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else if was_open {
        println!("cancelled auction {}", auction.task.id);
    } else {
        println!(
            "auction {} already {} — no change",
            auction.task.id,
            auction.status.as_str()
        );
    }
    Ok(())
}
