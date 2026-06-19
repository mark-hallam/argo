//! `argo platform` — the DEC-057 Phase 1 platform-staff console CLI.
//!
//! The deliberate, audited seeding path for Skycot's own operators. Staff is
//! granted to an EXISTING user account (the person must have signed up first),
//! and is default-deny — only rows created here authorize the account-level
//! staff surfaces.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum PlatformCmd {
    /// Manage platform-staff grants (Skycot's own operators).
    Staff {
        #[command(subcommand)]
        cmd: StaffCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum StaffCmd {
    /// Grant platform-staff to an existing user account (by email).
    Add {
        /// Email of an existing, signed-up user account.
        email: String,
        /// Role: `support` | `billing_ops` | `super_admin`.
        #[arg(long, default_value = "support")]
        role: String,
        /// Emit the grant as JSON.
        #[arg(long)]
        json: bool,
    },
    /// List every platform-staff grant.
    List {
        /// Emit the roster as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Revoke a platform-staff grant (by email).
    Remove {
        /// Email of the staff member to revoke.
        email: String,
        /// Emit the result as JSON.
        #[arg(long)]
        json: bool,
    },
}
