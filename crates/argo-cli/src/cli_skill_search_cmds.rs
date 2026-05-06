//! `argo skill search` subcommand arguments.
use clap::Args;

#[derive(Args, Debug)]
pub struct SkillSearchCmd {
    /// Search terms matched against skill name, description, tenant, and OCI ref.
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
}
