//! Group-wise dispatchers for [`super::worker_actions::run`]. Each
//! helper owns a disjoint subset of [`WorkerCmd`] variants and stays
//! under the 80-LOC budget; `run` itself becomes a flat fan-out
//! over the groups.

use crate::cli::cli_worker_channels_cmds::WorkerChannelsCmd;
use crate::cli::cli_worker_cmds::{
    WorkerArchiveCmd, WorkerCharterCmd, WorkerCmd, WorkerElasticityCmd, WorkerItemsCmd,
    WorkerKpiCmd, WorkerOkrCmd, WorkerPolicyCmd,
};
use crate::cli::cli_worker_deps_cmds::WorkerDepsCmd;
use argo_types::ArgoResult;

/// Top-level dispatch over [`WorkerCmd`]. Splits the long match into
/// group dispatchers grouped by domain so each callee stays within
/// the 80-LOC ceiling.
pub(super) async fn dispatch(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Create { .. }
        | WorkerCmd::Onboard { .. }
        | WorkerCmd::Show { .. }
        | WorkerCmd::List { .. }
        | WorkerCmd::Update { .. }
        | WorkerCmd::Resume { .. }
        | WorkerCmd::Doctor { .. }
        | WorkerCmd::Anomalies { .. }
        | WorkerCmd::Rehydrate { .. }
        | WorkerCmd::Delete { .. }
        | WorkerCmd::Promote { .. }
        | WorkerCmd::Level { .. } => dispatch_lifecycle(cmd).await,
        WorkerCmd::Freeze {
            worker_id,
            reason,
            json,
        } => super::worker_freeze::freeze(&worker_id, &reason, json).await,
        WorkerCmd::Unfreeze { worker_id, json } => {
            super::worker_freeze::unfreeze(&worker_id, json).await
        }
        WorkerCmd::Costs(_)
        | WorkerCmd::Fleet { .. }
        | WorkerCmd::Portrait { .. }
        | WorkerCmd::Restarts { .. }
        | WorkerCmd::Skills { .. }
        | WorkerCmd::Success { .. }
        | WorkerCmd::Review { .. }
        | WorkerCmd::Budget(_)
        | WorkerCmd::Elasticity(_) => dispatch_reporting(cmd).await,
        WorkerCmd::Policy(_)
        | WorkerCmd::Charter(_)
        | WorkerCmd::Archive(_)
        | WorkerCmd::Channels(_) => dispatch_governance(cmd).await,
        WorkerCmd::Kpi(_) | WorkerCmd::Deps(_) | WorkerCmd::Okr(_) | WorkerCmd::Items(_) => {
            dispatch_metrics(cmd).await
        }
        WorkerCmd::Actions(actions_cmd) => {
            super::worker_actions_actions_dispatch_arms::run(actions_cmd).await
        }
    }
}

async fn dispatch_lifecycle(cmd: WorkerCmd) -> ArgoResult<()> {
    match &cmd {
        WorkerCmd::Create { .. }
        | WorkerCmd::Onboard { .. }
        | WorkerCmd::Show { .. }
        | WorkerCmd::List { .. }
        | WorkerCmd::Doctor { .. } => dispatch_lifecycle_init(cmd).await,
        _ => dispatch_lifecycle_modify(cmd).await,
    }
}

async fn dispatch_lifecycle_init(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Create {
            vertical,
            persona,
            charter,
            builder,
            voice_preamble,
            duties,
            fs_root,
            json,
        } => {
            super::worker_create::run(
                &vertical,
                &persona,
                &charter,
                builder.as_deref(),
                voice_preamble.as_deref(),
                &duties,
                fs_root.as_deref(),
                json,
            )
            .await
        }
        WorkerCmd::Onboard {
            persona_spec,
            id,
            role_name,
            json,
        } => {
            super::worker_onboard::run(&persona_spec, id.as_deref(), role_name.as_deref(), json)
                .await
        }
        WorkerCmd::Show {
            worker_id,
            tenant,
            json,
        } => super::worker_show::run(&tenant, &worker_id, json).await,
        WorkerCmd::List {
            tenant,
            persona,
            json,
        } => super::worker_list::list(&tenant, persona.as_deref(), json).await,
        WorkerCmd::Doctor { vertical, json } => {
            super::worker_doctor::run(vertical.as_deref(), json).await
        }
        _ => unreachable!("dispatch_lifecycle_init called with non-init variant"),
    }
}

