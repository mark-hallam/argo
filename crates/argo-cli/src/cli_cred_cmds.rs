//! `argo cred` subcommands for workspace credential storage.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum CredCmd {
    /// Store one credential. If value is omitted, prompt without echo.
    #[command(after_help = "Examples:\n  \
        argo cred set ANTHROPIC_API_KEY\n  \
        argo cred set OPENAI_API_KEY sk-...   # value inline (history risk)")]
    Set { key: String, value: Option<String> },
    /// List stored credential keys; values are masked by default.
    #[command(after_help = "Examples:\n  \
        argo cred list\n  \
        argo cred list --reveal\n  \
        argo cred list --json")]
    List {
        #[arg(long)]
        reveal: bool,
        /// Emit `{ok,count,credentials}` as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove one stored credential.
    #[command(after_help = "Examples:\n  \
        argo cred clear OPENAI_API_KEY")]
    Clear { key: String },
    /// Remove every stored credential.
    #[command(after_help = "Examples:\n  \
        argo cred clear-all")]
    ClearAll,
    /// Emit shell export statements for stored credentials.
    #[command(after_help = "Examples:\n  \
        eval \"$(argo cred export --shell bash)\"\n  \
        argo cred export --shell zsh")]
    Export {
        #[arg(long)]
        shell: String,
    },
    /// Import credential-looking values from the current environment.
    #[command(after_help = "Examples:\n  \
        argo cred import --shell bash")]
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
    /// Rewrite the workspace credentials file with every plaintext
    /// entry wrapped in an age envelope under the workspace master
    /// key (`ARGO_CREDENTIALS_MASTER_KEY` env or
    /// `~/.skycot/.master_key` file). Already-encrypted entries pass
    /// through unchanged.
    EncryptAll {
        /// Override the credentials file path (defaults to
        /// `<workspace>/.argo/credentials.toml`).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Emit a JSON report instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::Cli;

    fn parsed_debug(args: &[&str]) -> String {
        let mut argv = vec!["argo"];
        argv.extend_from_slice(args);
        format!("{:?}", Cli::try_parse_from(argv).expect("parse CLI args"))
    }

    #[test]
    fn former_list_show_status_gaps_accept_json_flag() {
        let commands: &[&[&str]] = &[
            &["cred", "list", "--json"],
            &[
                "team",
                "list",
                "--tenant",
                "00000000-0000-0000-0000-000000000001",
                "--json",
            ],
            &[
                "team",
                "show",
                "--tenant",
                "00000000-0000-0000-0000-000000000001",
                "00000000-0000-0000-0000-000000000002",
                "--json",
            ],
            &[
                "worker",
                "okr",
                "show",
                "00000000-0000-0000-0000-000000000003",
                "--tenant",
                "00000000-0000-0000-0000-000000000001",
                "--json",
            ],
            &["worker", "deps", "list", "worker-1", "--json"],
            &["worker", "elasticity", "status", "--json"],
            &["sandbox", "e2b", "list", "--json"],
            &[
                "forge",
                "runs",
                "show",
                "00000000-0000-0000-0000-000000000004",
                "--json",
            ],
        ];

        for args in commands {
            let parsed = parsed_debug(args);
            assert!(parsed.contains("json: true"), "{args:?} parsed as {parsed}");
        }
    }
}
