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
