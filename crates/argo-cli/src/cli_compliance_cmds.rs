//! `argo compliance` subcommand surface.
//!
//! GDPR Article 15 (right-to-access) data export:
//!
//! ```text
//! argo compliance export --tenant <id> --format gdpr [--out <path>]
//! ```
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ComplianceCmd {
    /// SOC2 Common Criteria 6.x audit-trail report. Groups events
    /// in `[--from, --to)` by control category (Access, ChangeMgmt,
    /// DataMgmt, Security, Other).
    Soc2(crate::cli::ComplianceSoc2Args),
    /// Emit every tenant-scoped row across the database for GDPR
    /// right-to-access. Output is a single JSON envelope.
    Export(ComplianceExportArgs),
}

#[derive(Args, Debug)]
pub struct ComplianceExportArgs {
    /// Tenant id (UUID) whose data should be exported.
    #[arg(long)]
    pub tenant: String,
    /// Output shape. Currently the only accepted value is `gdpr`.
    #[arg(long, default_value = "gdpr")]
    pub format: String,
    /// Write the JSON envelope to this path. Defaults to stdout.
    #[arg(long)]
    pub out: Option<String>,
}
