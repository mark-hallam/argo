//! Per-domain dispatch sub-fns for the vertical-template /
//! cost-control slice of the Command enum: `Vertical / Genesis /
//! Cron / Messages / Plan / Costs / Budget`. Each fn is the body
//! of one top-level `match Command::*` arm extracted out of
//! `main.rs` (`claude-argo-cli-main-rs-split`); semantics are
//! byte-identical to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn vertical(cmd: cli::VerticalCmd) -> ArgoResult<()> {
    handlers::vertical::run(cmd).await
}

pub(crate) fn genesis(cmd: cli::GenesisCmd) -> ArgoResult<()> {
    handlers::genesis::run(cmd)
}

pub(crate) async fn cron(cmd: cli::CronCmd) -> ArgoResult<()> {
    handlers::cron::run(cmd).await
}

pub(crate) async fn messages(cmd: cli::MessagesCmd) -> ArgoResult<()> {
    handlers::messages::run(cmd).await
}

pub(crate) async fn plan(cmd: cli::PlanCmd) -> ArgoResult<()> {
    match cmd {
        cli::PlanCmd::Generate {
            goal,
            worker,
            llm,
            vertical,
            output,
            json,
        } => {
            handlers::plan_generate::run(
                &goal,
                &worker,
                llm,
                vertical.as_deref(),
                output.as_deref(),
                json,
            )
            .await
        }
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
    }
}

pub(crate) async fn costs(cmd: Option<cli::CostsCmd>) -> ArgoResult<()> {
    match cmd {
        Some(cli::CostsCmd::Forecast {
            horizon,
            lookback,
            format,
        }) => handlers::costs_forecast::run(horizon, &lookback, format).await,
        other => handlers::costs::run(other).await,
    }
}

pub(crate) async fn budget(cmd: cli::BudgetCmd) -> ArgoResult<()> {
    handlers::budget::run(cmd).await
}
