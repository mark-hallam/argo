//! `argo persona` subcommand tree.
//!
//! v0 ships a single command — `edit` — that opens `$EDITOR` on a
//! TOML view of one persona row and writes the changes back if the
//! parsed TOML is well-formed and the size limits hold.
//!
//! Personas live 1:1 with workers in the `persona` table (see
//! `crates/argo-store/migrations/12__worker_identity.sql`). The
//! lookup key for `edit` is `persona.name`; if more than one row
//! shares the name the operator must disambiguate by tenant.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum PersonaCmd {
    /// Open `$EDITOR` on a TOML view of the named persona.
    /// Validates name regex, voice ≤ 4 KB, bio ≤ 1 KB. On
    /// validation failure the temp file is left in place and its
    /// path is printed so the operator can recover the unsaved
    /// edit.
    ///
    /// `$EDITOR` resolution chain: `$ARGO_EDITOR` > `$VISUAL` >
    /// `$EDITOR` > `vi`.
    Edit {
        /// Persona name (the `name` column on the `persona` row).
        name: String,
        /// Optional tenant id (UUID). Required only when more
        /// than one persona row matches `name`.
        #[arg(long)]
        tenant: Option<String>,
        /// Skip the `$EDITOR` shell-out and read the new TOML
        /// from this path. Used by tests + scripts.
        #[arg(long, value_name = "PATH")]
        in_place: Option<PathBuf>,
        /// Emit a `{ok, name, bytes}` JSON object instead of the
        /// human-readable summary.
        #[arg(long)]
        json: bool,
    },
}
