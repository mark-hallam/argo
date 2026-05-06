//! `argo org` subcommands (V1.3 — `claude-org-awareness`).
//! Sibling-file pattern; cli_subcommands.rs holds only the
//! single Org variant pointing at the OrgCmd enum here.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum OrgCmd {
    /// Render the org chart for `<tenant>` as an ASCII tree.
    Tree {
        /// Tenant id whose org chart should be rendered.
        #[arg(long)]
        tenant: String,
        /// Emit JSON (one object per unit/member) instead of the
        /// ASCII tree.
        #[arg(long)]
        json: bool,
    },
    /// Resolve a name-or-alias to its canonical OrgMember row.
    Member {
        /// Tenant id whose org chart should be queried.
        #[arg(long)]
        tenant: String,
        /// Canonical name OR alias to resolve.
        name_or_alias: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Walk the manager chain from a member toward the org root.
    ReportsTo {
        /// Tenant id.
        #[arg(long)]
        tenant: String,
        /// Member name-or-alias to start the walk from.
        member: String,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
    /// Bulk-import an org chart from a YAML doc.
    Ingest {
        /// Path to a YAML file whose top-level keys are
        /// `tenant`, `org`, `units`, and `members`.
        path: PathBuf,
        /// Emit JSON summary instead of a one-line text recap.
        #[arg(long)]
        json: bool,
    },
}
