//! `argo` — the user-facing CLI binary.
//!
//! See `docs/plan/argo-build-plan.md` §19 for the full command tree.
//!
//! The dispatch arms used to live inline in this file (525 LOC,
//! over the 500-LOC ceiling). Per-domain bodies are now extracted
//! into `main_dispatch_*` siblings (one per cohesive Command-variant
//! group) per `claude-argo-cli-main-rs-split`. Top-level routing
//! (one match arm per `Command` variant + a Runtime sub-router) is
//! all that remains here.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

#[path = "handlers/action_replay.rs"]
mod action_replay;
mod cli;
mod handlers;
mod home;
mod main_dispatch_audit;
mod main_dispatch_dev;
mod main_dispatch_gateway;
mod main_dispatch_session;
mod main_dispatch_setup;
mod main_dispatch_skill;
mod main_dispatch_vertical;
mod main_dispatch_worker;
mod secrets_scrubber;
#[cfg(test)]
mod secrets_scrubber_tests;
mod system_jobs;
mod tracing_setup;

use argo_types::ArgoResult;
use clap::Parser;
use cli::cli_runtime_cmds::RuntimeCmd;
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
    let Some(command) = cli.command else {
        println!(
            "argo {} — interactive chat (run `argo --help` for the command tree)",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    };
    match command {
        // Skill / knowledge / model surfaces.
        Command::Skill { cmd } => main_dispatch_skill::skill(cmd).await,
        Command::Memory { cmd } => main_dispatch_skill::memory(cmd).await,
        Command::Trust { cmd } => main_dispatch_skill::trust(cmd).await,
        Command::Judge { cmd } => main_dispatch_skill::judge(cmd).await,
        Command::Plugin { cmd } => main_dispatch_skill::plugin(cmd),
        Command::Model { cmd } => main_dispatch_skill::model(cmd),
        Command::Mcp { cmd } => main_dispatch_skill::mcp(cmd).await,
        Command::Forge { cmd } => main_dispatch_skill::forge(cmd).await,
        // Session / handoff / subtask / action surfaces.
        Command::Session { cmd } => main_dispatch_session::session(cmd).await,
        Command::Handoff { cmd } => main_dispatch_session::handoff(cmd).await,
        Command::Subtask { cmd } => main_dispatch_session::subtask(cmd).await,
        Command::Action { cmd } => main_dispatch_session::action(cmd).await,
        // Workforce / org / handoff-routing surfaces.
        Command::Worker { cmd } => main_dispatch_worker::worker(cmd).await,
        Command::Persona { cmd } => main_dispatch_worker::persona(cmd).await,
        Command::Contract { cmd } => main_dispatch_worker::contract(cmd).await,
        Command::Roster { cmd } => main_dispatch_worker::roster(cmd).await,
        Command::Team { cmd } => main_dispatch_worker::team(cmd).await,
        Command::Org { cmd } => main_dispatch_worker::org(cmd).await,
        Command::Operator { cmd } => main_dispatch_worker::operator(cmd).await,
        Command::Tenant { cmd } => main_dispatch_worker::tenant(cmd).await,
        Command::Duty { cmd } => main_dispatch_worker::duty(cmd).await,
        Command::Policy { cmd } => main_dispatch_worker::policy(cmd).await,
        Command::HandoffRules { cmd } => main_dispatch_worker::handoff_rules(cmd).await,
        Command::HandoffAuctions { cmd } => main_dispatch_worker::handoff_auctions(cmd).await,
        Command::HandoffPolicy { cmd } => main_dispatch_worker::handoff_policy(cmd).await,
        Command::HandoffStats { args } => main_dispatch_worker::handoff_stats(args).await,
        Command::Workflow { cmd } => main_dispatch_worker::workflow(cmd).await,
        Command::Work { cmd } => main_dispatch_worker::work(cmd).await,
        Command::Agents { cmd } => main_dispatch_worker::agents(cmd),
        // Audit / observability / compliance surfaces.
        Command::Audit { cmd } => main_dispatch_audit::audit(cmd).await,
        Command::Compliance { cmd } => main_dispatch_audit::compliance(cmd).await,
        Command::Approval { cmd } => main_dispatch_audit::approval(cmd).await,
        Command::Activity { cmd } => main_dispatch_audit::activity(cmd).await,
        Command::Trajectory { cmd } => main_dispatch_audit::trajectory(cmd).await,
        Command::Incident { cmd } => main_dispatch_audit::incident(cmd),
        Command::Eval { cmd } => main_dispatch_audit::eval(cmd).await,
        Command::Feedback { cmd } => main_dispatch_audit::feedback(cmd).await,
        Command::Classify { cmd } => main_dispatch_audit::classify(cmd).await,
        Command::Cu { cmd } => main_dispatch_audit::cu(cmd).await,
        // Runtime-edge surfaces (gateway, daemon, connectors,
        // sandbox, security).
        Command::Gateway { cmd } => main_dispatch_gateway::gateway(cmd).await,
        Command::Daemon { cmd } => main_dispatch_gateway::daemon(cmd).await,
        Command::Connector { cmd } => main_dispatch_gateway::connector(cmd).await,
        Command::Sandbox { cmd } => main_dispatch_gateway::sandbox(cmd).await,
        Command::Security { cmd } => main_dispatch_gateway::security(cmd).await,
        // Bootstrap / installer / packaging surfaces.
        Command::Wizard { cmd } => main_dispatch_setup::wizard(cmd).await,
        Command::Provider { cmd } => main_dispatch_setup::provider(cmd).await,
        Command::Db { cmd } => main_dispatch_setup::db(cmd).await,
        Command::Workspace { cmd } => main_dispatch_setup::workspace(cmd).await,
        Command::Route { cmd } => main_dispatch_setup::route(cmd).await,
        Command::Cred { cmd } => main_dispatch_setup::cred(cmd),
        Command::Backup { cmd } => main_dispatch_setup::backup(cmd).await,
        Command::Restore { args } => main_dispatch_setup::restore(args).await,
        Command::SelfUpdate {
            check,
            channel,
            force,
            json,
        } => main_dispatch_setup::self_update(check, channel, force, json).await,
        // Vertical-template / cost-control surfaces.
        Command::Vertical { cmd } => main_dispatch_vertical::vertical(cmd).await,
        Command::Genesis { cmd } => main_dispatch_vertical::genesis(cmd),
        Command::Cron { cmd } => main_dispatch_vertical::cron(cmd).await,
        Command::Messages { cmd } => main_dispatch_vertical::messages(cmd).await,
        Command::Plan { cmd } => main_dispatch_vertical::plan(cmd).await,
        Command::Costs { cmd } => main_dispatch_vertical::costs(cmd).await,
        Command::Budget { cmd } => main_dispatch_vertical::budget(cmd).await,
        // Direct-action operator commands (session lifecycle, runtime
        // diagnostics, system ops). Routed through `dispatch_runtime`
        // so each variant lands in the right per-domain sibling.
        Command::Runtime(rt) => dispatch_runtime(rt).await,
        // Anything still unwired (e.g. `Command::Import`) falls
        // through to the same "not yet wired" message the pre-split
        // dispatch printed.
        other => {
            eprintln!("argo: '{other:?}' is not yet wired in this build.");
            eprintln!("See ROADMAP.md for the integration-tier plan.");
            std::process::exit(2);
        }
    }
}

