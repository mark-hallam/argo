-- Stream `claude-skill-auction` — handoff fallback auction
-- audit trail. Two tables — auction_log persists each auction's
-- header (task description, lifecycle, winner), and auction_bid
-- persists every bid for replay.
--
-- SAFETY: auction ids are deployment-local orchestration ids and bids
-- use deployment-unique worker ids. Tenant-facing handoff statistics
-- are served from tenant-scoped session joins. This replay log is not
-- exposed as a cross-tenant row list.

CREATE TABLE IF NOT EXISTS auction_log (
    -- Caller-supplied auction id.
    id TEXT PRIMARY KEY,
    -- Free-form task description shown to bidders.
    description TEXT NOT NULL,
    -- Comma-separated required-skill names.
    required_skills TEXT NOT NULL DEFAULT '',
    -- Lifecycle: open / closed / cancelled / no_bidders.
    status TEXT NOT NULL,
    -- Microseconds since epoch when opened.
    opened_at_micros INTEGER NOT NULL,
    -- Microseconds since epoch when closed or cancelled (NULL
    -- while open).
    closed_at_micros INTEGER,
    -- Winner worker id (NULL on no_bidders / cancelled).
    winner_worker_id TEXT,
    -- Winning score (NULL on no_bidders / cancelled).
    winner_score REAL,
    -- Winning eta_secs (NULL on no_bidders / cancelled).
    winner_eta_secs INTEGER
);

CREATE INDEX IF NOT EXISTS idx_auction_log_status
    ON auction_log(status);

CREATE INDEX IF NOT EXISTS idx_auction_log_opened
    ON auction_log(opened_at_micros DESC);

CREATE TABLE IF NOT EXISTS auction_bid (
    auction_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    score REAL NOT NULL,
    eta_secs INTEGER NOT NULL,
    submitted_at_micros INTEGER NOT NULL,
    PRIMARY KEY (auction_id, worker_id)
);

CREATE INDEX IF NOT EXISTS idx_auction_bid_auction
    ON auction_bid(auction_id);
