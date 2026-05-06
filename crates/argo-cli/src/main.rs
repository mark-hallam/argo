//! `argo` — the user-facing CLI binary.
//!
//! See `docs/plan/argo-build-plan.md` §19 for the full command tree.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

#[path = "handlers/action_replay.rs"]
mod action_replay;
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
        Some(Command::Agents { cmd }) => handlers::agents::run(cmd),
        Some(Command::Gateway { cmd }) => handlers::gateway::run(cmd).await,
        Some(Command::Doctor { json, format }) => handlers::doctor::run(json, &format).await,
        Some(Command::Health { json, format }) => handlers::health::run(json, &format).await,
        Some(Command::New {
            provider,
            model,
            title,
            vertical,
        }) => handlers::new::run(&provider, &model, title.as_deref(), vertical.as_deref()).await,
        Some(Command::Resume { id }) => handlers::resume::run(&id).await,
        Some(Command::Chat {
            id,
            user,
            stream,
            interactive,
            max_tool_rounds,
        }) => handlers::chat::run(&id, user.as_deref(), stream, interactive, max_tool_rounds).await,
        Some(Command::Steer { session_id, text }) => handlers::steer::run(&session_id, &text).await,
        Some(Command::Db { cmd }) => handlers::db::run(cmd).await,
        Some(Command::Workspace { cmd }) => handlers::workspace::run(cmd).await,
        Some(Command::Audit { cmd }) => handlers::audit::run(cmd).await,
        Some(Command::Setup {
            check,
            require,
            non_interactive,
            json,
        }) => handlers::setup::run(check, require, non_interactive, json).await,
        Some(Command::Vertical { cmd }) => handlers::vertical::run(cmd).await,
        Some(Command::Genesis { cmd }) => handlers::genesis::run(cmd),
        Some(Command::Messages { cmd }) => handlers::messages::run(cmd).await,
        Some(Command::Cron { cmd }) => handlers::cron::run(cmd).await,
        Some(Command::Work { cmd }) => handlers::work::run(cmd).await,
        Some(Command::Worker { cmd }) => handlers::worker_actions::run(cmd).await,
        Some(Command::Tenant { cmd }) => handlers::tenant::run(cmd).await,
        Some(Command::Org { cmd }) => handlers::org::run(cmd).await,
        Some(Command::Workflow { cmd }) => handlers::workflow::run(cmd).await,
        Some(Command::Trajectory { cmd }) => match cmd {
            cli::TrajectoryCmd::Export {
                since,
                until,
                format,
                filter,
                include_screenshots,
                gdpr_redact_emails,
                output,
                json,
            } => handlers::trajectory_export::run(handlers::trajectory_export::ExportArgs {
                since,
                until,
                format,
                filter,
                include_screenshots,
                gdpr_redact_emails,
                output,
                json,
            })
            .await,
        },
        Some(Command::Action { cmd }) => action_replay::run(cmd).await,
        Some(Command::Activity { cmd }) => handlers::activity_tail::run(cmd).await,
        Some(Command::Costs { cmd }) => handlers::costs::run(cmd).await,
        Some(Command::Budget { cmd }) => handlers::budget::run(cmd).await,
        Some(Command::Eval { cmd }) => handlers::eval_run::run(cmd).await,
        Some(Command::Forge { cmd }) => handlers::forge::run(cmd).await,
        Some(Command::Judge { cmd }) => handlers::judge::run(cmd).await,
        Some(Command::Mcp { cmd }) => handlers::mcp::run(cmd).await,
        Some(Command::Memory { cmd }) => handlers::memory::run(cmd).await,
        Some(Command::Model { cmd }) => handlers::model::run(cmd),
        Some(Command::Plugin { cmd }) => handlers::plugin::run(cmd),
        Some(Command::Provider { cmd }) => handlers::provider::run(cmd).await,
        Some(Command::Route { cmd }) => handlers::route::run(cmd).await,
        Some(Command::Session { cmd }) => handlers::session::run(cmd).await,
        Some(Command::Handoff { cmd }) => handlers::session::run_handoff(cmd).await,
        Some(Command::Skill { cmd }) => handlers::skill::run(cmd).await,
        Some(Command::Trace { turn, session }) => handlers::trace::run(turn, session).await,
        Some(Command::Tui) => handlers::tui::run().await,
        Some(Command::Tools { cmd }) => handlers::tools::run(cmd).await,
        Some(other) => {
            eprintln!("argo: '{other:?}' is not yet wired in this build.");
            eprintln!("See ROADMAP.md for the integration-tier plan.");
            std::process::exit(2);
        }
    }
}
