//! Subcommands for `argo recover-cache list/show/clear` — the
//! operator-facing surface over the `recovery_cache` table populated
//! by `argo recover` (#901).

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum RecoverCacheCmd {
    /// List recent recovery cache entries newest-first.
    List(RecoverCacheListCmd),
    /// Print cached suggestions for a session without re-calling the
    /// LLM. Operates fully offline.
    Show(RecoverCacheShowCmd),
    /// Invalidate cache entries — by session, by age, or wipe all
    /// (with typed confirmation).
    Clear(RecoverCacheClearCmd),
}

/// `argo recover-cache list [--since <duration>] [--limit N]`
#[derive(Args, Debug)]
pub struct RecoverCacheListCmd {
    /// Filter to entries cached within the past `<duration>` (e.g.
    /// `7d`, `12h`, `30m`). Default: no filter.
    #[arg(long)]
    pub since: Option<String>,
    /// Maximum number of rows to return (default: 25, max: 1000).
    #[arg(long)]
    pub limit: Option<u32>,
    /// Emit JSON envelope.
    #[arg(long)]
    pub json: bool,
}

/// `argo recover-cache show <session-id>`
#[derive(Args, Debug)]
pub struct RecoverCacheShowCmd {
    /// Session id (UUID) whose cached suggestions to print.
    pub session_id: String,
    /// Emit JSON envelope.
    #[arg(long)]
    pub json: bool,
}

/// `argo recover-cache clear [<session-id>] [--all] [--older-than
/// <duration>] [--confirm <token>]`
#[derive(Args, Debug)]
pub struct RecoverCacheClearCmd {
    /// Session id whose cache entries to remove. Mutually exclusive
    /// with `--all` / `--older-than`.
    pub session_id: Option<String>,
    /// Wipe every entry. Requires `--confirm yes-i-mean-it`.
    #[arg(long, conflicts_with_all = ["session_id", "older_than"])]
    pub all: bool,
    /// Evict entries older than this duration (e.g. `7d`, `30d`,
    /// `12h`).
    #[arg(long, conflicts_with_all = ["session_id", "all"])]
    pub older_than: Option<String>,
    /// Typed confirmation token. Required for `--all`.
    #[arg(long)]
    pub confirm: Option<String>,
    /// Emit JSON envelope.
    #[arg(long)]
    pub json: bool,
}
