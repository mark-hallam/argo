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
    /// Browser-facing surfaces mounted ahead of the bearer gate:
    /// `/dashboard/*` (operator dashboard) and `/marketplace/*`
    /// (skill marketplace MVP — browse + install cross-tenant
    /// skills).
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
        /// Hidden maintainer flag — emit per-init-phase wall-clock
        /// timings to stderr and exit before listening. Used by
        /// `scripts/profile-startup.sh` to drive cold/warm startup
        /// regression bench on M-series hardware (see brief
        /// claude-perf-startup-time).
        #[arg(long, hide = true)]
        profile_startup: bool,
        /// Hidden escape hatch — bypass the four critical startup
        /// health checks (store reachable, schema version match,
        /// provider key present, channels reachable). Use only for
        /// dev / recovery scenarios where the gateway must boot
        /// against a degraded environment. Production should leave
        /// the checks enabled and let exit code 78 trigger a
        /// CrashLoopBackoff.
        #[arg(long, hide = true)]
        skip_startup_check: bool,
        /// Opt in to binding a non-loopback address for dev/recovery.
        #[arg(long = "bind-non-loopback")]
        bind_non_loopback: bool,
        /// Required with `--bind-non-loopback`; records operator
        /// acceptance of the plaintext bind warning.
        #[arg(long = "accept-insecure-bind-warning")]
        accept_insecure_bind_warning: bool,
        /// Deprecated old escape hatch. Use `--bind-non-loopback`
        /// plus `--accept-insecure-bind-warning` instead.
        #[arg(long = "insecure-bind")]
        insecure_bind: bool,
    },
    Stop,
    /// Manage the first-run setup wizard bootstrap token.
    Bootstrap {
        #[command(subcommand)]
        cmd: GatewayBootstrapCmd,
    },
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
    /// Inspect per-HTTP-request gateway audit rows.
    Audit {
        #[command(subcommand)]
        cmd: GatewayAuditCmd,
    },
    /// Manage `service_token_binding` rows — anchor bearer tokens to
    /// a `(principal, tenant)` pair so the gateway can derive the
    /// authenticated principal from the credential rather than from
    /// caller-supplied headers
    /// (`mission-audit-principal-tenant-from-token`).
    Token {
        #[command(subcommand)]
        cmd: GatewayTokenCmd,
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

/// `argo gateway token ...` subcommands
/// (`mission-audit-principal-tenant-from-token`).
#[derive(Subcommand, Debug)]
pub enum GatewayTokenCmd {
    /// Bind a bearer token to a `(principal, tenant)` pair. The
    /// token is hashed with SHA-256 before persistence; the raw
    /// value is never written to the DB. The token MUST be passed
    /// via `--token-file <path>` or the `ARGO_GATEWAY_BIND_TOKEN`
    /// env var — never on argv directly.
    Bind {
        /// Path to a file whose first line is the raw bearer token.
        /// Mutually exclusive with `ARGO_GATEWAY_BIND_TOKEN`.
        #[arg(long = "token-file")]
        token_file: Option<std::path::PathBuf>,
        /// Principal identity to stamp on requests using this
        /// token (e.g. `worker:<uuid>`, `human:<email>`,
        /// `system:<name>`).
        #[arg(long)]
        principal: String,
        /// Tenant id (UUID) the token operates under.
        #[arg(long)]
        tenant: String,
        /// Optional operator-facing label (e.g. `"ci-runner"`).
        #[arg(long)]
        label: Option<String>,
    },
    /// Remove a binding by its raw token (hashed for lookup).
    Unbind {
        #[arg(long = "token-file")]
        token_file: Option<std::path::PathBuf>,
    },
    /// List every bound token (fingerprint + principal + tenant +
    /// label + created-at). The raw token / full hash are never
    /// printed.
    List {
        #[arg(long)]
        json: bool,
    },
}

/// `argo gateway audit ...` subcommands.
#[derive(Subcommand, Debug)]
pub enum GatewayAuditCmd {
    /// List request audit rows newest-first.
    List {
        /// Include rows at or after this time (`1h ago`, `30m`, or RFC3339).
        #[arg(long)]
        since: Option<String>,
        /// Filter by exact principal id.
        #[arg(long)]
        principal: Option<String>,
        /// Filter by exact HTTP status code.
        #[arg(long)]
        status: Option<u16>,
        /// Output a JSON envelope.
        #[arg(long)]
        json: bool,
        /// Maximum rows to return (1..1000).
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Show one request audit row by request id.
    Show {
        /// UUID request id.
        request_id: String,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// `argo gateway bootstrap ...` subcommands.
#[derive(Subcommand, Debug)]
pub enum GatewayBootstrapCmd {
    /// Regenerate `$ARGO_HOME/.bootstrap-token` and audit the rotation.
    Rotate,
}
