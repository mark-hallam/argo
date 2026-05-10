# Mission load-bearing wire — real-time tokio broadcast bidder loops for Auction

**Owner**: claude. **Depends on**: `argo-orchestrator::auction` SHIPPED (verified at `crates/argo-orchestrator/src/auction.rs`: `Auction`, `AuctionTask`, `AuctionBid`, `AuctionWinner`, `open`/`submit_bid`/`close`/`cancel` at lines 131/144/165/182); `claude-skill-auction` MERGED (#900, CLI-only path).
**Authored against**: origin/main `a309aede` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-auction-tokio-broadcast-bidder`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 item #55 ("Add real-time tokio broadcast bidder loops to Auction (currently CLI only — claude-skill-auction #900 SHIPPED)"). Today auctions open + accept bids only when an operator invokes `argo handoff auction` from the CLI. Mission Claim 2 ("delegation as unit") needs workers in-process to subscribe to open auctions, evaluate fit, and submit bids autonomously — that requires a tokio broadcast channel.

## Scope

Add a `AuctionDispatcher` that owns a `tokio::sync::broadcast::Sender<AuctionEvent>` and a paired async loop:

```rust
pub enum AuctionEvent {
    Opened(AuctionTask),
    BidReceived { task_id: Uuid, bid: AuctionBid },
    Closed { task_id: Uuid, winner: Option<AuctionWinner> },
    Cancelled(Uuid),
}

pub struct AuctionDispatcher {
    tx: broadcast::Sender<AuctionEvent>,
    state: Arc<RwLock<HashMap<Uuid, Auction>>>,
}

impl AuctionDispatcher {
    pub fn new(channel_capacity: usize) -> Self;
    pub fn subscribe(&self) -> broadcast::Receiver<AuctionEvent>;
    pub async fn open_auction(&self, task: AuctionTask) -> ArgoResult<Uuid>;
    pub async fn submit_bid(&self, task_id: Uuid, bid: AuctionBid) -> ArgoResult<()>;
    pub async fn close_auction(&self, task_id: Uuid) -> ArgoResult<Option<AuctionWinner>>;
    pub async fn cancel_auction(&self, task_id: Uuid) -> ArgoResult<()>;
}
```

Workers (consumers) `subscribe()` to receive `AuctionEvent::Opened(task)` notifications, evaluate fit (`task.required_skills` vs worker's `skill_set`), and call `submit_bid()` on a match. The dispatcher fans out `BidReceived`/`Closed`/`Cancelled` events so other observers (dashboard, audit) can react.

CLI path preserved: `argo handoff auction` now goes through `AuctionDispatcher` instead of directly calling `Auction::open`. Behavior byte-identical for single-operator workflow.

This is NOT in scope: changing `Auction`/`AuctionBid`/`AuctionTask` struct fields; persisting AuctionDispatcher state to DB (in-memory only for v1; restart loses live auctions); bid-evaluation strategy (workers' own logic, separate brief if needed); broadcast over network/multi-process (single-process tokio for v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-orchestrator/src/auction_dispatcher.rs` (NEW, ≤ 350 LOC) — `AuctionDispatcher` struct + methods; reuses existing `Auction` per-task state.
- `crates/argo-orchestrator/src/auction_dispatcher_tests.rs` (NEW, sibling) — 8 unit tests: subscribe-receives-opened; multiple-subscribers-each-receive; submit-bid-fans-out; close-emits-winner; close-emits-none-when-no-bids; cancel-fans-out; channel-capacity-backpressure-drops-old; concurrent-bids-no-deadlock.
- `crates/argo-orchestrator/src/lib.rs` — narrowly scoped: ONE `pub mod auction_dispatcher;` line + re-export `AuctionDispatcher`, `AuctionEvent`.
- `crates/argo-cli/src/handlers/handoff_auctions.rs` (or wherever `argo handoff auction` lives — verify in first 5min) — narrowly scoped: REPLACE direct `Auction::open` call with `AuctionDispatcher::open_auction`. NO behavior change visible to CLI users.
- `crates/argo-cli/tests/auction_dispatcher_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: spawn 2 mock-worker subscribers, open auction, assert both receive event, both submit bids, close emits winner.

## Files you must NOT touch

- `crates/argo-orchestrator/src/auction.rs` — `Auction`/`AuctionTask`/`AuctionBid`/`AuctionWinner` types are stable; reuse only (extending struct fields requires a separate brief)
- `crates/argo-orchestrator/src/auction_persist.rs` — persistence is upstream; v1 dispatcher is in-memory only
- Any non-orchestrator/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (in-memory only)

## Acceptance criteria

- [ ] `AuctionDispatcher` with `subscribe`/`open_auction`/`submit_bid`/`close_auction`/`cancel_auction` exists
- [ ] `AuctionEvent` enum with `Opened`/`BidReceived`/`Closed`/`Cancelled` variants
- [ ] `tokio::sync::broadcast::Sender` channel with configurable capacity (default 64)
- [ ] Multiple subscribers each receive every event (fan-out semantics)
- [ ] Lagged subscribers drop oldest events (broadcast backpressure semantics — explicit test)
- [ ] `argo handoff auction` CLI path routes through dispatcher; output byte-identical to today
- [ ] No persistence — restart loses live auctions (documented in module doc-comment)
- [ ] 8 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-orchestrator -p argo-cli` clean
- [ ] `cargo clippy -p argo-orchestrator -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-orchestrator -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-auction-tokio-broadcast-bidder`
- PR title: `orchestrator: AuctionDispatcher with tokio broadcast fan-out (mission-auction-tokio-broadcast-bidder)`

## Notes for the implementer

- The broadcast channel capacity of 64 is the brief's default — generous enough that subscribers running at human-reaction speed won't lag, conservative enough that a misbehaving subscriber doesn't balloon memory.
- Lagged-subscriber drop semantics are load-bearing: a slow worker that holds a receiver shouldn't block the dispatcher. The test for this is critical — verify with `tokio::time::pause()`-based determinism.
- This brief intentionally stops at fan-out. Workers' bid-evaluation logic (how they decide to bid) is a separate concern that lives in the worker side. For now, integration smoke uses mock workers that always bid on every task.
- Persistence is deliberately out of scope. Live auctions are ephemeral; if you need durable auctions, the existing `auction_persist` flow is still available for CLI-initiated auctions.
