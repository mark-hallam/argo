//! `argo policy …` subcommand enum (Stream `claude-policy-as-code`).
//! Sibling-file pattern; re-exported via cli.rs's glob to keep the
//! cli_subcommands.rs file focused on its own narrow surface.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum PolicyCmd {
    /// List policy files registered to the workspace (queries the
    /// `policy_files` table). Pass `--dir <path>` to list a
    /// directory of YAML files instead (filesystem view).
    List {
        /// Show files in the given directory instead of the
        /// registered set. Useful for previewing a folder before
        /// `argo policy apply`.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Emit a JSON array instead of the human-readable table.
        #[arg(long)]
        json: bool,
    },
    /// Render one policy file (parsed + validated) for inspection.
    /// `<path>` is a path to a YAML file on disk.
    Show {
        /// Path to a YAML file. Reads + parses + validates, then
        /// prints the structured result.
        path: PathBuf,
        /// Emit a single JSON object instead of YAML pretty-print.
        #[arg(long)]
        json: bool,
    },
    /// Parse + validate one policy YAML file. Exits non-zero on
    /// any parse/validation error. CI-friendly: pair with `find`
    /// to validate a whole policy directory in one shot.
    Validate {
        /// Path to a YAML file on disk.
        path: PathBuf,
        /// Emit `{ok, name, errors}` as JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Register a policy YAML file to the workspace. Validates
    /// the file, copies it into `<workspace>/policies/<name>.yaml`,
    /// then records (workspace_id, name, sha256, applied_at) in
    /// the `policy_files` table.
    Apply {
        /// Path to the source YAML file.
        path: PathBuf,
        /// Emit JSON instead of the human-readable confirmation.
        #[arg(long)]
        json: bool,
    },
}
