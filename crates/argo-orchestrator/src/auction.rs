//! Stream `claude-skill-auction` — handoff fallback when no
//! routing rule matches. Workers bid for the work based on skill
//! fit + trust + quality; highest score wins, ties broken by
//! lowest eta.
//!
//! v1 ships the **primitives + persistence + CLI**: auction
//! lifecycle (open → bid → close), scoring formula, and an audit
//! trail in `auction_log` + `auction_bid` tables. The walker
//! integration is a thin hookup: when the static escalation chain
//! exhausts, the walker may consult an [`AuctionStore`] to record
//! that a fallback auction was attempted before falling through to
//! the existing human-operator notification.
//!
//! Real-time tokio broadcasting + worker-side bidder loops are
//! intentionally a follow-up — v1's tests exercise the
//! deterministic in-memory path that's reachable without async
//! orchestration so coverage is hermetic.
//!
//! ## Hard rules
//!
//! - Bids on a `Closed` or `Cancelled` auction are rejected.
//! - `close()` always returns deterministically: highest score
//!   wins; ties resolve by lowest eta. The order in which bids
//!   were submitted does NOT affect the winner.
//! - Cancelled auctions never have a winner, even if bids exist.

use argo_types::{ArgoError, ArgoResult};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

const SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/27__handoff_auctions.sql");

/// One auction task — the description handed off to bidders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuctionTask {
    /// Unique id. Caller-supplied (typically `auction-<uuid>`).
    pub id: String,
    /// Free-form description of the work.
    pub description: String,
    /// Skill names the work requires (used for fit scoring).
    pub required_skills: Vec<String>,
}

/// A worker's bid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuctionBid {
    /// Bidder worker id.
    pub worker_id: String,
    /// Computed fit score (caller decides the formula; see
    /// [`fit_score`] for the v1 default).
    pub score: f64,
    /// Estimated time to complete, in seconds. Used as the
    /// tie-breaker when scores match.
    pub eta_secs: u32,
    /// Microseconds since epoch when the bid was submitted.
    pub submitted_at_micros: i64,
}

/// Auction lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuctionStatus {
    /// Bids may still be submitted.
    Open,
    /// `close()` was called; the winner (if any) is recorded.
    Closed,
    /// Operator-cancelled. Never produces a winner.
    Cancelled,
    /// Closed with zero bids. Distinct from `Closed` so the audit
    /// trail can call out fallthroughs.
    NoBidders,
}

impl AuctionStatus {
    /// String form persisted in sqlite + emitted by JSON output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::NoBidders => "no_bidders",
        }
    }

    /// Parse from sqlite text.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "open" => Self::Open,
            "closed" => Self::Closed,
            "cancelled" => Self::Cancelled,
            "no_bidders" => Self::NoBidders,
            _ => Self::Open,
        }
    }
}

/// Winner of a closed auction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuctionWinner {
    /// Worker id with the highest score (lowest eta on ties).
    pub worker_id: String,
    /// Winning score.
    pub score: f64,
    /// Winning eta.
    pub eta_secs: u32,
}

/// In-memory auction state. Persistence is the [`AuctionStore`]
/// wrapper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auction {
    /// Task description. The id here is the auction id.
    pub task: AuctionTask,
    /// All bids, in submission order.
    pub bids: Vec<AuctionBid>,
    /// Lifecycle.
    pub status: AuctionStatus,
    /// Microseconds since epoch when the auction opened.
    pub opened_at_micros: i64,
    /// Microseconds since epoch when the auction closed (or was
    /// cancelled). `None` while open.
    pub closed_at_micros: Option<i64>,
    /// Resolved winner, if any.
    pub winner: Option<AuctionWinner>,
}

impl Auction {
    /// Open a fresh auction.
    #[must_use]
    pub fn open(task: AuctionTask) -> Self {
        Self {
            task,
            bids: Vec::new(),
            status: AuctionStatus::Open,
            opened_at_micros: Utc::now().timestamp_micros(),
            closed_at_micros: None,
            winner: None,
        }
    }

    /// Submit a bid. Errors if the auction is no longer open OR
    /// the worker has already bid.
    pub fn submit_bid(&mut self, bid: AuctionBid) -> ArgoResult<()> {
        if !matches!(self.status, AuctionStatus::Open) {
            return Err(ArgoError::Provider(format!(
                "auction {} is {} — bids rejected",
                self.task.id,
                self.status.as_str()
            )));
        }
        if self.bids.iter().any(|b| b.worker_id == bid.worker_id) {
            return Err(ArgoError::Provider(format!(
                "worker {} already bid on auction {}",
                bid.worker_id, self.task.id
            )));
        }
        self.bids.push(bid);
        Ok(())
    }

