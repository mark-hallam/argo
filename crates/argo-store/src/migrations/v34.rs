//! Schema migration v34 — handoff fallback auction audit trail.
//!
//! Reconciles raw SQL migration `27__handoff_auctions.sql` (Stream
//! `claude-skill-auction`). Two tables — `auction_log` persists each
//! auction's header (task description, lifecycle, winner), and
//! `auction_bid` persists every bid for replay.
//!
//! 27-collision split: this slot pairs with v35 below
//! (27__spend_anomaly). Brief assigns handoff_auctions → v34,
//! spend_anomaly → v35, consistent with the published-contract
//! ordering used for prior collision groups (see v31 doc comment).
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops the
//! two tables and their indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V34_UP {
        exec(tx, "v34 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V34_DOWN {
        exec(tx, "v34 down", sql).await?;
    }
    Ok(())
}

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
    Ok(())
}

const V34_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS auction_log (
        id TEXT PRIMARY KEY,
        description TEXT NOT NULL,
        required_skills TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL,
        opened_at_micros INTEGER NOT NULL,
        closed_at_micros INTEGER,
        winner_worker_id TEXT,
        winner_score REAL,
        winner_eta_secs INTEGER
    )",
    "CREATE INDEX IF NOT EXISTS idx_auction_log_status
     ON auction_log(status)",
    "CREATE INDEX IF NOT EXISTS idx_auction_log_opened
     ON auction_log(opened_at_micros DESC)",
    "CREATE TABLE IF NOT EXISTS auction_bid (
        auction_id TEXT NOT NULL,
        worker_id TEXT NOT NULL,
        score REAL NOT NULL,
        eta_secs INTEGER NOT NULL,
        submitted_at_micros INTEGER NOT NULL,
        PRIMARY KEY (auction_id, worker_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_auction_bid_auction
     ON auction_bid(auction_id)",
];

const V34_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_auction_bid_auction",
    "DROP TABLE IF EXISTS auction_bid",
    "DROP INDEX IF EXISTS idx_auction_log_opened",
    "DROP INDEX IF EXISTS idx_auction_log_status",
    "DROP TABLE IF EXISTS auction_log",
];
