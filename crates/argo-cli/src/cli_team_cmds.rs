//! `argo team` subcommands (Stream `claude-worker-team-roster-cli`).
//! Sibling-file pattern; cli_command_enum.rs keeps only the
//! single Team variant pointing at the TeamCmd enum here.
//!
//! Operator-facing surface that bridges `org_unit` (org tree) +
//! `roster` / `roster_member` (membership) into a single "team"
//! abstraction. See `crates/argo-store/src/team_roster.rs` for the
//! schema bridge.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum TeamCmd {
    /// List all teams in a tenant + active member counts.
    List {
        /// Tenant uuid this team set belongs to.
        #[arg(long)]
        tenant: String,
        /// Output format: `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Show one team's metadata and active members.
    Show {
        /// Tenant uuid this team belongs to.
        #[arg(long)]
        tenant: String,
        /// Team id (uuid).
        team_id: String,
        /// Output format: `markdown` (default) or `json`.
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    /// Add a worker to a team. Idempotent: re-adding an active
    /// member updates only the role; re-adding a removed member
    /// reactivates the row.
    Add {
        /// Tenant uuid this team belongs to.
        #[arg(long)]
        tenant: String,
        /// Team id (uuid).
        team_id: String,
        /// Worker id (uuid) to add.
        worker_id: String,
        /// Optional free-form role label (`lead`, `clerk`, …).
        #[arg(long)]
        role: Option<String>,
        /// Emit `{ok, team_id, worker_id, role}` as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Soft-delete a worker's membership. Sets `removed_at` so
    /// audit / KPI history rows that reference the (team, worker)
    /// pair stay valid.
    Remove {
        /// Tenant uuid this team belongs to.
        #[arg(long)]
        tenant: String,
        /// Team id (uuid).
        team_id: String,
        /// Worker id (uuid) to remove.
        worker_id: String,
        /// Emit `{ok, team_id, worker_id}` as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Rename a team. Updates both the org-chart row and the
    /// matching roster row.
    Rename {
        /// Tenant uuid this team belongs to.
        #[arg(long)]
        tenant: String,
        /// Team id (uuid).
        team_id: String,
        /// New display name.
        new_name: String,
        /// Emit `{ok, team_id, name}` as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Create a new team (org-chart unit + matching roster row).
    Create {
        /// Tenant uuid this team belongs to.
        #[arg(long)]
        tenant: String,
        /// Operator-friendly team name (must be unique per tenant).
        name: String,
        /// Optional parent team id; the parent must already exist.
        #[arg(long)]
        parent: Option<String>,
        /// Emit `{ok, team_id, name, parent}` as JSON.
        #[arg(long)]
        json: bool,
    },
}
