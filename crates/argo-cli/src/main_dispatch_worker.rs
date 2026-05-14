//! Per-domain dispatch sub-fns for the org / workforce slice of the
//! Command enum: `Worker / Persona / Contract / Roster / Team / Org /
//! Operator / Tenant / Duty / Policy / HandoffRules / HandoffAuctions /
//! HandoffPolicy / HandoffStats / Workflow / Work / Agents`. Each fn
//! is the body of one top-level `match Command::*` arm extracted out
//! of `main.rs` (`claude-argo-cli-main-rs-split`); semantics are
//! byte-identical to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn worker(cmd: cli::WorkerCmd) -> ArgoResult<()> {
    handlers::worker_actions::run(cmd).await
}

pub(crate) async fn persona(cmd: cli::PersonaCmd) -> ArgoResult<()> {
    handlers::persona_edit::run(cmd).await
}

pub(crate) async fn contract(cmd: cli::cli_contract_cmds::ContractCmd) -> ArgoResult<()> {
    match cmd {
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
    }
}

pub(crate) async fn roster(cmd: cli::cli_roster_cmds::RosterCmd) -> ArgoResult<()> {
    handlers::roster_list::run(cmd).await
}

pub(crate) async fn team(cmd: cli::cli_team_cmds::TeamCmd) -> ArgoResult<()> {
    handlers::team::run(cmd).await
}

pub(crate) async fn org(cmd: cli::cli_org_cmds::OrgCmd) -> ArgoResult<()> {
    handlers::org::run(cmd).await
}

pub(crate) async fn operator(cmd: cli::OperatorCmd) -> ArgoResult<()> {
    handlers::operator_risk_overrides::dispatch(cmd).await
}

pub(crate) async fn tenant(cmd: cli::TenantCmd) -> ArgoResult<()> {
    handlers::tenant::run(cmd).await
}

pub(crate) async fn duty(cmd: cli::DutyCmd) -> ArgoResult<()> {
    handlers::duty_list::run(cmd).await
}

pub(crate) async fn policy(cmd: cli::cli_policy_cmds::PolicyCmd) -> ArgoResult<()> {
    handlers::policy_handlers::run(cmd).await
}

pub(crate) async fn handoff_rules(cmd: cli::cli_handoff_cmds::HandoffRulesCmd) -> ArgoResult<()> {
    handlers::handoff_rules_list::run(cmd).await
}

pub(crate) async fn handoff_auctions(
    cmd: cli::cli_handoff_cmds::HandoffAuctionsCmd,
) -> ArgoResult<()> {
    match cmd {
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
    }
}

pub(crate) async fn handoff_policy(
    cmd: cli::cli_handoff_cmds::HandoffPolicyCmd,
) -> ArgoResult<()> {
    handlers::handoff_policy::run(cmd).await
}

pub(crate) async fn handoff_stats(
    args: cli::cli_handoff_cmds::HandoffStatsArgs,
) -> ArgoResult<()> {
    handlers::handoff_stats::run(args).await
}

pub(crate) async fn workflow(cmd: cli::WorkflowCmd) -> ArgoResult<()> {
    handlers::workflow::run(cmd).await
}

pub(crate) async fn work(cmd: cli::WorkCmd) -> ArgoResult<()> {
    handlers::work::run(cmd).await
}

pub(crate) fn agents(cmd: Option<cli::AgentsCmd>) -> ArgoResult<()> {
    handlers::agents::run(cmd)
}
