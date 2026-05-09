//! `argo worker okr set/show/progress` subcommand schema (Stream
//! `claude-worker-okr-cli`, mission claim 2 worker-goals layer).
//! Lives in a sibling file so `cli_worker_cmds.rs` stays under the
//! 500-LOC per-file ceiling — same pattern used by
//! `cli_worker_kpi_cmds.rs`.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WorkerOkrCmd {
    /// Declare an objective + its weighted key-results for one period.
    Set {
        /// Worker id (UUIDv7) the objective belongs to.
        worker_id: String,
        /// Tenant id whose worker namespace owns the row.
        #[arg(long)]
        tenant: String,
        /// Period token (`YYYYQ[1-4]`, e.g. `2026Q2`).
        #[arg(long, value_parser = parse_period)]
        period: String,
        /// Operator-facing objective title.
        #[arg(long)]
        title: String,
        /// Optional long-form description.
        #[arg(long)]
        description: Option<String>,
        /// Repeatable `--kr kpi:op:target:weight`. `op` is `gte|lte|eq`,
        /// `weight > 0`. At least one is required.
        #[arg(long = "kr", num_args = 1.., required = true)]
        kr: Vec<String>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// List one worker's objectives, optionally filtered by period.
    Show {
        /// Worker id (UUIDv7) to query.
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Filter to one period (`YYYYQ[1-4]`).
        #[arg(long, value_parser = parse_period)]
        period: Option<String>,
        /// Output format: `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Score each objective against the latest `worker_kpi_value` row.
    Progress {
        /// Worker id (UUIDv7) to score.
        worker_id: String,
        /// Tenant id whose worker namespace should be queried.
        #[arg(long)]
        tenant: String,
        /// Filter to one period (`YYYYQ[1-4]`).
        #[arg(long, value_parser = parse_period)]
        period: Option<String>,
        /// Emit JSON instead of a human-readable scorecard.
        #[arg(long)]
        json: bool,
    },
}

/// Validate `YYYYQ[1-4]` exactly. Mirrors the brief's
/// `^(19|20)\d{2}Q[1-4]$` regex without pulling in a regex crate.
pub fn parse_period(raw: &str) -> Result<String, String> {
    let bytes = raw.as_bytes();
    if bytes.len() != 6 {
        return Err(format!(
            "invalid period '{raw}': expected YYYYQ[1-4] (e.g. 2026Q2)"
        ));
    }
    let century_ok = matches!(&raw[..2], "19" | "20");
    let year_digits_ok = bytes[2].is_ascii_digit() && bytes[3].is_ascii_digit();
    let q_ok = bytes[4] == b'Q';
    let quarter_ok = matches!(bytes[5], b'1' | b'2' | b'3' | b'4');
    if century_ok && year_digits_ok && q_ok && quarter_ok {
        Ok(raw.to_string())
    } else {
        Err(format!(
            "invalid period '{raw}': expected YYYYQ[1-4] (e.g. 2026Q2)"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_period_accepts_valid() {
        assert_eq!(parse_period("2026Q2").unwrap(), "2026Q2");
        assert_eq!(parse_period("1999Q4").unwrap(), "1999Q4");
        assert_eq!(parse_period("2099Q1").unwrap(), "2099Q1");
    }

    #[test]
    fn parse_period_rejects_invalid() {
        assert!(parse_period("2026q2").is_err(), "lowercase q");
        assert!(parse_period("2026Q5").is_err(), "quarter out of range");
        assert!(parse_period("1899Q1").is_err(), "century outside 19/20");
        assert!(parse_period("2026").is_err(), "missing Q segment");
        assert!(parse_period("2026Q").is_err(), "missing quarter digit");
        assert!(parse_period("Q12026").is_err(), "garbage layout");
    }
}