async fn dispatch_runtime(cmd: RuntimeCmd) -> ArgoResult<()> {
    match cmd {
        // Session lifecycle.
        RuntimeCmd::New {
            provider,
            model,
            title,
            vertical,
        } => main_dispatch_session::runtime_new(provider, model, title, vertical).await,
        RuntimeCmd::Resume { id } => main_dispatch_session::runtime_resume(id).await,
        RuntimeCmd::Chat {
            id,
            user,
            stream,
            interactive,
            max_tool_rounds,
        } => {
            main_dispatch_session::runtime_chat(id, user, stream, interactive, max_tool_rounds)
                .await
        }
        RuntimeCmd::Steer { session_id, text } => {
            main_dispatch_session::runtime_steer(session_id, text).await
        }
        RuntimeCmd::Trace {
            cmd: Some(cli::TraceCmd::Explore { session, format, path }),
            ..
        } => main_dispatch_session::runtime_trace_explore(session, format, path),
        RuntimeCmd::Trace {
            cmd: None,
            turn,
            session,
        } => main_dispatch_session::runtime_trace(turn, session).await,
        RuntimeCmd::Recover { session_id, json } => {
            main_dispatch_session::runtime_recover(session_id, json).await
        }
        RuntimeCmd::RecoverCache { cmd } => {
            main_dispatch_session::runtime_recover_cache(cmd).await
        }
        // Runtime-edge surfaces.
        RuntimeCmd::Health { json, format } => {
            main_dispatch_gateway::runtime_health(json, format).await
        }
        RuntimeCmd::Doctor { cmd, json, format } => {
            main_dispatch_gateway::runtime_doctor(cmd, json, format).await
        }
        RuntimeCmd::Latency { format } => main_dispatch_gateway::runtime_latency(format).await,
        // Bootstrap surfaces.
        RuntimeCmd::Setup {
            check,
            require,
            non_interactive,
            json,
        } => main_dispatch_setup::runtime_setup(check, require, non_interactive, json).await,
        RuntimeCmd::Onboard {
            non_interactive,
            json,
            config,
        } => main_dispatch_setup::runtime_onboard(non_interactive, json, config).await,
        RuntimeCmd::Update => main_dispatch_setup::runtime_update(),
        // Developer-experience surfaces.
        RuntimeCmd::Tui => main_dispatch_dev::runtime_tui().await,
        RuntimeCmd::Tools { cmd } => main_dispatch_dev::runtime_tools(cmd).await,
        RuntimeCmd::Voice { mock_script, json } => main_dispatch_dev::runtime_voice(mock_script, json),
    }
}
