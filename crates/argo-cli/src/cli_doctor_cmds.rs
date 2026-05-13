//! `argo doctor` subcommands.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DoctorCmd {
    /// List approval requests expired by the timeout sweeper.
    Approvals {
        /// Look-back window. Supports bare seconds plus `s`, `m`, and `h` suffixes.
        #[arg(long, default_value = "24h")]
        since: String,
        /// Emit a JSON array instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Apply safe doctor remediations, or one named fix with --issue.
    Fix {
        /// Allow confirmation-level fixes such as migrations.
        #[arg(long)]
        auto: bool,
        /// Run only one fix id.
        #[arg(long)]
        issue: Option<String>,
        /// Emit a JSON envelope with fix results.
        #[arg(long)]
        json: bool,
    },
    /// Continuously emit the doctor JSON envelope until Ctrl-C.
    Watch {
        /// Poll interval. Supports bare seconds plus `ms`, `s`, and `m` suffixes.
        #[arg(long, default_value = "30s")]
        interval: String,
        /// Clear the terminal before each cycle for a live dashboard redraw.
        #[arg(long)]
        clear: bool,
    },
    /// Inspect SQLite WAL state — checkpoints the WAL (PASSIVE) and
    /// reports pragma readouts plus `*.db-wal` file size. Operator
    /// inspection for the `mission-sqlite-wal-tuning` profiles.
    WalStats {
        /// Emit a single JSON object instead of the text report.
        #[arg(long)]
        json: bool,
    },
    /// List the in-process per-action-type throttle buckets
    /// (`mission-per-action-type-throttling`). Reports the
    /// configured policies and any live `(tenant_id, action_type)`
    /// bucket state held by the dispatch gate. Output is empty
    /// when `[runtime.action_throttle] enabled = false`.
    Throttles {
        /// Emit a single JSON object instead of the text report.
        #[arg(long)]
        json: bool,
    },
}
