//! `argo budget` subcommands for per-worker spend visibility.
use clap::{Subcommand, ValueEnum};

/// Output encoding for budget read-side commands.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum BudgetOutputFormat {
    /// Human-readable text.
    Human,
    /// JSON output matching the legacy `--json` switch.
    Json,
    /// RFC 4180 CSV with a header row.
    Csv,
}

impl BudgetOutputFormat {
    /// Preserve legacy `--json` behavior while allowing
    /// `--format json|csv|human`.
    pub fn resolve(self, json: bool) -> Self {
        if json {
            Self::Json
        } else {
            self
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum BudgetCmd {
    /// Show daily/monthly rollups, policy headroom, and last
    /// five charges for one worker.
    Show {
        worker_id: String,
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum, default_value_t = BudgetOutputFormat::Human)]
        format: BudgetOutputFormat,
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
        #[arg(long, value_enum, default_value_t = BudgetOutputFormat::Human)]
        format: BudgetOutputFormat,
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
        #[arg(long, value_enum, default_value_t = BudgetOutputFormat::Human)]
        format: BudgetOutputFormat,
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
    /// Watch worker budget spend and fire push alerts on 80% /
    /// 100% daily-cap threshold crossings.
    Watch {
        /// Restrict the watch pass to one worker id.
        #[arg(long)]
        worker: Option<String>,
        /// Run a single pass instead of looping forever.
        #[arg(long)]
        once: bool,
        /// Emit JSON tick summaries.
        #[arg(long)]
        json: bool,
    },
    /// Inspect or test budget alert deliveries.
    Alerts {
        #[command(subcommand)]
        cmd: BudgetAlertsCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum BudgetAlertsCmd {
    /// List recent budget alerts.
    List {
        /// Restrict to one worker id.
        #[arg(long)]
        worker: Option<String>,
        /// Lookback window, e.g. 30m, 1h, 7d, or RFC3339.
        #[arg(long)]
        since: Option<String>,
        /// Cap the row count.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Emit one JSON object per alert.
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum, default_value_t = BudgetOutputFormat::Human)]
        format: BudgetOutputFormat,
    },
    /// Fire a synthetic budget alert immediately, bypassing dedup.
    Test {
        /// Worker id to include in the alert.
        #[arg(long)]
        worker: String,
        /// Threshold to test.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        threshold: u8,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
}
