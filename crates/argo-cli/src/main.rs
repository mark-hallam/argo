//! `argo` — the user-facing CLI binary.
//!
//! See `docs/plan/argo-build-plan.md` §19 for the full command tree.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

#[path = "handlers/action_replay.rs"]
mod action_replay;
mod cli;
mod handlers;
mod home;
mod secrets_scrubber;
#[cfg(test)]
mod secrets_scrubber_tests;
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
        Some(Command::Feedback { cmd }) => match cmd {
            cli::FeedbackCmd::Up {
                action_id,
                comment,
                worker,
                json,
            } => handlers::feedback_up::run(&action_id, comment.as_deref(), worker.as_deref(), json)
                .await,
            cli::FeedbackCmd::Down {
                action_id,
                comment,
                worker,
                json,
            } => handlers::feedback_down::run(
                &action_id,
                comment.as_deref(),
                worker.as_deref(),
                json,
            )
            .await,
            cli::FeedbackCmd::List {
                worker,
                vote,
                limit,
                json,
            } => handlers::feedback_list::run(worker.as_deref(), vote.as_deref(), limit, json)
                .await,
        },
        Some(Command::Classify { cmd }) => match cmd {
            cli::ClassifyCmd::Set {
                target,
                class,
                by,
                json,
            } => handlers::classify_set::run(&target, &class, by.as_deref(), json).await,
            cli::ClassifyCmd::List {
                class,
                kind,
                worker,
                expired,
                limit,
                json,
            } => handlers::classify_list::run(
                class.as_deref(),
                kind.as_deref(),
                worker.as_deref(),
                expired,
                limit,
                json,
            )
            .await,
            cli::ClassifyCmd::Rules { json } => handlers::classify_rules::run(json).await,
        },
        Some(Command::Plan { cmd }) => match cmd {
            cli::PlanCmd::Generate {
                goal,
                worker,
                llm,
                vertical,
                output,
                json,
            } => handlers::plan_generate::run(
                &goal,
                &worker,
                llm,
                vertical.as_deref(),
                output.as_deref(),
                json,
            )
            .await,
            cli::PlanCmd::Show { target, json } => handlers::plan_show::run(&target, json).await,
            cli::PlanCmd::Execute {
                yaml_path,
                dry_run,
                json,
            } => handlers::plan_execute::run(&yaml_path, dry_run, json).await,
            cli::PlanCmd::Template { cmd } => match cmd {
                cli::cli_plan_cmds::PlanTemplateCmd::List {
                    grade,
                    limit,
                    json,
                } => handlers::plan_template_list::run(grade.as_deref(), limit, json).await,
                cli::cli_plan_cmds::PlanTemplateCmd::Show { id, json } => {
                    handlers::plan_template_show::run(&id, json).await
                }
                cli::cli_plan_cmds::PlanTemplateCmd::Promote { id, json } => {
                    handlers::plan_template_promote::run(&id, json).await
                }
            },
        },
        Some(Command::Runtime(RuntimeCmd::Doctor {
            cmd: Some(cli::DoctorCmd::Fix { auto, issue, json: fix_json }),
            json,
            ..
        })) => handlers::doctor_fix::run(auto, issue.as_deref(), json || fix_json).await,
        Some(Command::Runtime(RuntimeCmd::Doctor {
            cmd: Some(cli::DoctorCmd::Watch { interval, clear }),
            ..
        })) => handlers::doctor_watch::run(&interval, clear).await,
        Some(Command::Runtime(RuntimeCmd::Doctor {
            cmd: None,
            json,
            format,
        })) => handlers::doctor::run(json, &format).await,
        Some(Command::Runtime(RuntimeCmd::Health { json, format })) => {
            handlers::health::run(json, &format).await
        }
        Some(Command::Runtime(RuntimeCmd::New {
            provider,
            model,
            title,
            vertical,
        })) => handlers::new::run(&provider, &model, title.as_deref(), vertical.as_deref()).await,
        Some(Command::Runtime(RuntimeCmd::Resume { id })) => handlers::resume::run(&id).await,
        Some(Command::Runtime(RuntimeCmd::Chat {
            id,
            user,
            stream,
            interactive,
            max_tool_rounds,
        })) => handlers::chat::run(&id, user.as_deref(), stream, interactive, max_tool_rounds).await,
        Some(Command::Runtime(RuntimeCmd::Steer { session_id, text })) => {
            handlers::steer::run(&session_id, &text).await
        }
        Some(Command::Db { cmd }) => handlers::db::run(cmd).await,
        Some(Command::Workspace { cmd }) => handlers::workspace::run(cmd).await,
        Some(Command::Audit { cmd }) => handlers::audit::run(cmd).await,
        Some(Command::Runtime(RuntimeCmd::Setup {
            check,
            require,
            non_interactive,
            json,
        })) => handlers::setup::run(check, require, non_interactive, json).await,
        Some(Command::Runtime(RuntimeCmd::Onboard {
            non_interactive,
            json,
            config,
        })) => handlers::onboard::run(non_interactive, json, config).await,
        Some(Command::Vertical { cmd }) => handlers::vertical::run(cmd).await,
        Some(Command::Genesis { cmd }) => handlers::genesis::run(cmd),
        Some(Command::Messages { cmd }) => handlers::messages::run(cmd).await,
        Some(Command::Cron { cmd }) => handlers::cron::run(cmd).await,
        Some(Command::Cred { cmd }) => match cmd {
            cli::CredCmd::Set { key, value } => handlers::cred_set::run(&key, value.as_deref()),
            cli::CredCmd::List { reveal } => handlers::cred_list::run(reveal),
            cli::CredCmd::Clear { key } => handlers::cred_clear::clear(&key),
            cli::CredCmd::ClearAll => handlers::cred_clear::clear_all(),
            cli::CredCmd::Export { shell } => handlers::cred_export::run(&shell),
            cli::CredCmd::Import { shell } => handlers::cred_import::run(&shell),
        },
        Some(Command::Backup { cmd }) => match cmd {
            cli::BackupCmd::Create {
                output,
                include_credentials,
                json,
            } => {
                handlers::backup_create::run(output.as_deref(), include_credentials, json).await
            }
            cli::BackupCmd::List { json } => handlers::backup_list::run(json).await,
            cli::BackupCmd::Verify { path, json } => {
                handlers::backup_verify::run(&path, json).await
            }
        },
        Some(Command::Restore { args }) => {
            handlers::backup_restore::run(&args.path, args.into.as_deref(), args.dry_run, args.json)
                .await
        }
        Some(Command::Daemon { cmd }) => match cmd {
            cli::DaemonCmd::Install {
                json,
                dry_run,
                bin,
                launch_agents_dir,
            } => {
                handlers::daemon_install::run(
                    json,
                    dry_run,
                    bin.as_deref(),
                    launch_agents_dir.as_deref(),
                )
                .await
            }
            cli::DaemonCmd::Uninstall {
                json,
                dry_run,
                launch_agents_dir,
            } => {
                handlers::daemon_uninstall::run(json, dry_run, launch_agents_dir.as_deref())
                    .await
            }
        },
        Some(Command::Connector { cmd }) => handlers::connector_authorize::run(cmd).await,
        Some(Command::Work { cmd }) => handlers::work::run(cmd).await,
        Some(Command::Worker { cmd }) => handlers::worker_actions::run(cmd).await,
        Some(Command::Contract { cmd }) => match cmd {
            cli::cli_contract_cmds::ContractCmd::Apply {
                file,
                tenant,
                worker,
                dry_run,
                json,
            } => {
                handlers::contract::apply(
                    &file,
                    tenant.as_deref(),
                    worker.as_deref(),
                    dry_run,
                    json,
                )
                .await
            }
            cli::cli_contract_cmds::ContractCmd::Validate { file, json } => {
                handlers::contract::validate(&file, json).await
            }
            cli::cli_contract_cmds::ContractCmd::Show {
                worker_id,
                tenant,
                json,
            } => handlers::contract::show(&worker_id, &tenant, json).await,
        },
        Some(Command::Approval { cmd }) => match cmd {
            cli::ApprovalCmd::Export(args) => handlers::approval_export::run(args).await,
            cli::ApprovalCmd::History(args) => handlers::approval_history::run(args).await,
            cli::ApprovalCmd::Policy { cmd } => handlers::approval_policy::run(cmd).await,
        },
        Some(Command::Compliance { cmd }) => match cmd {
            cli::ComplianceCmd::Soc2(args) => {
                let cwd = std::env::current_dir()
                    .map_err(|e| argo_types::ArgoError::Provider(format!("current_dir: {e}")))?;
                let home = home::resolve_home(&cwd)?;
                home::ensure_dir(&home)?;
                let store = argo_store::SessionStore::open(&home::db_path(&home)).await?;
                store.migrate().await?;
                let soc2_args = handlers::compliance_soc2::ComplianceSoc2Args {
                    from: args.from,
                    to: args.to,
                    actor: args.actor,
                    action: args.action,
                    tenant: args.tenant,
                    format: args.format,
                };
                let _ = handlers::compliance_soc2::run(soc2_args, &store).await?;
                Ok(())
            }
            cli::ComplianceCmd::Export(args) => handlers::compliance_export::run(args).await,
            cli::ComplianceCmd::Delete(args) => {
                let rc = handlers::compliance_delete::run(args).await?;
                if rc != 0 {
                    std::process::exit(rc);
                }
                Ok(())
            }
        },
        Some(Command::Tenant { cmd }) => handlers::tenant::run(cmd).await,
        Some(Command::Persona { cmd }) => handlers::persona_edit::run(cmd).await,
        Some(Command::Org { cmd }) => handlers::org::run(cmd).await,
        Some(Command::Roster { cmd }) => handlers::roster_list::run(cmd).await,
        Some(Command::Team { cmd }) => handlers::team::run(cmd).await,
        Some(Command::Policy { cmd }) => handlers::policy_handlers::run(cmd).await,
        Some(Command::HandoffRules { cmd }) => handlers::handoff_rules_list::run(cmd).await,
        Some(Command::Duty { cmd }) => handlers::duty_list::run(cmd).await,
        Some(Command::HandoffAuctions { cmd }) => match cmd {
            cli::cli_handoff_cmds::HandoffAuctionsCmd::List {
                status,
                limit,
                json,
            } => handlers::handoff_auctions_list::run(status.as_deref(), limit, json).await,
            cli::cli_handoff_cmds::HandoffAuctionsCmd::Show { id, json } => {
                handlers::handoff_auctions_show::run(&id, json).await
            }
            cli::cli_handoff_cmds::HandoffAuctionsCmd::Cancel { id, json } => {
                handlers::handoff_auctions_cancel::run(&id, json).await
            }
        },
        Some(Command::HandoffPolicy { cmd }) => handlers::handoff_policy::run(cmd).await,
        Some(Command::HandoffStats { args }) => handlers::handoff_stats::run(args).await,
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
        Some(Command::Action { cmd }) => match cmd {
            cli::ActionCmd::Replay {
                action_id,
                executor,
                dry_run,
                bundle_dir,
            } => {
                action_replay::run(cli::ActionCmd::Replay {
                    action_id,
                    executor,
                    dry_run,
                    bundle_dir,
                })
                .await
            }
            cli::ActionCmd::History {
                worker_id,
                since,
                limit,
                include_skill,
                json,
            } => {
                handlers::action_history::run(
                    &worker_id,
                    since.as_deref(),
                    limit,
                    include_skill,
                    json,
                )
                .await
            }
            cli::ActionCmd::Timeline {
                worker_id,
                since,
                bucket_mins,
                json,
            } => handlers::action_timeline::run(&worker_id, &since, bucket_mins, json).await,
            cli::ActionCmd::Recent {
                tenant,
                worker,
                limit,
                json,
            } => handlers::action_recent::run(tenant.as_deref(), worker.as_deref(), limit, json)
                .await,
        },
        Some(Command::Activity { cmd }) => handlers::activity_tail::run(cmd).await,
        Some(Command::Costs {
            cmd: Some(cli::CostsCmd::Forecast { horizon, lookback, format }),
        }) => handlers::costs_forecast::run(horizon, &lookback, format).await,
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
        Some(Command::Runtime(RuntimeCmd::Trace {
            cmd: Some(cli::TraceCmd::Explore { session, format, path }),
            ..
        })) => {
            let fmt = handlers::trace_explore::TraceFormat::parse(&format)?;
            let path_buf = path.map(std::path::PathBuf::from);
            handlers::trace_explore::run(&session, fmt, path_buf.as_deref())
        }
        Some(Command::Runtime(RuntimeCmd::Trace { cmd: None, turn, session })) => {
            handlers::trace::run(turn, session).await
        }
        Some(Command::Incident { cmd }) => match cmd {
            cli::IncidentCmd::New {
                from,
                out_dir,
                force,
            } => handlers::incident::run(from, out_dir, force),
        },
        Some(Command::Runtime(RuntimeCmd::Tui)) => handlers::tui::run().await,
        Some(Command::Runtime(RuntimeCmd::Tools { cmd })) => handlers::tools::run(cmd).await,
        Some(Command::Runtime(RuntimeCmd::Recover { session_id, json })) => {
            handlers::recover::run(&session_id, json).await
        }
        Some(Command::Runtime(RuntimeCmd::RecoverCache { cmd })) => {
            handlers::recover_cache::run(cmd).await
        }
        Some(Command::Subtask { cmd }) => match cmd {
            cli::SubtaskCmd::List {
                session,
                worker,
                parent_goal,
                state,
                limit,
                json,
            } => {
                handlers::subtask_list::run(
                    session.as_deref(),
                    worker.as_deref(),
                    parent_goal.as_deref(),
                    state.as_deref(),
                    limit,
                    json,
                )
                .await
            }
            cli::SubtaskCmd::Show { id, json } => handlers::subtask_show::run(&id, json).await,
            cli::SubtaskCmd::Tree { session, json } => {
                handlers::subtask_tree::run(&session, json).await
            }
        },
        Some(Command::Runtime(RuntimeCmd::Voice { mock_script, json })) => {
            let code = handlers::voice::run(mock_script, json);
            std::process::exit(code);
        }
        Some(Command::Runtime(RuntimeCmd::Latency { format })) => {
            handlers::latency::run(&format).await
        }
        Some(Command::Runtime(RuntimeCmd::Update)) => {
            eprintln!("argo: 'Update' is not yet wired in this build.");
            eprintln!("See ROADMAP.md for the integration-tier plan.");
            std::process::exit(2);
        }
        Some(Command::Cu { cmd }) => handlers::cu_replay::run(cmd).await,
        Some(other) => {
            eprintln!("argo: '{other:?}' is not yet wired in this build.");
            eprintln!("See ROADMAP.md for the integration-tier plan.");
            std::process::exit(2);
        }
    }
}
