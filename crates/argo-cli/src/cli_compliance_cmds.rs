//! `argo compliance` subcommand surface.
//!
//! GDPR Article 15 (right-to-access) data export:
//!
//! ```text
//! argo compliance export --tenant-id <id> --format json [--out <path>]
//! ```
//!
//! GDPR Article 17 (right-to-erasure) tenant-cascade delete:
//!
//! ```text
//! argo compliance delete --tenant <id> --confirm <id> [--dry-run]
//! ```
//!
//! GDPR Article 30 records of processing activities:
//!
//! ```text
//! argo compliance ropa --tenant <id> --output <path>
//! ```
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ComplianceCmd {
    /// SOC2 Common Criteria 6.x audit-trail report. Groups events
    /// in `[--from, --to)` by control category (Access, ChangeMgmt,
    /// DataMgmt, Security, Other).
    Soc2(crate::cli::CliComplianceSoc2Args),
    /// Emit tenant-scoped compliance data with filters and an
    /// integrity stamp.
    Export(CliComplianceExportArgs),
    /// Write a tenant-scoped GDPR Article 30 ROPA JSON document.
    Ropa(CliComplianceRopaArgs),
    /// Cascade-delete every tenant-scoped row for GDPR right-to-
    /// erasure. Audit rows are tombstoned in place (PII fields
    /// replaced with `<erased>`, hash chain re-stitched). The whole
    /// operation is a single transaction; `--dry-run` rolls it back.
    Delete(CliComplianceDeleteArgs),
}

#[derive(Args, Debug)]
pub struct CliComplianceExportArgs {
    /// Tenant id (UUID) whose data should be exported.
    #[arg(long, alias = "tenant-id")]
    pub tenant: String,
    /// Output shape: `json`, `csv`, or `pdf`. `gdpr` remains a
    /// backwards-compatible alias for `json`.
    #[arg(long, default_value = "json")]
    pub format: String,
    /// Include rows created on or after this RFC3339 date/time.
    #[arg(long = "start-date")]
    pub start_date: Option<String>,
    /// Include rows created on or before this RFC3339 date/time.
    #[arg(long = "end-date")]
    pub end_date: Option<String>,
    /// Restrict rows to this session id.
    #[arg(long = "session-id")]
    pub session_id: Option<String>,
    /// Write the JSON envelope to this path. Defaults to stdout.
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Args, Debug)]
pub struct CliComplianceRopaArgs {
    /// Tenant id (UUID) whose Article 30 record should be exported.
    #[arg(long, alias = "tenant-id")]
    pub tenant: String,
    /// Write the Article 30 JSON document to this path.
    #[arg(long)]
    pub output: String,
}

#[derive(Args, Debug)]
pub struct CliComplianceDeleteArgs {
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