async fn dispatch_lifecycle_modify(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Update {
            worker_id,
            auto_approval_json,
            json,
        } => {
            super::worker_update::update_auto_approval(&worker_id, &auto_approval_json, json).await
        }
        WorkerCmd::Resume {
            worker_id,
            tenant,
            reason,
            json,
        } => super::worker_resume::run(&worker_id, &tenant, reason.as_deref(), json).await,
        WorkerCmd::Anomalies {
            worker_id,
            since_minutes,
            limit,
            json,
        } => super::worker_anomalies::run(&worker_id, since_minutes, limit, json).await,
        WorkerCmd::Rehydrate {
            worker_id,
            tenant,
            json,
        } => super::worker_archive::rehydrate(&tenant, &worker_id, json).await,
        WorkerCmd::Delete {
            worker_id,
            tenant,
            force,
            json,
        } => super::worker_archive::delete(&tenant, &worker_id, force, json).await,
        WorkerCmd::Promote {
            worker_id,
            tenant,
            to,
            force,
            reason,
            json,
        } => {
            super::worker_promote::promote(
                &worker_id,
                &tenant,
                to.as_deref(),
                force,
                reason.as_deref(),
                json,
            )
            .await
        }
        WorkerCmd::Level {
            worker_id,
            tenant,
            json,
        } => super::worker_promote::level(&worker_id, &tenant, json).await,
        _ => unreachable!("dispatch_lifecycle_modify called with non-modify variant"),
    }
}

async fn dispatch_reporting(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Costs(costs_cmd) => super::worker_costs::run(costs_cmd).await,
        WorkerCmd::Fleet {
            tenant,
            limit,
            since,
            json,
        } => super::worker_fleet::run(tenant.as_deref(), limit, since.as_deref(), json).await,
        WorkerCmd::Portrait {
            worker_id,
            tenant,
            json,
        } => super::worker_portrait::run(&worker_id, &tenant, json).await,
        WorkerCmd::Restarts { worker, since } => {
            super::worker_restarts::run(worker.as_deref(), since.as_deref()).await
        }
        WorkerCmd::Skills {
            worker_id,
            tenant,
            available_only,
            json,
        } => super::worker_skills::run(&worker_id, &tenant, available_only, json).await,
        WorkerCmd::Success {
            worker_id,
            since,
            bucket,
            format,
        } => super::worker_metrics::run(&worker_id, since.as_deref(), &bucket, &format).await,
        WorkerCmd::Review {
            worker_id,
            period,
            since,
            format,
        } => super::worker_review::run(&worker_id, &period, since.as_deref(), &format).await,
        WorkerCmd::Budget(budget_cmd) => super::worker_budget::run(budget_cmd).await,
        WorkerCmd::Elasticity(cmd) => match cmd {
            WorkerElasticityCmd::Status { json, text } => {
                super::worker_elasticity::status(json || !text)
            }
            WorkerElasticityCmd::Config { text } => super::worker_elasticity::config(!text),
        },
        _ => unreachable!("dispatch_reporting called with non-reporting WorkerCmd variant"),
    }
}

async fn dispatch_governance(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Policy(WorkerPolicyCmd::Show {
            worker_id,
            tenant,
            json,
        }) => super::worker_list::policy_show(&tenant, &worker_id, json).await,
        WorkerCmd::Charter(charter_cmd) => dispatch_charter(charter_cmd).await,
        WorkerCmd::Archive(archive_cmd) => dispatch_archive(archive_cmd).await,
        WorkerCmd::Channels(channels_cmd) => dispatch_channels(channels_cmd).await,
        _ => unreachable!("dispatch_governance called with non-governance WorkerCmd variant"),
    }
}

async fn dispatch_charter(cmd: WorkerCharterCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCharterCmd::Set {
            worker_id,
            tenant,
            file,
            kpis,
            json,
        } => super::worker_charter::set(&tenant, &worker_id, &file, kpis.as_deref(), json).await,
        WorkerCharterCmd::Get {
            worker_id,
            tenant,
            md,
            json,
        } => super::worker_charter::get(&tenant, &worker_id, md, json).await,
        WorkerCharterCmd::Clear {
            worker_id,
            tenant,
            json,
        } => super::worker_charter::clear(&tenant, &worker_id, json).await,
    }
}

async fn dispatch_archive(cmd: WorkerArchiveCmd) -> ArgoResult<()> {
    match cmd {
        WorkerArchiveCmd::Add {
            worker_id,
            tenant,
            reason,
            principal,
            json,
        } => {
            super::worker_archive::archive(
                &tenant,
                &worker_id,
                reason.as_deref(),
                principal.as_deref(),
                json,
            )
            .await
        }
        WorkerArchiveCmd::List { tenant, json } => super::worker_archive::list(&tenant, json).await,
    }
}

