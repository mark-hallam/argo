//! `argo cred` subcommands for workspace credential storage.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum CredCmd {
    /// Store one credential. If value is omitted, prompt without echo.
    Set {
        key: String,
        value: Option<String>,
    },
    /// List stored credential keys; values are masked by default.
    List {
        #[arg(long)]
        reveal: bool,
    },
    /// Remove one stored credential.
    Clear {
        key: String,
    },
    /// Remove every stored credential.
    ClearAll,
    /// Emit shell export statements for stored credentials.
    Export {
        #[arg(long)]
        shell: String,
    },
    /// Import credential-looking values from the current environment.
    Import {
        #[arg(long)]
        shell: String,
    },
    /// Set the credential storage backend in `argo.toml`
    /// (`[credentials] backend = "file" | "keyring" | "auto"`).
    Backend {
        /// One of: `file`, `keyring`, `auto`.
        backend: String,
    },
    /// Migrate workspace credentials from the file store into the OS
    /// keyring. Atomic: either every key migrates or none — keyring
    /// writes are rolled back on partial failure and the file is left
    /// intact.
    MigrateToKeyring {
        /// List entries that would be migrated without modifying any
        /// state.
        #[arg(long)]
        dry_run: bool,
        /// Override the source credentials file (defaults to
        /// `<workspace>/.argo/credentials.toml`).
        #[arg(long)]
        from_file: Option<PathBuf>,
    },
}
