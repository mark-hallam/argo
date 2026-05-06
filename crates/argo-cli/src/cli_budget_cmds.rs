//! `argo budget` subcommands for per-worker spend visibility.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum BudgetCmd {
    /// Show daily/monthly rollups, policy headroom, and last
    /// five charges for one worker.
    Show {
        worker_id: String,
        #[arg(long)]
        json: bool,
    },
    /// List recent worker-budget charges.
    Recent {
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        worker: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List spend-anomaly events (Welford z-score on per-worker
    /// USD-per-hour history; threshold env-overridable via
    /// `ARGO_SPEND_ANOMALY_Z`, default 4.0). Stream
    /// `claude-spend-anomaly-alerts`.
    Anomalies {
        /// Restrict to one worker.
        #[arg(long)]
        worker: Option<String>,
        /// Lookback window — accepts e.g. `60s`, `30m`, `24h`, `7d`.
        #[arg(long)]
        since: Option<String>,
        /// Cap the row count.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Emit a JSON envelope instead of the human-readable
        /// table.
        #[arg(long)]
        json: bool,
    },
    /// Run one anomaly-detection tick: walk every active worker,
    /// fold the previous-hour USD-per-hour into the
    /// (worker, hour-of-day) baseline, and write a
    /// `worker_spend_anomaly` row when |z| exceeds the configured
    /// threshold. Designed for cron / scheduler invocation.
    AnomalyTick {
        /// Emit the per-worker decisions as JSON instead of the
        /// human summary.
        #[arg(long)]
        json: bool,
    },
}
