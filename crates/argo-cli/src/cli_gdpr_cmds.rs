use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum GdprCmd {
    /// Delete/redact all data for one subject email.
    Delete(GdprDeleteCmd),
}

#[derive(Args, Debug)]
pub struct GdprDeleteCmd {
    #[arg(long)]
    pub subject: String,
    /// Dry-run only. This is the default unless --confirm is supplied.
    #[arg(long, conflicts_with = "confirm")]
    pub dry_run: bool,
    /// Perform deletion after exact-match email retype.
    #[arg(long)]
    pub confirm: bool,
    /// Tarball path for pre-deletion records. Requires --confirm.
    #[arg(long)]
    pub bundle_out: Option<String>,
    /// Emit a single JSON envelope.
    #[arg(long)]
    pub json: bool,
}
