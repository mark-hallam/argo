//! SQLite persistence for auctions — schema bootstrap and write
//! paths. Read paths live in [`crate::auction_query`]
//! (Stream `claude-skill-auction`).

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

use crate::auction::{Auction, AuctionBid};

pub(crate) const SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/27__handoff_auctions.sql");

/// Persistent auction store (sqlite). Idempotent schema bootstrap.
#[derive(Clone)]
pub struct AuctionStore {
    pub(crate) pool: SqlitePool,
}

impl std::fmt::Debug for AuctionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuctionStore").finish()
    }
}

impl AuctionStore {
    /// Build over a sqlite pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Apply the schema. Idempotent.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in SCHEMA_SQL.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(&self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("auction schema: {e}")))?;
        }
        Ok(())
    }

    /// Persist a fresh open auction. The bids are written
    /// separately via [`Self::insert_bid`].
    pub async fn insert_auction(&self, auction: &Auction) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let required = auction.task.required_skills.join(",");
        sqlx::query(
            "INSERT INTO auction_log \
             (id, description, required_skills, status, opened_at_micros, \
              closed_at_micros, winner_worker_id, winner_score, winner_eta_secs) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&auction.task.id)
        .bind(&auction.task.description)
        .bind(&required)
        .bind(auction.status.as_str())
        .bind(auction.opened_at_micros)
        .bind(auction.closed_at_micros)
        .bind(auction.winner.as_ref().map(|w| w.worker_id.clone()))
        .bind(auction.winner.as_ref().map(|w| w.score))
        .bind(auction.winner.as_ref().map(|w| i64::from(w.eta_secs)))
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert auction: {e}")))?;
        Ok(())
    }

    /// Insert one bid against an existing auction.
    pub async fn insert_bid(&self, auction_id: &str, bid: &AuctionBid) -> ArgoResult<()> {
        sqlx::query(
            "INSERT INTO auction_bid \
             (auction_id, worker_id, score, eta_secs, submitted_at_micros) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(auction_id)
        .bind(&bid.worker_id)
        .bind(bid.score)
        .bind(i64::from(bid.eta_secs))
        .bind(bid.submitted_at_micros)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert bid: {e}")))?;
        Ok(())
    }

    /// Update an auction's status + winner after `close()` /
    /// `cancel()`. Used by the CLI's cancel handler.
    pub async fn update_status(&self, auction: &Auction) -> ArgoResult<()> {
        sqlx::query(
            "UPDATE auction_log SET status = ?, closed_at_micros = ?, \
             winner_worker_id = ?, winner_score = ?, winner_eta_secs = ? \
             WHERE id = ?",
        )
        .bind(auction.status.as_str())
        .bind(auction.closed_at_micros)
        .bind(auction.winner.as_ref().map(|w| w.worker_id.clone()))
        .bind(auction.winner.as_ref().map(|w| w.score))
        .bind(auction.winner.as_ref().map(|w| i64::from(w.eta_secs)))
        .bind(&auction.task.id)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("update status: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auction::{Auction, AuctionBid, AuctionStatus, AuctionTask};
    use sqlx::sqlite::SqlitePoolOptions;

    fn bid(worker: &str, score: f64, eta: u32) -> AuctionBid {
        AuctionBid {
            worker_id: worker.to_string(),
            score,
            eta_secs: eta,
            submitted_at_micros: 0,
        }
    }

    fn task(id: &str) -> AuctionTask {
        AuctionTask {
            id: id.to_string(),
            description: "test task".into(),
            required_skills: vec!["alpha".into(), "beta".into()],
        }
    }

    async fn fresh_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn store_round_trips_auction_with_bids() {
        let pool = fresh_pool().await;
        let store = AuctionStore::new(pool);
        store.ensure_schema().await.unwrap();
        let mut auction = Auction::open(task("a-store-1"));
        auction.submit_bid(bid("w1", 80.0, 60)).unwrap();
        auction.submit_bid(bid("w2", 90.0, 30)).unwrap();
        auction.close();
        store.insert_auction(&auction).await.unwrap();
        for b in &auction.bids {
            store.insert_bid(&auction.task.id, b).await.unwrap();
        }
        let fetched = store.get(&auction.task.id).await.unwrap().unwrap();
        assert_eq!(fetched.task.id, "a-store-1");
        assert_eq!(fetched.status, AuctionStatus::Closed);
        assert_eq!(fetched.winner.as_ref().unwrap().worker_id, "w2");
        assert_eq!(fetched.bids.len(), 2);
    }

    #[tokio::test]
    async fn store_update_status_persists_cancel() {
        let pool = fresh_pool().await;
        let store = AuctionStore::new(pool);
        store.ensure_schema().await.unwrap();
        let mut auction = Auction::open(task("a-cancel-update"));
        store.insert_auction(&auction).await.unwrap();
        auction.cancel();
        store.update_status(&auction).await.unwrap();
        let fetched = store.get(&auction.task.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, AuctionStatus::Cancelled);
    }
}
