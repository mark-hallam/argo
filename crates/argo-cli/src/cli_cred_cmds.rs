//! `argo cred` subcommands for workspace credential storage.

use clap::Subcommand;

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
}
