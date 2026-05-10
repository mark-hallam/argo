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
//! ## Hard rules
//!
//! - Bids on a `Closed` or `Cancelled` auction are rejected.
//! - `close()` always returns deterministically: highest score
//!   wins; ties resolve by lowest eta. The order in which bids
//!   were submitted does NOT affect the winner.
//! - Cancelled auctions never have a winner, even if bids exist.
//!
//! Schema + write paths live in [`crate::auction_persist`]; read
//! paths + display helpers in [`crate::auction_query`].

use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub use crate::auction_persist::AuctionStore;
pub use crate::auction_query::micros_to_rfc3339;

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

#[cfg(test)]
mod tests {
    use super::*;

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
