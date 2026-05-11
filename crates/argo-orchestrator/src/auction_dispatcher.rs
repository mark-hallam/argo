//! Stream `mission-auction-tokio-broadcast-bidder` — load-bearing
//! wire from CLI-only auctions to in-process worker subscribers.
//!
//! The [`AuctionDispatcher`] owns a `tokio::sync::broadcast` channel
//! plus an in-memory map of live [`Auction`]s keyed by task id. Workers
//! `subscribe()` to receive [`AuctionEvent`]s as auctions open, take
//! bids, close, or cancel.
//!
//! Persistence is intentionally out of scope: live auctions are
//! ephemeral and lost on restart. Durable auctions still flow through
//! the existing [`crate::auction_persist::AuctionStore`] path.
//!
//! Backpressure follows tokio broadcast semantics: lagged subscribers
//! drop the oldest events (and surface `RecvError::Lagged(n)` on the
//! next `recv()`); the dispatcher does not block.

use std::collections::HashMap;
use std::sync::Arc;

use argo_types::{ArgoError, ArgoResult};
use tokio::sync::{broadcast, RwLock};

use crate::auction::{Auction, AuctionBid, AuctionTask, AuctionWinner};

/// Default broadcast channel capacity. Generous enough that
/// human-reaction-speed subscribers won't lag, conservative enough
/// that a misbehaving subscriber doesn't balloon memory.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 64;

/// Events fanned out to every subscriber.
///
/// `Clone` is required by `tokio::sync::broadcast`; every variant's
/// inner types already derive `Clone`.
#[derive(Debug, Clone)]
pub enum AuctionEvent {
    /// A new auction was opened. Carries the full task so subscribers
    /// can evaluate fit without an extra round-trip.
    Opened(AuctionTask),
    /// A bid landed on a live auction.
    BidReceived {
        /// Task id (matches `AuctionTask::id`).
        task_id: String,
        /// The bid as accepted.
        bid: AuctionBid,
    },
    /// An auction closed. `winner` is `None` when there were no bids.
    Closed {
        /// Task id (matches `AuctionTask::id`).
        task_id: String,
        /// Winning worker, if any.
        winner: Option<AuctionWinner>,
    },
    /// An auction was cancelled by the operator (no winner).
    Cancelled(String),
}

/// Fan-out wrapper around per-task [`Auction`] state.
///
/// Note: The brief sketched `Uuid` task ids, but `AuctionTask::id` is
/// already a `String` (caller-supplied, typically `auction-<uuid>`).
/// Using `String` keeps the dispatcher symmetric with the existing
/// type without forcing a struct-level change to `AuctionTask`.
#[derive(Debug)]
pub struct AuctionDispatcher {
    tx: broadcast::Sender<AuctionEvent>,
    state: Arc<RwLock<HashMap<String, Auction>>>,
}

impl AuctionDispatcher {
    /// Construct a dispatcher with the given broadcast channel
    /// capacity. Use [`DEFAULT_CHANNEL_CAPACITY`] (64) for the
    /// suggested default.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_orchestrator::{AuctionDispatcher, DEFAULT_CHANNEL_CAPACITY};
    /// let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    /// let _rx = dispatcher.subscribe();
    /// ```
    #[must_use]
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _initial_rx) = broadcast::channel(channel_capacity.max(1));
        Self {
            tx,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to the event stream. Returned receivers see only
    /// events emitted *after* the call (broadcast semantics — no
    /// replay of historical events).
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AuctionEvent> {
        self.tx.subscribe()
    }

    /// Current subscriber count. Useful for tests that want to wait
    /// until N receivers are attached before opening an auction.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Open a fresh auction. Returns the task id (echoed from
    /// `task.id`) so callers can pass it into subsequent
    /// [`Self::submit_bid`] / [`Self::close_auction`] calls without
    /// re-deriving it. Errors when an auction with the same id is
    /// already live.
    pub async fn open_auction(&self, task: AuctionTask) -> ArgoResult<String> {
        let task_id = task.id.clone();
        {
            let mut state = self.state.write().await;
            if state.contains_key(&task_id) {
                return Err(ArgoError::Provider(format!(
                    "auction {task_id} already live in dispatcher"
                )));
            }
            state.insert(task_id.clone(), Auction::open(task.clone()));
        }
        let _ = self.tx.send(AuctionEvent::Opened(task));
        Ok(task_id)
    }

    /// Submit a bid against a live auction. Errors propagate from
    /// [`Auction::submit_bid`] (auction not open, duplicate worker).
    /// Errors when no live auction exists for the id.
    pub async fn submit_bid(&self, task_id: &str, bid: AuctionBid) -> ArgoResult<()> {
        let bid_clone = bid.clone();
        {
            let mut state = self.state.write().await;
            let auction = state.get_mut(task_id).ok_or_else(|| {
                ArgoError::Provider(format!("auction {task_id} not live in dispatcher"))
            })?;
            auction.submit_bid(bid)?;
        }
        let _ = self.tx.send(AuctionEvent::BidReceived {
            task_id: task_id.to_string(),
            bid: bid_clone,
        });
        Ok(())
    }

    /// Close a live auction, fan out the result, and drop it from
    /// the dispatcher's in-memory map. Returns the winner (if any).
    /// Errors when no live auction exists for the id.
    pub async fn close_auction(&self, task_id: &str) -> ArgoResult<Option<AuctionWinner>> {
        let winner = {
            let mut state = self.state.write().await;
            let mut auction = state.remove(task_id).ok_or_else(|| {
                ArgoError::Provider(format!("auction {task_id} not live in dispatcher"))
            })?;
            auction.close()
        };
        let _ = self.tx.send(AuctionEvent::Closed {
            task_id: task_id.to_string(),
            winner: winner.clone(),
        });
        Ok(winner)
    }

    /// Cancel a live auction (no winner) and drop it. Idempotent
    /// only at the [`Auction::cancel`] level; the dispatcher itself
    /// errors when the id is unknown so callers can detect typos.
    pub async fn cancel_auction(&self, task_id: &str) -> ArgoResult<()> {
        {
            let mut state = self.state.write().await;
            let mut auction = state.remove(task_id).ok_or_else(|| {
                ArgoError::Provider(format!("auction {task_id} not live in dispatcher"))
            })?;
            auction.cancel();
        }
        let _ = self.tx.send(AuctionEvent::Cancelled(task_id.to_string()));
        Ok(())
    }
}

#[cfg(test)]
#[path = "auction_dispatcher_tests.rs"]
mod dispatcher_tests;
