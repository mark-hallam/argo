use clap::Subcommand;

#[path = "cli_tenant_quota_cmds.rs"]
pub mod cli_tenant_quota_cmds;
pub use cli_tenant_quota_cmds::TenantQuotaCmd;

#[derive(Subcommand, Debug)]
pub enum TenantCmd {
    /// List tenants known to this workspace store.
    List {
        /// Include tenants that were archived for audit retention.
        #[arg(long)]
        include_archived: bool,
        /// Emit JSON instead of a human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Show tenant workers, sessions, and recent activity.
    Info {
        /// Tenant id (UUID).
        id: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Show tenant metadata, roster, activity, channels, and rules.
    Show {
        /// Tenant id (UUID).
        id: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Register a new tenant name in the current workspace store.
    Create {
        /// Human-readable tenant/workspace name.
        name: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Rename a tenant id, or use --to to rename its display name.
    Rename {
        /// Tenant id (UUID).
        id: String,
        /// New stable tenant id (UUID).
        new_id: Option<String>,
        /// New human-readable tenant name (legacy display-name rename).
        #[arg(long, value_name = "NEW_NAME")]
        to: Option<String>,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Soft-delete a tenant from default listings while preserving audit history.
    Archive {
        /// Tenant id (UUID).
        id: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Change the operator of record for a tenant.
    Transfer {
        /// Tenant id (UUID).
        id: String,
        /// New operator email.
        #[arg(long, value_name = "NEW_OPERATOR_EMAIL")]
        owner: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Per-tenant resource quotas and period-to-date headroom.
    Quota {
        #[command(subcommand)]
        cmd: TenantQuotaCmd,
    },
    /// Per-tenant gateway rate-limit overrides (Stream
    /// `mission-per-tenant-rate-limiting`). Missing override → use
    /// the `[gateway.rate_limit.tenant]` config default.
    #[command(name = "rate-limit")]
    RateLimit {
        #[command(subcommand)]
        cmd: TenantRateLimitCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum TenantRateLimitCmd {
    /// Set or replace one tenant's per-minute budget.
    Set {
        /// Tenant id (UUID).
        tenant: String,
        /// Per-minute budget (`> 0`).
        #[arg(long)]
        rpm: u32,
        /// Emit JSON instead of a one-line summary.
        #[arg(long)]
        json: bool,
    },
    /// Show one tenant's override, if any.
    Get {
        /// Tenant id (UUID).
        tenant: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// List every stored override.
    List {
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Clear one tenant's override; falls back to the config default.
    Delete {
        /// Tenant id (UUID).
        tenant: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}