    /// Close the auction. Returns the winner if any bids were
    /// submitted. Updates [`Self::status`] + [`Self::winner`] +
    /// [`Self::closed_at_micros`].
    pub fn close(&mut self) -> Option<AuctionWinner> {
        if !matches!(self.status, AuctionStatus::Open) {
            return self.winner.clone();
        }
        let winner = pick_winner(&self.bids);
        self.status = if winner.is_some() {
            AuctionStatus::Closed
        } else {
            AuctionStatus::NoBidders
        };
        self.closed_at_micros = Some(Utc::now().timestamp_micros());
        self.winner = winner.clone();
        winner
    }

    /// Operator-cancel an open auction. Idempotent on already-
    /// cancelled / closed / no-bidders auctions (no-op).
    pub fn cancel(&mut self) {
        if matches!(self.status, AuctionStatus::Open) {
            self.status = AuctionStatus::Cancelled;
            self.closed_at_micros = Some(Utc::now().timestamp_micros());
        }
    }
}

/// Tie-broken winner picker. Highest score wins; ties fall to
/// lowest eta; ties on both fall to lexicographically smallest
/// worker_id (deterministic for tests).
fn pick_winner(bids: &[AuctionBid]) -> Option<AuctionWinner> {
    bids.iter()
        .min_by(|a, b| {
            // Reverse-compare on score (highest first), then eta,
            // then worker_id.
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.eta_secs.cmp(&b.eta_secs))
                .then(a.worker_id.cmp(&b.worker_id))
        })
        .map(|b| AuctionWinner {
            worker_id: b.worker_id.clone(),
            score: b.score,
            eta_secs: b.eta_secs,
        })
}

/// V1 fit-score formula. Larger = better. Returns `0.0` when
/// `total_required` is zero (no skills required ⇒ any worker
/// scores zero, walker falls back to NoBidders + chain).
///
/// `trust_factor` is a 0..=1 scalar (caller maps `TrustGrade` to
/// a number — e.g. Verified=1.0, Reviewed=0.7, Unverified=0.3).
/// `quality` is the 0..=100 score from skill-quality-judge.
#[must_use]
pub fn fit_score(
    matched_skills: u32,
    total_required: u32,
    trust_factor: f64,
    quality: u8,
) -> f64 {
    if total_required == 0 {
        return 0.0;
    }
    let coverage = f64::from(matched_skills) / f64::from(total_required);
    let q = f64::from(quality) / 100.0;
    coverage * trust_factor * q * 100.0
}

/// Persistent auction store (sqlite). Idempotent schema bootstrap.
#[derive(Clone)]
pub struct AuctionStore {
    pool: SqlitePool,
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

    /// List auctions, newest-first. Optional status filter.
    pub async fn list(
        &self,
        status: Option<AuctionStatus>,
        limit: u32,
    ) -> ArgoResult<Vec<Auction>> {
        self.ensure_schema().await?;
        let status_filter = status.map(|s| s.as_str()).unwrap_or("");
        let limit_i64 = i64::from(limit);
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i64,
                Option<i64>,
                Option<String>,
                Option<f64>,
                Option<i64>,
            ),
        >(
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
        for (id, description, skills_csv, status, opened_at, closed_at, ww, ws, we) in rows {
            let task = AuctionTask {
                id: id.clone(),
                description,
                required_skills: split_csv(&skills_csv),
            };
            let winner = match (ww, ws, we) {
                (Some(w), Some(s), Some(e)) => Some(AuctionWinner {
                    worker_id: w,
                    score: s,
                    eta_secs: u32::try_from(e).unwrap_or(u32::MAX),
                }),
                _ => None,
            };
            let bids = self.list_bids(&id).await?;
            out.push(Auction {
                task,
                bids,
                status: AuctionStatus::from_str_lossy(&status),
                opened_at_micros: opened_at,
                closed_at_micros: closed_at,
                winner,
            });
        }
        Ok(out)
    }