async fn dispatch_channels(cmd: WorkerChannelsCmd) -> ArgoResult<()> {
    match cmd {
        WorkerChannelsCmd::Add {
            worker_id,
            spec,
            tenant,
            json,
        } => super::worker_channels::add(&tenant, &worker_id, &spec, json).await,
        WorkerChannelsCmd::Remove {
            worker_id,
            channel_id,
            tenant,
            json,
        } => super::worker_channels::remove(&tenant, &worker_id, &channel_id, json).await,
        WorkerChannelsCmd::List {
            worker_id,
            tenant,
            json,
        } => super::worker_channels::list(&tenant, &worker_id, json).await,
    }
}

async fn dispatch_metrics(cmd: WorkerCmd) -> ArgoResult<()> {
    match cmd {
        WorkerCmd::Kpi(kpi_cmd) => dispatch_kpi(kpi_cmd).await,
        WorkerCmd::Deps(deps_cmd) => dispatch_deps(deps_cmd).await,
        WorkerCmd::Okr(okr_cmd) => dispatch_okr(okr_cmd).await,
        WorkerCmd::Items(items_cmd) => dispatch_items(items_cmd).await,
        _ => unreachable!("dispatch_metrics called with non-metrics WorkerCmd variant"),
    }
}

async fn dispatch_kpi(cmd: WorkerKpiCmd) -> ArgoResult<()> {
    match cmd {
        WorkerKpiCmd::Record {
            worker_id,
            kpi,
            value,
            tenant,
            note,
            json,
        } => {
            super::worker_kpi::record(&worker_id, &kpi, value, &tenant, note.as_deref(), json).await
        }
        WorkerKpiCmd::List {
            worker_id,
            tenant,
            kpi,
            since,
            json,
        } => {
            super::worker_kpi::list(&worker_id, &tenant, kpi.as_deref(), since.as_deref(), json)
                .await
        }
        WorkerKpiCmd::Series {
            worker_id,
            kpi,
            tenant,
            bucket,
            json,
        } => super::worker_kpi::series(&worker_id, &kpi, &tenant, &bucket, json).await,
    }
}

async fn dispatch_deps(cmd: WorkerDepsCmd) -> ArgoResult<()> {
    match cmd {
        WorkerDepsCmd::Add {
            a,
            b,
            kind,
            note,
            json,
        } => super::worker_deps::add(&a, &b, &kind, note.as_deref(), json).await,
        WorkerDepsCmd::Remove { a, b, kind, json } => {
            super::worker_deps::remove(&a, &b, &kind, json).await
        }
        WorkerDepsCmd::List {
            worker_id,
            direction,
            format,
            json,
        } => super::worker_deps::list(&worker_id, &direction, json, &format).await,
        WorkerDepsCmd::Graph { format } => super::worker_deps::graph(&format).await,
        WorkerDepsCmd::Impact { worker_id, json } => {
            super::worker_deps::impact(&worker_id, json).await
        }
    }
}

async fn dispatch_okr(cmd: WorkerOkrCmd) -> ArgoResult<()> {
    match cmd {
        WorkerOkrCmd::Set {
            worker_id,
            tenant,
            period,
            title,
            description,
            kr,
            json,
        } => {
            super::worker_okr::set(
                &worker_id,
                &tenant,
                &period,
                &title,
                description.as_deref(),
                &kr,
                json,
            )
            .await
        }
        WorkerOkrCmd::Show {
            worker_id,
            tenant,
            period,
            format,
            json,
        } => super::worker_okr::show(&worker_id, &tenant, period.as_deref(), json, &format).await,
        WorkerOkrCmd::Progress {
            worker_id,
            tenant,
            period,
            json,
        } => super::worker_okr::progress(&worker_id, &tenant, period.as_deref(), json).await,
    }
}

async fn dispatch_items(cmd: WorkerItemsCmd) -> ArgoResult<()> {
    match cmd {
        WorkerItemsCmd::List {
            vertical,
            state,
            limit,
            json,
        } => super::worker_actions_items::list(vertical.as_deref(), &state, limit, json).await,
        WorkerItemsCmd::Show {
            item_id,
            vertical,
            json,
        } => super::worker_actions_items::show(&item_id, vertical.as_deref(), json).await,
    }
}
