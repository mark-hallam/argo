//! SQLite read paths + display helpers for auctions. Companion to
//! [`crate::auction_persist`] which owns schema + writes
//! (Stream `claude-skill-auction`).

use argo_types::{ArgoError, ArgoResult};
use chrono::{TimeZone, Utc};

use crate::auction::{Auction, AuctionBid, AuctionStatus, AuctionTask, AuctionWinner};
use crate::auction_persist::AuctionStore;

#[derive(sqlx::FromRow)]
struct AuctionRow {
    id: String,
    description: String,
    required_skills: String,
    status: String,
    opened_at_micros: i64,
    closed_at_micros: Option<i64>,
    winner_worker_id: Option<String>,
    winner_score: Option<f64>,
    winner_eta_secs: Option<i64>,
}

impl AuctionStore {
    /// List auctions, newest-first. Optional status filter.
    pub async fn list(
        &self,
        status: Option<AuctionStatus>,
        limit: u32,
    ) -> ArgoResult<Vec<Auction>> {
        self.ensure_schema().await?;
        let status_filter = status.map(|s| s.as_str()).unwrap_or("");
        let limit_i64 = i64::from(limit);
        let rows = sqlx::query_as::<_, AuctionRow>(
            "SELECT id, description, required_skills, status, opened_at_micros, \
             closed_at_micros, winner_worker_id, winner_score, winner_eta_secs \
             FROM auction_log \
             WHERE (?1 = '' OR status = ?1) \
             ORDER BY opened_at_micros DESC \
             LIMIT ?2",
        )
        .bind(status_filter)
        .bind(limit_i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list auctions: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.row_to_auction(row).await?);
        }
        Ok(out)
    }

    /// Fetch one auction by id. `None` if absent.
    pub async fn get(&self, id: &str) -> ArgoResult<Option<Auction>> {
        self.ensure_schema().await?;
        let rows = sqlx::query_as::<_, AuctionRow>(
            "SELECT id, description, required_skills, status, opened_at_micros, \
             closed_at_micros, winner_worker_id, winner_score, winner_eta_secs \
             FROM auction_log WHERE id = ?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("get auction: {e}")))?;
        match rows.into_iter().next() {
            Some(row) => Ok(Some(self.row_to_auction(row).await?)),
            None => Ok(None),
        }
    }

    /// List bids on one auction, in submission order.
    pub async fn list_bids(&self, auction_id: &str) -> ArgoResult<Vec<AuctionBid>> {
        self.ensure_schema().await?;
        let rows = sqlx::query_as::<_, (String, f64, i64, i64)>(
            "SELECT worker_id, score, eta_secs, submitted_at_micros \
             FROM auction_bid \
             WHERE auction_id = ? \
             ORDER BY submitted_at_micros ASC",
        )
        .bind(auction_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list bids: {e}")))?;
        Ok(rows
            .into_iter()
            .map(|(worker_id, score, eta_secs, submitted_at_micros)| AuctionBid {
                worker_id,
                score,
                eta_secs: u32::try_from(eta_secs).unwrap_or(u32::MAX),
                submitted_at_micros,
            })
            .collect())
    }

    async fn row_to_auction(&self, row: AuctionRow) -> ArgoResult<Auction> {
        let task = AuctionTask {
            id: row.id.clone(),
            description: row.description,
            required_skills: split_csv(&row.required_skills),
        };
        let winner = match (row.winner_worker_id, row.winner_score, row.winner_eta_secs) {
            (Some(w), Some(s), Some(e)) => Some(AuctionWinner {
                worker_id: w,
                score: s,
                eta_secs: u32::try_from(e).unwrap_or(u32::MAX),
            }),
            _ => None,
        };
        let bids = self.list_bids(&row.id).await?;
        Ok(Auction {
            task,
            bids,
            status: AuctionStatus::from_str_lossy(&row.status),
            opened_at_micros: row.opened_at_micros,
            closed_at_micros: row.closed_at_micros,
            winner,
        })
    }
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect()
}

/// Pretty-print microseconds as RFC-3339.
#[must_use]
pub fn micros_to_rfc3339(micros: i64) -> String {
    Utc.timestamp_micros(micros)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| micros.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auction::{Auction, AuctionBid, AuctionStatus, AuctionTask};
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

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
    async fn store_list_filters_by_status() {
        let pool = fresh_pool().await;
        let store = AuctionStore::new(pool);
        store.ensure_schema().await.unwrap();
        let mut closed = Auction::open(task("a-closed"));
        closed.submit_bid(bid("w1", 50.0, 30)).unwrap();
        closed.close();
        store.insert_auction(&closed).await.unwrap();
        let mut cancelled = Auction::open(task("a-cancelled"));
        cancelled.cancel();
        store.insert_auction(&cancelled).await.unwrap();
        let only_closed = store
            .list(Some(AuctionStatus::Closed), 100)
            .await
            .unwrap();
        assert_eq!(only_closed.len(), 1);
        assert_eq!(only_closed[0].task.id, "a-closed");
        let only_cancelled = store
            .list(Some(AuctionStatus::Cancelled), 100)
            .await
            .unwrap();
        assert_eq!(only_cancelled.len(), 1);
        let everything = store.list(None, 100).await.unwrap();
        assert_eq!(everything.len(), 2);
    }
}
