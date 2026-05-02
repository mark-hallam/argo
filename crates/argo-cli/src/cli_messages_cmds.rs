//! `argo messages …` — workspace-wide message rollups. Lifted out
//! of `cli_session_cmds.rs` once that file went over the 500-LOC
//! budget. Parallel to `argo audit` but scoped to the messages
//! table. Re-exported via `cli.rs`'s wildcard glob so callers
//! don't need to know the new path.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum MessagesCmd {
    /// Daily message rollup. Mirrors the gateway's
    /// `GET /messages/by-day`.
    ByDay {
        /// Lookback window in days (default 30, cap 365).
        #[arg(long, default_value_t = 30)]
        days: u32,
        /// Output as a single JSON object (`{by_day}`).
        #[arg(long)]
        json: bool,
    },
    /// Histogram of messages grouped by role. Mirrors the gateway's
    /// `GET /messages/by-role` (#611).
    ByRole {
        /// Filter to messages within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_role}`).
        #[arg(long)]
        json: bool,
    },
    /// Top-N sessions by message row count. Mirrors the gateway's
    /// `GET /messages/by-session` (#615).
    BySession {
        /// Top-N session limit (default 50, cap 500).
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Filter to messages within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a single JSON object (`{by_session}`).
        #[arg(long)]
        json: bool,
    },
    /// Workspace-wide recent message tail. Mirrors the gateway's
    /// `GET /messages/recent` (#619).
    Recent {
        /// Tail size (default 50, cap 500).
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Filter to one role (user/assistant/tool/system).
        #[arg(long)]
        role: Option<String>,
        /// Filter to messages within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a JSON array.
        #[arg(long)]
        json: bool,
    },
}
