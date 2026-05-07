use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum BackupCmd {
    /// Create a tar.gz backup of the workspace .argo directory.
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
    List {
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Verify a backup tarball's manifest and file hashes.
    Verify {
        path: PathBuf,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
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
