//! `argo` — the user-facing CLI binary.
//!
//! See `docs/plan/argo-build-plan.md` §19 for the full command tree.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod cli;
mod handlers;
mod home;
mod tracing_setup;

use argo_types::ArgoResult;
use clap::Parser;
use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    tracing_setup::init();
    let cli = Cli::parse();
    let exit_code = match dispatch(cli).await {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("argo: {err}");
            1
        }
    };
    std::process::exit(exit_code);
}

async fn dispatch(cli: Cli) -> ArgoResult<()> {
    match cli.command {
        None => {
            println!(
                "argo {} — interactive chat (run `argo --help` for the command tree)",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        Some(Command::Doctor) => handlers::doctor::run(),
        Some(Command::Db { cmd }) => handlers::db::run(cmd).await,
        Some(Command::Workspace { cmd }) => handlers::workspace::run(cmd).await,
        Some(Command::Audit { cmd }) => handlers::audit::run(cmd).await,
        Some(Command::Cron { cmd }) => handlers::cron::run(cmd).await,
        Some(Command::Costs { cmd }) => handlers::costs::run(cmd),
        Some(Command::Memory { cmd }) => handlers::memory::run(cmd).await,
        Some(Command::Provider { cmd }) => handlers::provider::run(cmd),
        Some(Command::Session { cmd }) => handlers::session::run(cmd).await,
        Some(Command::Skill { cmd }) => handlers::skill::run(cmd).await,
        Some(other) => {
            eprintln!("argo: '{other:?}' is not yet wired in this build.");
            eprintln!("See ROADMAP.md for the integration-tier plan.");
            std::process::exit(2);
        }
    }
}
