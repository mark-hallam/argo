use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum CronCmd {
    /// List scheduled jobs in the workspace's `cron/jobs/` directory.
    List {
        /// Output as JSON (one object per job) for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Create a new job (`<name> <schedule> <prompt> <platform> <chat>`).
    Create {
        name: String,
        schedule: String,
        prompt: String,
        platform: String,
        chat: String,
    },
    Edit {
        name: String,
    },
    /// Delete a job by name.
    Delete {
        name: String,
    },
    /// Trigger a job immediately (mainly for tests / manual reruns).
    Trigger {
        name: String,
    },
    /// Run one scheduler tick at the current time. Reports which
    /// jobs would fire; pass `--dry-run` to skip writing the
    /// last-fired cursor.
    Tick {
        #[arg(long)]
        dry_run: bool,
    },
    /// Run the cron tick loop in the foreground at a fixed
    /// interval, holding `cron/.tick.lock` so multiple daemons
    /// don't double-fire. Exits with a non-zero status if the
    /// lock is already held.
    Daemon {
        /// Interval between ticks (seconds). Default 60s.
        #[arg(long, default_value_t = 60)]
        interval_secs: u64,
        /// Stop after N ticks (mainly for tests / manual checks).
        /// Omit to run until SIGINT.
        #[arg(long)]
        max_ticks: Option<u64>,
    },
    /// Install the four canonical system maintenance jobs (Forge
    /// optimize / quality recompute / Dreams curation / A/B
    /// promotion) into `cron/jobs/` by copying the templates from
    /// `cron/system/`. Existing files are preserved unless
    /// `--force` is passed.
    InitSystem {
        /// Preview what would be written without touching disk.
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing job files in `cron/jobs/`.
        #[arg(long)]
        force: bool,
    },
}
