//! `argo gateway …` — gateway control + probes. Lifted out of
//! `cli_subcommands.rs` once that file went over the 500-LOC
//! budget on the new `Probe` variant (#631). Re-exported via
//! `cli.rs`'s wildcard glob so callers don't need to know the
//! new path.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum GatewayCmd {
    /// Bind a TCP listener and serve the JSON HTTP gateway. The
    /// first cut answers `GET /health` + `GET /version`; the
    /// `Adapter` routing surface from `argo-gateway` lands later.
    Start {
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        #[arg(long, default_value_t = 7400)]
        port: u16,
        /// Stop after N requests (mainly for tests; omit to run
        /// until SIGINT / SIGTERM).
        #[arg(long)]
        max_requests: Option<u64>,
        /// Suppress the boot-time route catalog banner. Top of the
        /// precedence chain — overrides `ARGO_BANNER_*` env vars and
        /// any `[startup_banner]` TOML section.
        #[arg(long)]
        quiet: bool,
    },
    Stop,
    /// One-line status: running PID + listening address.
    Status {
        /// Output as a JSON object `{running, pid?, addr?}`.
        #[arg(long)]
        json: bool,
    },
    /// Print the gateway's static route catalog. No daemon
    /// required — reads the in-binary catalog. `--method
    /// GET|POST` filters by verb; `--grep <sub>` filters by
    /// path/description substring (case-insensitive); `--limit
    /// N` truncates to the first N rows; `--json` for the
    /// machine-readable form.
    Routes {
        /// Output as JSON. Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`
        /// (renders the catalog as a markdown table — paste-able
        /// into READMEs). `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long)]
        method: Option<String>,
        /// Case-insensitive substring filter on both path and
        /// description.
        #[arg(long)]
        grep: Option<String>,
        /// Truncate the output to the first N rows after filters.
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Hit one of the K8s-style gateway probe endpoints (#630)
    /// and return exit 0 on success, 1 on failure. Useful for
    /// shell scripts / CI gates / boot-up health checks that
    /// want a binary signal without parsing `argo doctor`.
    /// Default kind is `ready` (DB-aware); pass `live` for the
    /// minimal liveness probe.
    Probe {
        /// Probe kind: `live` (cheap liveness) or `ready`
        /// (verifies the SQLite store responds).
        #[arg(default_value = "ready")]
        kind: String,
        /// Bind host (default 127.0.0.1).
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Bind port (default 7400).
        #[arg(long, default_value_t = 7400)]
        port: u16,
        /// Output as a JSON object instead of human text.
        #[arg(long)]
        json: bool,
    },
}
