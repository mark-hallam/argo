//! Session / forge / judge / route / import command enums.
//! Extracted from `cli_subcommands.rs` to keep that file under
//! the 500-LOC budget. Re-exported by `cli_subcommands.rs` so
//! existing `use crate::cli::SessionCmd` callers stay unchanged.
//!
//! Per `codex-loc-budget-followup-wave3` (HARD RULE 6 — file must
//! stay under 500 LOC), the dominant [`SessionCmd`] enum lives in
//! the `cli_session_cmd_main.rs` sibling and is re-exported here so
//! the external path `cli_session_cmds::SessionCmd` is unchanged.
use clap::Subcommand;

#[path = "cli_session_cmd_main.rs"]
mod cli_session_cmd_main;

pub use cli_session_cmd_main::SessionCmd;

// `ForgeCmd` + `ForgeRunsCmd` live in `cli_forge_cmds.rs` —
// extracted to keep this file under the 500-LOC ceiling once
// `ForgeCmd::Daemon` was added for the v1.1 daemon split.
pub use super::cli_forge_cmds::{ForgeCmd, ForgeRunsCmd};

#[derive(Subcommand, Debug)]
pub enum JudgeCmd {
    Verdict {
        id: String,
    },
    Override {
        id: String,
        #[arg(long)]
        to: String,
    },
    /// Run the judge-vs-human disagreement calibration suite over a
    /// directory of pre-labelled fixtures. Reports agreement rate
    /// overall and per tag bucket. With `--bail-below <rate>`, exits
    /// non-zero when the overall rate falls below the threshold —
    /// suitable for CI gating on judge-model upgrades. The default
    /// threshold of 0.85 is a starting point; tune per-tenant.
    /// The judge transport is built with `temperature = 0` for
    /// reproducibility — see `argo-judge::eval_suite` for details.
    EvalDisagreement {
        /// Directory of `*.json` fixtures. Defaults to the bundled
        /// `crates/argo-judge/fixtures/disagreement/` directory.
        #[arg(long)]
        fixtures: Option<std::path::PathBuf>,
        /// Provider manifest id used to construct the judge transport.
        #[arg(long, default_value = "anthropic")]
        provider: String,
        /// Judge model id (e.g. `claude-sonnet-4-6`).
        #[arg(long, default_value = "claude-sonnet-4-6")]
        model: String,
        /// Emit a structured JSON envelope instead of a text summary.
        #[arg(long)]
        json: bool,
        /// Exit non-zero if the overall agreement rate is strictly
        /// below this value. Range `[0.0, 1.0]`. Default `0.85`.
        #[arg(long, default_value_t = 0.85)]
        bail_below: f32,
    },
}

#[derive(Subcommand, Debug)]
pub enum RouteCmd {
    Inspect { session_id: String },
    Bench,
}

#[derive(Subcommand, Debug)]
pub enum ImportCmd {
    Hermes { path: String },
}

#[derive(Subcommand, Debug)]
pub enum EvalCmd {
    Validate { path: String },
}

#[derive(Subcommand, Debug)]
pub enum HandoffCmd {
    List {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        worker: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u32,
        #[arg(long)]
        json: bool,
    },
    Show {
        id: String,
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Cancel {
        id: String,
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

// `MessagesCmd` was extracted to cli_messages_cmds.rs (#624)
// when this file went over the 500-LOC budget. Re-exported by
// the wildcard glob in cli.rs so callers don't need to know
// the new path.
pub use super::cli_messages_cmds::MessagesCmd;
