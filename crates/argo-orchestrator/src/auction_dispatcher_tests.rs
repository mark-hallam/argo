//! Unit tests for [`super::AuctionDispatcher`].
//!
//! Coverage matrix (8 tests, per brief):
//! 1. subscribe-receives-opened
//! 2. multiple-subscribers-each-receive
//! 3. submit-bid-fans-out
//! 4. close-emits-winner
//! 5. close-emits-none-when-no-bids
//! 6. cancel-fans-out
//! 7. channel-capacity-backpressure-drops-old
//! 8. concurrent-bids-no-deadlock

use std::sync::Arc;

use tokio::sync::broadcast::error::RecvError;

use super::{AuctionDispatcher, AuctionEvent, DEFAULT_CHANNEL_CAPACITY};
use crate::auction::{AuctionBid, AuctionTask};

fn task(id: &str) -> AuctionTask {
    AuctionTask {
        id: id.to_string(),
        description: "draft test work".into(),
        required_skills: vec!["alpha".into()],
    }
}

fn bid(worker: &str, score: f64, eta: u32) -> AuctionBid {
    AuctionBid {
        worker_id: worker.to_string(),
        score,
        eta_secs: eta,
        submitted_at_micros: 1_000_000,
    }
}

#[tokio::test]
async fn subscribe_receives_opened_event() {
    let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    let mut rx = dispatcher.subscribe();
    dispatcher.open_auction(task("a-open-1")).await.unwrap();

    match rx.recv().await.unwrap() {
        AuctionEvent::Opened(t) => assert_eq!(t.id, "a-open-1"),
        other => panic!("expected Opened, got {other:?}"),
    }
}

#[tokio::test]
async fn multiple_subscribers_each_receive_event() {
    let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    let mut rx_a = dispatcher.subscribe();
    let mut rx_b = dispatcher.subscribe();
    let mut rx_c = dispatcher.subscribe();
    assert_eq!(dispatcher.subscriber_count(), 3);

    dispatcher.open_auction(task("a-fanout")).await.unwrap();

    for rx in [&mut rx_a, &mut rx_b, &mut rx_c] {
        match rx.recv().await.unwrap() {
            AuctionEvent::Opened(t) => assert_eq!(t.id, "a-fanout"),
            other => panic!("subscriber missed Opened, saw {other:?}"),
        }
    }
}

#[tokio::test]
async fn submit_bid_fans_out_to_subscribers() {
    let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    let mut rx = dispatcher.subscribe();
    dispatcher.open_auction(task("a-bid")).await.unwrap();
    dispatcher
        .submit_bid("a-bid", bid("worker-1", 80.0, 30))
        .await
        .unwrap();

    // First event: Opened.
    assert!(matches!(rx.recv().await.unwrap(), AuctionEvent::Opened(_)));
    // Second event: BidReceived.
    match rx.recv().await.unwrap() {
        AuctionEvent::BidReceived { task_id, bid } => {
            assert_eq!(task_id, "a-bid");
            assert_eq!(bid.worker_id, "worker-1");
            assert_eq!(bid.score, 80.0);
        }
        other => panic!("expected BidReceived, got {other:?}"),
    }
}

#[tokio::test]
async fn close_emits_winner_event() {
    let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    let mut rx = dispatcher.subscribe();
    dispatcher.open_auction(task("a-close-win")).await.unwrap();
    dispatcher
        .submit_bid("a-close-win", bid("low", 50.0, 60))
        .await
        .unwrap();
    dispatcher
        .submit_bid("a-close-win", bid("high", 95.0, 30))
        .await
        .unwrap();
    let winner = dispatcher.close_auction("a-close-win").await.unwrap();
    assert_eq!(winner.as_ref().map(|w| w.worker_id.as_str()), Some("high"));

    // Drain Opened + 2 BidReceived events.
    for _ in 0..3 {
        let _ = rx.recv().await.unwrap();
    }
    match rx.recv().await.unwrap() {
        AuctionEvent::Closed { task_id, winner } => {
            assert_eq!(task_id, "a-close-win");
            assert_eq!(winner.unwrap().worker_id, "high");
        }
        other => panic!("expected Closed, got {other:?}"),
    }
}

#[tokio::test]
async fn close_emits_none_when_no_bids() {
    let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    let mut rx = dispatcher.subscribe();
    dispatcher.open_auction(task("a-empty")).await.unwrap();
    let winner = dispatcher.close_auction("a-empty").await.unwrap();
    assert!(winner.is_none());

    // Drain Opened.
    assert!(matches!(rx.recv().await.unwrap(), AuctionEvent::Opened(_)));
    match rx.recv().await.unwrap() {
        AuctionEvent::Closed { task_id, winner } => {
            assert_eq!(task_id, "a-empty");
            assert!(winner.is_none());
        }
        other => panic!("expected Closed, got {other:?}"),
    }
}

#[tokio::test]
async fn cancel_fans_out_to_subscribers() {
    let dispatcher = AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY);
    let mut rx = dispatcher.subscribe();
    dispatcher.open_auction(task("a-cancel")).await.unwrap();
    dispatcher.cancel_auction("a-cancel").await.unwrap();

    // Drain Opened.
    assert!(matches!(rx.recv().await.unwrap(), AuctionEvent::Opened(_)));
    match rx.recv().await.unwrap() {
        AuctionEvent::Cancelled(id) => assert_eq!(id, "a-cancel"),
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

#[tokio::test]
async fn channel_capacity_backpressure_drops_oldest() {
    // Capacity 2 + a single slow receiver that never drains. Open 4
    // auctions, then verify the receiver surfaces `Lagged(N)` and
    // skips the oldest events rather than blocking the dispatcher.
    let dispatcher = AuctionDispatcher::new(2);
    let mut rx = dispatcher.subscribe();
    for n in 0..4 {
        dispatcher
            .open_auction(task(&format!("a-back-{n}")))
            .await
            .unwrap();
    }

    // Receiver is lagged. tokio surfaces this on the next recv().
    let first = rx.recv().await;
    assert!(
        matches!(first, Err(RecvError::Lagged(skipped)) if skipped >= 1),
        "expected Lagged, got {first:?}",
    );
    // Subsequent recv()s yield the surviving (newest) events.
    let next = rx.recv().await.unwrap();
    match next {
        AuctionEvent::Opened(t) => assert!(
            t.id.starts_with("a-back-"),
            "expected a survived Opened, got id={}",
            t.id,
        ),
        other => panic!("expected Opened after Lagged, got {other:?}"),
    }
}

#[tokio::test]
async fn concurrent_bids_no_deadlock() {
    let dispatcher = Arc::new(AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY));
    dispatcher
        .open_auction(task("a-concurrent"))
        .await
        .unwrap();

    let mut handles = Vec::new();
    for i in 0..16 {
        let d = dispatcher.clone();
        handles.push(tokio::spawn(async move {
            d.submit_bid(
                "a-concurrent",
                bid(&format!("worker-{i}"), f64::from(i), 30),
            )
            .await
        }));
    }

    let results = futures_join_all(handles).await;
    for r in &results {
        assert!(r.is_ok(), "bid task failed: {r:?}");
    }
    let winner = dispatcher.close_auction("a-concurrent").await.unwrap();
    assert!(winner.is_some(), "expected a winner from 16 bids");
}

/// Tiny join-all that doesn't pull in the `futures` crate. Awaits
/// each handle in spawn order; sufficient for the 16-task test.
async fn futures_join_all<T>(
    handles: Vec<tokio::task::JoinHandle<T>>,
) -> Vec<Result<T, tokio::task::JoinError>> {
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        out.push(h.await);
    }
    out
}
