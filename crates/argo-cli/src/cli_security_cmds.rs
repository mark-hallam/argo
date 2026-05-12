//! `argo security ...` subcommand surface (Mission
//! `mission-vuln-management-workflow`). v1 ships a single thin
//! wrapper — `argo security audit` — over `cargo audit` and
//! `cargo deny check advisories`, emitting a merged JSON envelope.
//!
//! See `docs/skycot/security/vuln-management.md` for the workflow
//! these commands are designed to feed.

use clap::Subcommand;

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
}
