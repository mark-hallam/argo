use clap::Subcommand;

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
    /// Register a new tenant name in the current workspace store.
    Create {
        /// Human-readable tenant/workspace name.
        name: String,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Rename a tenant's display name while preserving its stable id.
    Rename {
        /// Tenant id (UUID).
        id: String,
        /// New human-readable tenant name.
        #[arg(long, value_name = "NEW_NAME")]
        to: String,
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
}
