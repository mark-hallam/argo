use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ActivityCmd {
    /// Follow worker activity as audit rows arrive.
    Tail {
        /// Filter to one tenant id.
        #[arg(long)]
        tenant: Option<String>,
        /// Filter to one worker id.
        #[arg(long)]
        worker: Option<String>,
        /// Backfill rows from a recent duration before tailing
        /// (examples: 30s, 5m, 1h, 1d).
        #[arg(long)]
        since: Option<String>,
        /// Emit newline-delimited JSON activity envelopes.
        #[arg(long)]
        json: bool,
    },
    /// List recent worker activity in a duration window.
    Since {
        /// Lookback duration (examples: 30s, 5m, 1h, 1d).
        duration: String,
        /// Maximum rows to print.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Filter to one tenant id.
        #[arg(long)]
        tenant: Option<String>,
        /// Filter to one worker id.
        #[arg(long)]
        worker: Option<String>,
        /// Emit newline-delimited JSON activity envelopes.
        #[arg(long)]
        json: bool,
    },
}
