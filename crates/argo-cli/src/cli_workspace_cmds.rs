//! Workspace / DB / Costs command enums. Hoisted out of
//! `cli_subcommands.rs` to keep that file under the 500-LOC
//! ceiling. Re-exported by `cli_subcommands.rs` so existing
//! `use crate::cli::FooCmd;` callers stay unchanged.
use clap::{Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum WorkspaceCmd {
    /// Initialize a fresh workspace at `.` (writes argo.toml + the
    /// canonical bundle directories).
    Init,
    /// One-line status summary (workspace + db + costs).
    Status {
        /// Output as a single JSON object for scripting.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Verify workspace integrity: argo.toml parses, memory files +
    ///   bundle dirs exist, agent persona files exist, manifests
    ///   referenced from `[provider.profiles.*]` resolve.
    Verify,
    /// Pull a workspace bundle from an OCI ref and extract it
    /// into `--into <dir>` (defaults to a sibling directory named
    /// after the repo). The destination directory must not exist.
    Fork {
        /// `<host>/<repo>:<tag>` reference, e.g.
        /// `ghcr.io/argo-agents/example:v1`.
        source: String,
        /// Destination directory. Defaults to
        /// `./fork-<repo>-<tag>` next to the cwd.
        #[arg(long)]
        into: Option<String>,
    },
    Push {
        oci_ref: String,
    },
    Pull {
        oci_ref: String,
    },
    /// List recorded marketplace pushes (newest first). Each row
    /// shows the OCI reference, bundle digest, pushed-at
    /// timestamp, and `pushed_by` attribution.
    Artifacts {
        #[arg(long, default_value_t = 25)]
        limit: u32,
        /// Output as JSON (one object per row) for scripting.
        #[arg(long)]
        json: bool,
    },
    Diff {
        other: String,
    },
    Publish,
}

#[derive(Subcommand, Debug)]
pub enum DbCmd {
    /// Migrate up to latest.
    MigrateUp,
    /// Migrate down to a specific version.
    MigrateDown {
        #[arg(long)]
        to: u32,
    },
    /// Show current schema version + per-table row counts.
    Status {
        /// Output as a single JSON object.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Drop sessions whose `started_at` is older than `--older-than-days`
    /// days. Pass `--force` to actually delete; otherwise dry-runs.
    Prune {
        #[arg(long, default_value_t = 90)]
        older_than_days: i64,
        #[arg(long)]
        force: bool,
    },
    /// Run SQLite `VACUUM` to reclaim file-space after a prune.
    Vacuum,
    /// Print the current SQLite schema migration version. Same
    /// number `db status` reports under "schema_version", but
    /// scripts often just want the bare integer.
    Version {
        /// Output as a single JSON object `{version}`.
        #[arg(long)]
        json: bool,
    },
    /// Run SQLite `PRAGMA integrity_check` against the workspace
    /// store. Prints "ok" when healthy, otherwise the diagnostic
    /// rows. Mirrors the gateway's `GET /db/integrity`.
    Integrity {
        /// Output as `{ok, check}` JSON.
        #[arg(long)]
        json: bool,
    },
    /// Rotate the at-rest encryption key in place via `PRAGMA rekey`
    /// (Stream `mission-encryption-at-rest-sqlcipher`). The old key
    /// is loaded from `[security.encryption] key_source` in
    /// `argo.toml`; the new key is supplied via `--new-key-source`.
    /// Stock builds (no SQLCipher-linked driver) succeed but do not
    /// actually encrypt — see `docs/skycot/RUNBOOK.md`.
    Rekey {
        /// `env:VAR` or `file:/path` for the *new* key. `none` is
        /// rejected — disabling encryption is a separate workflow.
        #[arg(long)]
        new_key_source: String,
        /// JSON output for scripting.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum CostsCmd {
    /// Set a hard cap (`--session`, `--day`, or `--month`) in USD.
    /// Pass `--reset` instead of a `<usd>` value to remove the
    /// cap (the field falls back to the workspace default).
    Cap {
        #[arg(long, conflicts_with_all = ["day", "month"])]
        session: bool,
        #[arg(long, conflicts_with_all = ["session", "month"])]
        day: bool,
        #[arg(long, conflicts_with_all = ["session", "day"])]
        month: bool,
        /// USD cap value. Either this OR `--reset` must be set.
        #[arg(conflicts_with = "reset")]
        usd: Option<f64>,
        /// Remove the cap field instead of setting it.
        #[arg(long)]
        reset: bool,
    },
    /// Show the configured caps + actual-spend roll-up.
    Show {
        /// Output as a single JSON object.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Group spend by `(provider, model)` and print the
        /// top spenders instead of the default ceilings view.
        #[arg(long, conflicts_with = "by_session")]
        by_model: bool,
        /// Group spend by `session_id` and print the top
        /// spenders instead of the default ceilings view.
        #[arg(long)]
        by_session: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Dump every row in `spend_ledger` as NDJSON (one JSON object
    /// per line). Pass `--out <path>` to write to a file; otherwise
    /// streams to stdout. `--since-minutes` scopes to recent rows.
    Export {
        #[arg(long)]
        out: Option<String>,
        /// Filter to rows charged within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
    },
    /// List sessions whose total spend exceeds `--threshold-usd`.
    /// Useful for spotting runaway sessions / abuse / model
    /// misconfiguration without parsing the ledger by hand.
    Alert {
        /// USD threshold; sessions with `SUM(amount_usd_micros)` >
        /// `threshold * 1e6` are reported.
        #[arg(long, default_value_t = 1.0)]
        threshold_usd: f64,
        /// Output as NDJSON (one row per offending session).
        #[arg(long)]
        json: bool,
    },
    /// Print just the today / month-to-date / all-time spend
    /// roll-up, nothing else. Mirrors the gateway's
    /// `GET /costs/total` endpoint.
    Total {
        /// Output as a single JSON object (`{today, this_month,
        /// all_time}` in usd_micros) for scripting.
        #[arg(long)]
        json: bool,
    },
    /// Daily spend rollup — chronological list of
    /// `{day, spent_usd_micros}` for the last `--days` days.
    /// Mirrors the gateway's `GET /costs/by-day` endpoint.
    ByDay {
        /// Lookback window in days (default 30, cap 365).
        #[arg(long, default_value_t = 30)]
        days: u32,
        /// Output as a single JSON object (`{by_day}`).
        #[arg(long)]
        json: bool,
    },
    /// Top-50 spend buckets grouped by `(provider, model)`.
    /// Mirrors the gateway's `GET /costs/by-model`.
    ByModel {
        /// Output as a single JSON object (`{by_model}`).
        #[arg(long)]
        json: bool,
    },
    /// Top-50 spend buckets grouped by provider only (no model
    /// split). Mirrors the gateway's `GET /costs/by-provider` (#632).
    ByProvider {
        /// Output as a single JSON object (`{by_provider}`).
        #[arg(long)]
        json: bool,
    },
    /// Top-50 spend buckets grouped by `session_id`.
    /// Mirrors the gateway's `GET /costs/by-session`.
    BySession {
        /// Output as a single JSON object (`{by_session}`).
        #[arg(long)]
        json: bool,
    },
    /// Workspace-wide recent spend ledger entries. Mirrors the
    /// gateway's `GET /costs/recent` (#625).
    Recent {
        /// Tail size (default 50, cap 500).
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Filter to charges within this many recent minutes.
        #[arg(long)]
        since_minutes: Option<i64>,
        /// Output as a JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Linear extrapolation of end-of-period spend from current
    /// trajectory: `rate = sum(spend in lookback) / lookback_seconds`,
    /// `projected_additional = rate × time-remaining`. The ±20% band
    /// is a flat v1 heuristic.
    Forecast {
        /// Horizon: end-of-day, end-of-month, or end-of-quarter.
        #[arg(long, value_enum, default_value_t = crate::handlers::costs_forecast::Horizon::Eom)]
        horizon: crate::handlers::costs_forecast::Horizon,
        /// Lookback window. Suffixes: `s|m|h|d` (e.g. `24h`, `7d`, `30d`).
        #[arg(long, default_value = "24h")]
        lookback: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = crate::handlers::costs_forecast::ForecastFormat::Markdown)]
        format: crate::handlers::costs_forecast::ForecastFormat,
    },
    /// Read-only spend anomaly table view.
    Anomalies {
        /// Filter to anomalies observed after RFC3339 or YYYY-MM-DD.
        #[arg(long)]
        since: Option<String>,
        /// Cap row count (default 50, hard max 1000).
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(1..=1000))]
        limit: u32,
        /// Output format.
        #[arg(long, value_enum, default_value_t = CostsAnomaliesFormat::Markdown)]
        format: CostsAnomaliesFormat,
        /// Filter to one tenant id via the worker table.
        #[arg(long)]
        tenant: Option<String>,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CostsAnomaliesFormat {
    Markdown,
    Json,
}
