//! `argo action` debugging commands.
use clap::{Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ActionCmd {
    /// Re-run a completed action against a fresh executor into a replay bundle.
    Replay {
        /// `action_id` from `argo worker actions list`.
        action_id: String,
        /// Override the executor kind instead of using the action's manifest default.
        #[arg(long, value_enum)]
        executor: Option<ActionReplayExecutor>,
        /// Print the resolved replay plan without executing or writing a bundle.
        #[arg(long)]
        dry_run: bool,
        /// Exact replay bundle directory to write. Defaults to a sibling replay timestamp dir.
        #[arg(long)]
        bundle_dir: Option<PathBuf>,
    },
    /// Show recent actions for one worker, newest first.
    History {
        worker_id: String,
        #[arg(long)]
        since: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        include_skill: bool,
        #[arg(long)]
        json: bool,
    },
    /// Render an activity histogram for one worker.
    Timeline {
        worker_id: String,
        #[arg(long, default_value = "24h")]
        since: String,
        #[arg(long, default_value_t = 5)]
        bucket_mins: u32,
        #[arg(long)]
        json: bool,
    },
    /// Show recent actions across workers.
    Recent {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        worker: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ActionReplayExecutor {
    Human,
    Api,
    #[value(name = "computer_use", alias = "computer-use")]
    ComputerUse,
}

impl ActionReplayExecutor {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Api => "api",
            Self::ComputerUse => "computer_use",
        }
    }
}
