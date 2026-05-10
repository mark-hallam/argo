//! `argo compliance` subcommand surface.
//!
//! GDPR Article 15 (right-to-access) data export:
//!
//! ```text
//! argo compliance export --tenant <id> --format gdpr [--out <path>]
//! ```
//!
//! GDPR Article 17 (right-to-erasure) tenant-cascade delete:
//!
//! ```text
//! argo compliance delete --tenant <id> --confirm <id> [--dry-run]
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
    /// Cascade-delete every tenant-scoped row for GDPR right-to-
    /// erasure. Audit rows are tombstoned in place (PII fields
    /// replaced with `<erased>`, hash chain re-stitched). The whole
    /// operation is a single transaction; `--dry-run` rolls it back.
    Delete(ComplianceDeleteArgs),
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

#[derive(Args, Debug)]
pub struct ComplianceDeleteArgs {
    /// Tenant id (UUID) whose data should be deleted.
    #[arg(long)]
    pub tenant: String,
    /// Confirmation id — must exactly match `--tenant`. Mismatch
    /// exits 2 without touching any row.
    #[arg(long)]
    pub confirm: String,
    /// Report per-table row counts that *would* be deleted, then
    /// rollback. No data is modified.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Emit the summary as a single JSON object instead of text.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}
