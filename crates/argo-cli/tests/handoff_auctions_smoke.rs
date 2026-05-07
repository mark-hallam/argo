//! End-to-end smoke for `argo handoff-auctions list/show/cancel`
//! (Stream `claude-skill-auction`). Seeds three bids by going
//! through the auction primitives directly, then exercises the
//! list/show/cancel CLI surface.

use argo_orchestrator::{Auction, AuctionBid, AuctionStore, AuctionTask};
use argo_store::SessionStore;
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn make_bid(worker: &str, score: f64, eta: u32) -> AuctionBid {
    AuctionBid {
        worker_id: worker.to_string(),
        score,
        eta_secs: eta,
        submitted_at_micros: 1_000_000,
    }
}

#[tokio::test]
async fn three_workers_bid_highest_score_wins() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    // Seed an auction with 3 bids directly via the AuctionStore.
    // The CLI handlers re-open the same DB from $ARGO_HOME.
    let db = home.path().join("sessions.db");
    let store = SessionStore::open(&db).await.unwrap();
    store.migrate().await.unwrap();
    let auction_store = AuctionStore::new(store.pool().clone());
    auction_store.ensure_schema().await.unwrap();

    let mut auction = Auction::open(AuctionTask {
        id: "auction-test-1".into(),
        description: "draft launch announcement".into(),
        required_skills: vec!["copywriting".into(), "marketing".into()],
    });
    auction.submit_bid(make_bid("worker-alice", 80.0, 60)).unwrap();
    auction.submit_bid(make_bid("worker-bob", 95.0, 30)).unwrap();
    auction.submit_bid(make_bid("worker-carol", 70.0, 45)).unwrap();
    let winner = auction.close().expect("3 bids ⇒ winner");
    assert_eq!(winner.worker_id, "worker-bob");

    auction_store.insert_auction(&auction).await.unwrap();
    for b in &auction.bids {
        auction_store.insert_bid(&auction.task.id, b).await.unwrap();
    }

    // List should include this auction.
    let list = run(
        workspace.path(),
        home.path(),
        &["handoff-auctions", "list", "--json"],
    );
    assert_ok(&list, "handoff-auctions list");
    let arr: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);
    assert_eq!(arr[0]["id"], "auction-test-1");
    assert_eq!(arr[0]["status"], "closed");
    assert_eq!(arr[0]["bid_count"], 3);
    assert_eq!(arr[0]["winner_worker_id"], "worker-bob");

    // Show should render bids in submission order with the winner.
    let show = run(
        workspace.path(),
        home.path(),
        &["handoff-auctions", "show", "auction-test-1", "--json"],
    );
    assert_ok(&show, "handoff-auctions show");
    let obj: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(obj["status"], "closed");
    assert_eq!(obj["winner"]["worker_id"], "worker-bob");
    let bids = obj["bids"].as_array().unwrap();
    assert_eq!(bids.len(), 3);

    // Cancel on a closed auction should be a no-op (idempotent).
    let cancel = run(
        workspace.path(),
        home.path(),
        &["handoff-auctions", "cancel", "auction-test-1", "--json"],
    );
    assert_ok(&cancel, "handoff-auctions cancel (closed → no-op)");
    let cobj: serde_json::Value = serde_json::from_slice(&cancel.stdout).unwrap();
    assert_eq!(cobj["was_open"], false);
    assert_eq!(cobj["status"], "closed");

    // Open a fresh auction and verify cancel flips status.
    let mut open_auction = Auction::open(AuctionTask {
        id: "auction-test-cancel".into(),
        description: "stays open".into(),
        required_skills: vec![],
    });
    open_auction.submit_bid(make_bid("w-1", 50.0, 60)).unwrap();
    auction_store.insert_auction(&open_auction).await.unwrap();
    for b in &open_auction.bids {
        auction_store
            .insert_bid(&open_auction.task.id, b)
            .await
            .unwrap();
    }

    let cancel2 = run(
        workspace.path(),
        home.path(),
        &[
            "handoff-auctions",
            "cancel",
            "auction-test-cancel",
            "--json",
        ],
    );
    assert_ok(&cancel2, "handoff-auctions cancel (open → cancelled)");
    let cobj2: serde_json::Value = serde_json::from_slice(&cancel2.stdout).unwrap();
    assert_eq!(cobj2["was_open"], true);
    assert_eq!(cobj2["status"], "cancelled");

    // Filter list by status=cancelled.
    let cancelled_list = run(
        workspace.path(),
        home.path(),
        &[
            "handoff-auctions",
            "list",
            "--status",
            "cancelled",
            "--json",
        ],
    );
    assert_ok(&cancelled_list, "handoff-auctions list --status cancelled");
    let arr: serde_json::Value = serde_json::from_slice(&cancelled_list.stdout).unwrap();
    assert_eq!(arr.as_array().map_or(0, Vec::len), 1);
    assert_eq!(arr[0]["id"], "auction-test-cancel");
}

#[test]
fn show_returns_error_on_unknown_id() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["handoff-auctions", "show", "nope-not-here"],
    );
    assert!(!bad.status.success(), "expected non-zero on missing id");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("not found"), "expected helpful error: {stderr}");
}
