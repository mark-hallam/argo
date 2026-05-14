//! Per-domain dispatch sub-fns for the audit / observability /
//! compliance slice of the Command enum: `Audit / Compliance /
//! Approval / Activity / Trajectory / Incident / Eval / Feedback /
//! Classify / Cu`. Each fn is the body of one top-level
//! `match Command::*` arm extracted out of `main.rs`
//! (`claude-argo-cli-main-rs-split`); semantics are byte-identical
//! to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use crate::home;
use argo_types::ArgoResult;

pub(crate) async fn audit(cmd: cli::AuditQueryCmd) -> ArgoResult<()> {
    handlers::audit::run(cmd).await
}

pub(crate) async fn compliance(cmd: cli::ComplianceCmd) -> ArgoResult<()> {
    match cmd {
        cli::ComplianceCmd::Soc2(args) => {
            let cwd = std::env::current_dir()
                .map_err(|e| argo_types::ArgoError::Io(format!("current_dir: {e}")))?;
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
    }
}

pub(crate) async fn approval(cmd: cli::ApprovalCmd) -> ArgoResult<()> {
    match cmd {
        cli::ApprovalCmd::Export(args) => handlers::approval_export::run(args).await,
        cli::ApprovalCmd::History(args) => handlers::approval_history::run(args).await,
        cli::ApprovalCmd::Policy { cmd } => handlers::approval_policy::run(cmd).await,
    }
}

pub(crate) async fn activity(cmd: cli::ActivityCmd) -> ArgoResult<()> {
    handlers::activity_tail::run(cmd).await
}

pub(crate) async fn trajectory(cmd: cli::TrajectoryCmd) -> ArgoResult<()> {
    match cmd {
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
    }
}

pub(crate) fn incident(cmd: cli::IncidentCmd) -> ArgoResult<()> {
    match cmd {
        cli::IncidentCmd::New {
            from,
            out_dir,
            force,
        } => handlers::incident::run(from, out_dir, force),
        cli::IncidentCmd::Bundle {
            since,
            output,
            log_tail,
        } => {
            let path = handlers::incident_bundle::run(handlers::incident_bundle::BundleArgs {
                since,
                output,
                log_tail,
            })?;
            println!("wrote incident bundle: {}", path.display());
            Ok(())
        }
    }
}

pub(crate) async fn eval(cmd: cli::cli_eval_cmds::EvalCmd) -> ArgoResult<()> {
    handlers::eval_run::run(cmd).await
}

pub(crate) async fn feedback(cmd: cli::FeedbackCmd) -> ArgoResult<()> {
    match cmd {
        cli::FeedbackCmd::Up {
            action_id,
            comment,
            worker,
            skill,
            json,
        } => handlers::feedback_up::run(
            &action_id,
            comment.as_deref(),
            worker.as_deref(),
            skill.as_deref(),
            json,
        )
        .await,
        cli::FeedbackCmd::Down {
            action_id,
            comment,
            worker,
            skill,
            json,
        } => handlers::feedback_down::run(
            &action_id,
            comment.as_deref(),
            worker.as_deref(),
            skill.as_deref(),
            json,
        )
        .await,
        cli::FeedbackCmd::List {
            worker,
            vote,
            limit,
            json,
        } => handlers::feedback_list::run(worker.as_deref(), vote.as_deref(), limit, json).await,
    }
}

pub(crate) async fn classify(cmd: cli::ClassifyCmd) -> ArgoResult<()> {
    match cmd {
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
        } => {
            handlers::classify_list::run(
                class.as_deref(),
                kind.as_deref(),
                worker.as_deref(),
                expired,
                limit,
                json,
            )
            .await
        }
        cli::ClassifyCmd::Rules { json } => handlers::classify_rules::run(json).await,
    }
}

pub(crate) async fn cu(cmd: cli::cli_cu_cmds::CuCmd) -> ArgoResult<()> {
    handlers::cu_replay::run(cmd).await
}
