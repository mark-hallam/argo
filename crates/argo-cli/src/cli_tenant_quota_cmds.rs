//! `argo tenant quota` subcommands.

use clap::{Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum TenantQuotaCmd {
    /// Set or update quota caps for one tenant.
    Set {
        /// Tenant id.
        tenant: String,
        /// Maximum period spend in USD.
        #[arg(long)]
        max_usd: Option<f64>,
        /// Maximum period action count.
        #[arg(long)]
        max_actions: Option<u64>,
        /// Maximum active worker count.
        #[arg(long)]
        max_workers: Option<u64>,
        /// Quota window for spend and action counts.
        #[arg(long, value_enum)]
        period: Option<TenantQuotaPeriodArg>,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Show quota, usage, and remaining headroom for one tenant.
    Show {
        /// Tenant id.
        tenant: String,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// List all configured tenant quotas.
    List {
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Clear a tenant quota row or a single quota field.
    Clear {
        /// Tenant id.
        tenant: String,
        /// Optional field to clear. Omit to remove the whole quota row.
        #[arg(long, value_enum)]
        field: Option<TenantQuotaFieldArg>,
        /// Emit a JSON envelope.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TenantQuotaPeriodArg {
    /// Daily quota window.
    Daily,
    /// Monthly quota window.
    Monthly,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TenantQuotaFieldArg {
    /// Clear the USD cap.
    #[value(name = "max-usd")]
    Usd,
    /// Clear the action-count cap.
    #[value(name = "max-actions")]
    Actions,
    /// Clear the worker-count cap.
    #[value(name = "max-workers")]
    Workers,
}