    /// Fetch one auction by id. `None` if absent.
    pub async fn get(&self, id: &str) -> ArgoResult<Option<Auction>> {
        let mut rows = self
            .list_filtered_by_id(id)
            .await?;
        Ok(rows.pop())
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

    async fn list_filtered_by_id(&self, id: &str) -> ArgoResult<Vec<Auction>> {
        self.ensure_schema().await?;
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i64,
                Option<i64>,
                Option<String>,
                Option<f64>,
                Option<i64>,
            ),
        >(
            "SELECT id, description, required_skills, status, opened_at_micros, \
             closed_at_micros, winner_worker_id, winner_score, winner_eta_secs \
             FROM auction_log WHERE id = ?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("get auction: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (auction_id, description, skills_csv, status, opened_at, closed_at, ww, ws, we) in rows
        {
            let task = AuctionTask {
                id: auction_id.clone(),
                description,
                required_skills: split_csv(&skills_csv),
            };
            let winner = match (ww, ws, we) {
                (Some(w), Some(s), Some(e)) => Some(AuctionWinner {
                    worker_id: w,
                    score: s,
                    eta_secs: u32::try_from(e).unwrap_or(u32::MAX),
                }),
                _ => None,
            };
            let bids = self.list_bids(&auction_id).await?;
            out.push(Auction {
                task,
                bids,
                status: AuctionStatus::from_str_lossy(&status),
                opened_at_micros: opened_at,
                closed_at_micros: closed_at,
                winner,
            });
        }
        Ok(out)
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

    #[test]
    fn pick_winner_returns_highest_score() {
        let bids = vec![bid("w1", 50.0, 60), bid("w2", 90.0, 30), bid("w3", 70.0, 60)];
        let winner = pick_winner(&bids).unwrap();
        assert_eq!(winner.worker_id, "w2");
        assert_eq!(winner.score, 90.0);
    }

    #[test]
    fn pick_winner_breaks_ties_by_eta() {
        let bids = vec![bid("w1", 80.0, 100), bid("w2", 80.0, 30)];
        let winner = pick_winner(&bids).unwrap();
        assert_eq!(winner.worker_id, "w2");
    }

    #[test]
    fn pick_winner_breaks_double_ties_by_worker_id() {
        let bids = vec![bid("zeta", 50.0, 60), bid("alpha", 50.0, 60)];
        let winner = pick_winner(&bids).unwrap();
        assert_eq!(winner.worker_id, "alpha");
    }

    #[test]
    fn pick_winner_returns_none_when_no_bids() {
        let bids: Vec<AuctionBid> = Vec::new();
        assert!(pick_winner(&bids).is_none());
    }

    #[test]
    fn fit_score_zero_when_no_skills_required() {
        assert_eq!(fit_score(0, 0, 1.0, 100), 0.0);
    }

    #[test]
    fn fit_score_full_match_full_trust_full_quality_is_100() {
        assert!((fit_score(2, 2, 1.0, 100) - 100.0).abs() < 0.001);
    }

    #[test]
    fn fit_score_partial_coverage_scales_proportionally() {
        let half = fit_score(1, 2, 1.0, 100);
        assert!((half - 50.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn submit_bid_rejects_after_close() {
        let mut auction = Auction::open(task("a-1"));
        auction.submit_bid(bid("w1", 80.0, 30)).unwrap();
        auction.close();
        let err = auction.submit_bid(bid("w2", 90.0, 20)).unwrap_err();
        assert!(format!("{err}").contains("closed"));
    }

    #[tokio::test]
    async fn submit_bid_rejects_duplicate_worker() {
        let mut auction = Auction::open(task("a-1"));
        auction.submit_bid(bid("w1", 80.0, 30)).unwrap();
        let err = auction.submit_bid(bid("w1", 90.0, 25)).unwrap_err();
        assert!(format!("{err}").contains("already bid"));
    }

    #[tokio::test]
    async fn close_with_no_bids_returns_none_and_marks_no_bidders() {
        let mut auction = Auction::open(task("a-1"));
        let winner = auction.close();
        assert!(winner.is_none());
        assert_eq!(auction.status, AuctionStatus::NoBidders);
        assert!(auction.closed_at_micros.is_some());
    }

    #[tokio::test]
    async fn cancel_marks_status_and_skips_winner() {
        let mut auction = Auction::open(task("a-1"));
        auction.submit_bid(bid("w1", 90.0, 30)).unwrap();
        auction.cancel();
        assert_eq!(auction.status, AuctionStatus::Cancelled);
        assert!(auction.winner.is_none());
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

    #[test]
    fn auction_status_string_round_trips() {
        for s in [
            AuctionStatus::Open,
            AuctionStatus::Closed,
            AuctionStatus::Cancelled,
            AuctionStatus::NoBidders,
        ] {
            assert_eq!(AuctionStatus::from_str_lossy(s.as_str()), s);
        }
        assert_eq!(
            AuctionStatus::from_str_lossy("unknown-junk"),
            AuctionStatus::Open
        );
    }
}
