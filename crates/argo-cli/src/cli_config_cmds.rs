use clap::Subcommand;

/// Configuration inspection commands.
#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    /// Show the effective merged configuration.
    Show {
        /// Dotted key to print, for example `gateway.host`.
        #[arg(long)]
        key: Option<String>,
        /// Emit JSON instead of TOML/text.
        #[arg(long)]
        json: bool,
        /// Annotate TOML lines with their source layer.
        #[arg(long)]
        source: bool,
        /// Reveal secret-looking fields. Also requires ARGO_SHOW_SECRETS=1.
        #[arg(long)]
        show_secrets: bool,
    },
}
