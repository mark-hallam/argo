//! `argo skill search` subcommand arguments.
use clap::Args;

#[derive(Args, Debug)]
pub struct SkillSearchCmd {
    /// Search terms matched against skill name, description, tenant, and OCI ref.
    /// Optional when `--vertical <id>` is set — listing all skills in a
    /// vertical doesn't need a search term.
    #[arg(default_value = "")]
    pub query: String,
    /// Minimum trust grade: builtin, published_verified, tenant,
    /// published_community, or learned_unreviewed.
    #[arg(long = "trust-min")]
    pub trust_min: Option<String>,
    /// Restrict results to one tenant or publisher namespace.
    #[arg(long)]
    pub tenant: Option<String>,
    /// Maximum number of ranked rows to return.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    /// Output a JSON array.
    #[arg(long)]
    pub json: bool,
    /// Filter to skills whose `vertical_id` column matches the
    /// given vertical id. Switches to a store-side query (mission
    /// claim 4 — skills as a per-vertical library).
    #[arg(long)]
    pub vertical: Option<String>,
    /// Pair with `--vertical <id>`: also surface skills attached to
    /// sessions in that vertical (via `session_skill`), regardless
    /// of how the skill row tagged itself. No-op without `--vertical`.
    #[arg(long = "include-used-by-sessions")]
    pub include_used_by_sessions: bool,
}
