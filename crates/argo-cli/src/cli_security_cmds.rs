//! `argo security ...` subcommand surface.
//!
//! Two coordinated security families share this enum:
//!
//! - `argo security audit` — vuln-scan wrapper around
//!   `cargo audit` + `cargo deny check advisories` (Mission
//!   `mission-vuln-management-workflow`).
//! - `argo security scan-secrets` — CI parity for the pre-commit
//!   secret-scan hook (Mission
//!   `mission-secret-scanning-pre-commit-hook`).
//! - `argo security rotate-keys` — key-rotation coordinator
//!   covering DB encryption, credentials master, gateway bearer,
//!   channel HMAC, and per-connector OAuth refresh tokens (Mission
//!   `mission-automated-key-rotation`).
//!
//! See `docs/skycot/security/vuln-management.md` and the "Key
//! rotation" section of `docs/skycot/RUNBOOK.md` for the workflows
//! these commands feed.
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum SecurityCmd {
    /// Run `cargo audit` + `cargo deny check advisories`, merge
    /// their results, and print a single JSON envelope of
    /// `{vulnerabilities[], total_count_by_severity}`. Exit codes:
    /// `0` = no unmitigated rows above floor, `1` = at least one
    /// unmitigated row above floor, `2` = tooling error (missing
    /// `cargo-audit`/`cargo-deny`, network failure pulling the
    /// RustSec feed, malformed JSON output).
    Audit {
        /// Drop rows below this severity from `vulnerabilities[]`.
        /// One of `low` (default), `medium`, `high`, `critical`.
        /// Below-floor rows still count toward
        /// `total_count_by_severity`.
        #[arg(long, default_value = "low")]
        severity_floor: String,
        /// Emit single-line compact JSON instead of pretty-printed.
        /// Useful for piping into `jq` / ops dashboards.
        #[arg(long)]
        compact: bool,
    },
    /// Scan one or more paths for likely credentials (OpenAI /
    /// Anthropic / AWS / Stripe / Slack / GitHub / SSH private
    /// keys / JWTs / Argo bearer tokens). CI parity for the
    /// pre-commit hook installed by `argo doctor install-hooks`.
    /// Exits non-zero when any unsuppressed match is found.
    ScanSecrets(SecretScanArgs),
    /// Coordinated key-rotation operations across the workspace
    /// (Mission `mission-automated-key-rotation`).
    RotateKeys {
        #[command(subcommand)]
        cmd: RotateKeysCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum RotateKeysCmd {
    /// Enumerate every named key with its last-rotated date and age
    /// in days. Default output is a Markdown table; `--json` emits
    /// a JSON array suitable for scripting.
    List(RotateKeysListArgs),
    /// Show the rotation plan for `<key-name>`: which subsystem owns
    /// it, what re-encryption / re-keying steps run, and any
    /// operator follow-ups (config edits, restarts). Read-only.
    Plan(RotateKeysPlanArgs),
    /// Execute the rotation plan for `<key-name>`. Idempotent —
    /// re-running with the same `--new-key-source` after a
    /// successful rotation is a no-op. Always writes a
    /// `security.key_rotated` audit row.
    Commit(RotateKeysCommitArgs),
    /// Exit non-zero when any registered key is older than
    /// `--threshold` days (default 90). Designed for cron-driven
    /// monitoring. Exit codes: 0 = all fresh, 1 = ≥1 stale,
    /// 2 = config error.
    AuditKeyAge(RotateKeysAuditAgeArgs),
}

#[derive(Args, Debug)]
pub struct RotateKeysListArgs {
    /// Equivalent to `--format json`.
    #[arg(long)]
    pub json: bool,
    /// Output format: `markdown` (default) or `json`.
    #[arg(long, default_value = "markdown")]
    pub format: String,
}

#[derive(Args, Debug)]
pub struct RotateKeysPlanArgs {
    /// Registered key name. Use `list` to enumerate.
    pub key_name: String,
    /// Equivalent to `--format json`.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct RotateKeysCommitArgs {
    /// Registered key name. Use `list` to enumerate.
    pub key_name: String,
    /// Optional source for the new key material. Accepted values:
    /// `random` (default — OS RNG), `prompt` (read from stdin
    /// without echo), or `env:<VAR>` (read from an env var).
    /// Never logged.
    #[arg(long, default_value = "random")]
    pub new_key_source: String,
    /// Skip the audit row + last-rotated bump; print the plan only.
    /// Equivalent to `argo security rotate-keys plan <name>`.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit a single JSON envelope instead of human-readable status.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct RotateKeysAuditAgeArgs {
    /// Age threshold in days. Keys older than this exit non-zero.
    #[arg(long, default_value_t = 90)]
    pub threshold: u32,
    /// Emit a single JSON envelope (`{stale, fresh, threshold_days}`)
    /// instead of human-readable status.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SecretScanArgs {
    /// One or more file or directory paths to scan. Directories
    /// are walked recursively, skipping `.git/` and files whose
    /// first 1 KiB contains a NUL byte (binary heuristic).
    pub paths: Vec<PathBuf>,
    /// Path to the operator allowlist. Defaults to
    /// `.skycot/secret-scan-allowlist.toml`. Missing file is OK
    /// (treated as empty allowlist); malformed TOML is fatal.
    #[arg(long)]
    pub allowlist: Option<PathBuf>,
    /// Emit findings as one JSON array on stdout. Default is a
    /// human-readable `file:line: [id] human -> redacted` per match.
    #[arg(long)]
    pub json: bool,
}
