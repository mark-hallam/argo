//! `argo trust …` subcommand enum (Stream
//! `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3
//! Claim 4 #81).
//!
//! Two surfaces:
//! - `argo trust graph --from <tenant>` prints outgoing edges from
//!   the named tenant with weight, observation count, and last-
//!   updated timestamp.
//! - `argo trust composed --from <a> --to <c>` BFS-walks the graph
//!   and prints the composed score + path taken (default cap 3
//!   hops; signal at 4+ hops is too lossy to be meaningful as a v1
//!   default).
//!
//! Lifted into its own file so the top-level `Command` enum + the
//! `cli.rs` reexports stay under the 500-LOC per-file invariant.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum TrustCmd {
    /// Print neighbors (outgoing trust edges) for a tenant.
    Graph {
        /// Tenant id to print outgoing edges for.
        #[arg(long)]
        from: String,
        /// How many days of observations to materialise the graph
        /// from. Default 90 — matches the brief's decay window.
        #[arg(long, default_value_t = 90)]
        days_back: u32,
        /// Emit one JSON array of `{to_tenant, weight,
        /// observation_count, last_updated_at_rfc3339}` rows
        /// instead of the human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Print the composed trust score from `--from` to `--to` plus
    /// the winning BFS path.
    Composed {
        /// Trusting tenant id.
        #[arg(long)]
        from: String,
        /// Trusted tenant id.
        #[arg(long)]
        to: String,
        /// Maximum hops to compose across. Default 3; values above
        /// 3 are honoured but discouraged (signal decays).
        #[arg(long, default_value_t = 3)]
        max_hops: usize,
        /// How many days of observations to materialise the graph
        /// from. Default 90.
        #[arg(long, default_value_t = 90)]
        days_back: u32,
        /// Emit one JSON `{from, to, max_hops, score, path}` object
        /// instead of the human-readable status line.
        #[arg(long)]
        json: bool,
    },
}
