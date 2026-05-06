use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum TenantCmd {
    /// List tenants known to this workspace store.
    List {
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
}
