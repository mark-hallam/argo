use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ProviderCmd {
    /// List provider manifests under `manifests/providers/`.
    List {
        /// Output as a JSON array of `{id, family, base_url}` rows.
        /// Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Render one provider manifest's full details (header, auth,
    /// transport, capabilities, models, discovery).
    Show {
        id: String,
        /// Output the parsed manifest as a single JSON object.
        #[arg(long)]
        json: bool,
    },
    Validate {
        manifest: String,
    },
    /// Validate every manifest under `manifests/providers/`. Exits
    /// non-zero if any fails parsing — wire into CI to catch
    /// drift early.
    ValidateAll,
    Test {
        provider: String,
    },
    /// Print prompt-cache hit-rate metrics aggregated per provider+model
    /// (mission-prompt-cache-hit-rate-observability). Numbers come from
    /// the in-process counters incremented on every Anthropic response;
    /// the gateway also exposes them under the standard `/metrics`
    /// endpoint.
    CacheStats {
        /// Filter to one provider id (e.g. `anthropic`).
        #[arg(long)]
        provider: Option<String>,
        /// Filter to one model id (e.g. `claude-opus-4-7`).
        #[arg(long)]
        model: Option<String>,
        /// Emit a JSON array instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ModelCmd {
    List {
        /// Output as JSON. Equivalent to `--format json`.
        #[arg(long)]
        json: bool,
        /// Output format: `text` (default), `json`, or `markdown`.
        /// `--json` wins if both are passed.
        #[arg(long, default_value = "text")]
        format: String,
    },
    Set { model: String },
    Test { model: String },
}
