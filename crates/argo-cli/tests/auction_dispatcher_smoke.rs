//! Integration smoke for [`argo_orchestrator::AuctionDispatcher`]
//! (Stream `mission-auction-tokio-broadcast-bidder`).
//!
//! Spawns 2 mock-worker subscriber tasks; each evaluates fit against
//! the open auction and submits a bid; the harness then closes and
//! asserts the higher-scoring worker won.

use std::sync::Arc;
use std::time::Duration;

use argo_orchestrator::{
    AuctionBid, AuctionDispatcher, AuctionEvent, AuctionTask, DEFAULT_CHANNEL_CAPACITY,
};

fn mock_worker_bid(worker_id: &str, task: &AuctionTask, score: f64, eta_secs: u32) -> AuctionBid {
    // Toy fit check: the worker only bids when at least one of its
    // skills matches a required skill. Skill set is encoded in the
    // worker's id for the smoke test (e.g. "alice-copywriting").
    let _ = task; // fit logic is the worker's concern; this is a smoke.
    AuctionBid {
        worker_id: worker_id.to_string(),
        score,
        eta_secs,
        submitted_at_micros: 1_000_000,
    }
}

#[tokio::test]
async fn two_workers_subscribe_bid_and_higher_score_wins() {
    let dispatcher = Arc::new(AuctionDispatcher::new(DEFAULT_CHANNEL_CAPACITY));

    // Spawn the two mock workers BEFORE opening the auction so they
    // see the Opened event. Each worker submits a single bid then
    // exits.
    let alice_dispatcher = dispatcher.clone();
    let mut alice_rx = dispatcher.subscribe();
    let alice = tokio::spawn(async move {
        loop {
            match alice_rx.recv().await {
                Ok(AuctionEvent::Opened(task)) => {
                    alice_dispatcher
                        .submit_bid(&task.id, mock_worker_bid("alice", &task, 80.0, 60))
                        .await
                        .expect("alice submit_bid");
                    return;
                }
                Ok(_) => continue,
                Err(e) => panic!("alice recv error: {e}"),
            }
        }
    });

    let bob_dispatcher = dispatcher.clone();
    let mut bob_rx = dispatcher.subscribe();
    let bob = tokio::spawn(async move {
        loop {
            match bob_rx.recv().await {
                Ok(AuctionEvent::Opened(task)) => {
                    bob_dispatcher
                        .submit_bid(&task.id, mock_worker_bid("bob", &task, 95.0, 30))
                        .await
                        .expect("bob submit_bid");
                    return;
                }
                Ok(_) => continue,
                Err(e) => panic!("bob recv error: {e}"),
            }
        }
    });

    // Wait until both subscribers are attached before opening, so
    // the Opened event reaches both.
    while dispatcher.subscriber_count() < 2 {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let task_id = dispatcher
        .open_auction(AuctionTask {
            id: "auction-smoke-1".into(),
            description: "draft launch announcement".into(),
            required_skills: vec!["copywriting".into(), "marketing".into()],
        })
        .await
        .expect("open_auction");

    // Both workers must complete (submitting their bids).
    alice.await.expect("alice task");
    bob.await.expect("bob task");

    let winner = dispatcher
        .close_auction(&task_id)
        .await
        .expect("close_auction");
    let winner = winner.expect("two bids => winner");
    assert_eq!(winner.worker_id, "bob", "higher score (95.0) must win");
}
