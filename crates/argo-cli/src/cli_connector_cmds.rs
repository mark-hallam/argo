//! `argo connector` subcommands for operator-managed MCP credentials.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ConnectorCmd {
    /// Authorize one connector and store credentials in this workspace.
    Authorize {
        kind: String,
        /// Token for PAT/bot-token connectors. Omit to prompt without echo.
        #[arg(long)]
        token: Option<String>,
        /// OAuth client id override; otherwise read from env/credentials.toml.
        #[arg(long)]
        client_id: Option<String>,
        /// OAuth client secret override; otherwise read from env/credentials.toml.
        #[arg(long)]
        client_secret: Option<String>,
        /// Add/override OAuth scopes. Repeatable.
        #[arg(long = "scope")]
        scopes: Vec<String>,
        /// Local callback port for OAuth redirects. Use 0 for any free port.
        #[arg(long, default_value_t = 8975)]
        redirect_port: u16,
        /// Do not launch the browser; print the authorization URL instead.
        #[arg(long)]
        no_browser: bool,
        /// OAuth authorization URL override for tests/custom providers.
        #[arg(long, hide = true)]
        auth_url: Option<String>,
        /// OAuth token URL override for tests/custom providers.
        #[arg(long, hide = true)]
        token_url: Option<String>,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Show authorization status for every known connector.
    ListStatus { #[arg(long)] json: bool },
    /// Delete stored credentials for one connector.
    Revoke { kind: String, #[arg(long)] json: bool },
    /// Run a no-op API call using stored credentials.
    Test { kind: String, #[arg(long)] json: bool },
}
