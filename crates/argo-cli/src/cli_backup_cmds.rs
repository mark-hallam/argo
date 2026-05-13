use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum BackupCmd {
    /// Create a tar.gz backup of the workspace .argo directory.
    #[command(after_help = "Examples:\n  \
        argo backup create\n  \
        argo backup create --output /backups/$(date +%F).tar.gz --json\n  \
        argo backup create --include-credentials")]
    Create {
        /// Write the backup to this path instead of .argo/backups.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Include .argo/credentials.toml in the archive.
        #[arg(long)]
        include_credentials: bool,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// List backups under .argo/backups.
    #[command(after_help = "Examples:\n  \
        argo backup list\n  \
        argo backup list --json")]
    List {
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Verify a backup tarball's manifest and file hashes.
    #[command(after_help = "Examples:\n  \
        argo backup verify .argo/backups/2026-05-13.tar.gz\n  \
        argo backup verify ./snap.tar.gz --json")]
    Verify {
        path: PathBuf,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:\n  \
    argo restore .argo/backups/2026-05-13.tar.gz\n  \
    argo restore ./snap.tar.gz --dry-run --json")]
pub struct RestoreCmd {
    pub path: PathBuf,
    /// Restore into this workspace directory.
    #[arg(long)]
    pub into: Option<PathBuf>,
    /// List files without extracting them.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit a JSON envelope.
    #[arg(long)]
    pub json: bool,
}
