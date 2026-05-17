//! Worker v3 action ledger CLI dispatch.
//!
//! The wire schema (`ActionState`, `ActionRecord`) lives in
//! `worker_actions_schema.rs`; the marketplace `ActionType` enum
//! lives in `worker_actions_type.rs`. Both are re-exported below so
//! callers keep using `super::worker_actions::ActionState` etc.

use crate::cli::cli_worker_channels_cmds::WorkerChannelsCmd;
use crate::cli::cli_worker_cmds::{
    WorkerArchiveCmd, WorkerCharterCmd, WorkerCmd, WorkerElasticityCmd, WorkerItemsCmd,
    WorkerKpiCmd, WorkerOkrCmd, WorkerPolicyCmd,
};
use crate::cli::cli_worker_deps_cmds::WorkerDepsCmd;
use argo_telemetry::span_context::{enter_context, LogContext};
use argo_types::ArgoResult;

// Re-export short() so the existing tests in `worker_actions_tests.rs`
// can keep calling it via `super::short` after the
// `claude-worker-actions-split` refactor.
#[cfg(test)]
pub(crate) use super::worker_actions_query::short;

// `ActionState` + `ActionRecord` live in `worker_actions_schema.rs`;
// `ActionType` lives in `worker_actions_type.rs`. They are re-exported
// here so callers (and the unit tests below via `use super::*`) keep
// using `super::worker_actions::ActionState` / `ActionRecord` /
// `ActionType` unchanged.
#[allow(unused_imports)]
pub(crate) use super::worker_actions_schema::{ActionRecord, ActionState};
#[allow(unused_imports)]
pub(crate) use super::worker_actions_type::ActionType;

// ───────────────────────── CLI ───────────────────────────

pub async fn run(cmd: WorkerCmd) -> ArgoResult<()> {
    // Worker-loop iteration boundary: every CLI invocation of a
    // `worker <subcommand>` is one logical iteration. Extract the
    // worker_id from the subcommand args (when present) so log lines
    // emitted under the handler carry it via the
    // ContextInjectorLayer.
    let _worker_ctx = enter_context(LogContext {
        worker_id: worker_id_from_cmd(&cmd),
        ..LogContext::default()
    });
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
        WorkerCmd::Policy(policy_cmd) => match policy_cmd {
            WorkerPolicyCmd::Show {
                worker_id,
                tenant,
                json,
            } => super::worker_list::policy_show(&tenant, &worker_id, json).await,
        },
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
        WorkerCmd::Doctor { vertical, json } => {
            super::worker_doctor::run(vertical.as_deref(), json).await
        }
        WorkerCmd::Anomalies {
            worker_id,
            since_minutes,
            limit,
            json,
        } => super::worker_anomalies::run(&worker_id, since_minutes, limit, json).await,
        WorkerCmd::Charter(charter_cmd) => match charter_cmd {
            WorkerCharterCmd::Set {
                worker_id,
                tenant,
                file,
                kpis,
                json,
            } => {
                super::worker_charter::set(&tenant, &worker_id, &file, kpis.as_deref(), json).await
            }
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
        },
        WorkerCmd::Archive(archive_cmd) => match archive_cmd {
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
            WorkerArchiveCmd::List { tenant, json } => {
                super::worker_archive::list(&tenant, json).await
            }
        },
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
        WorkerCmd::Channels(channels_cmd) => match channels_cmd {
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
        },
        WorkerCmd::Budget(budget_cmd) => super::worker_budget::run(budget_cmd).await,
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
        WorkerCmd::Restarts { worker, since } => {
            super::worker_restarts::run(worker.as_deref(), since.as_deref()).await
        }
        WorkerCmd::Skills {
            worker_id,
            tenant,
            available_only,
            json,
        } => super::worker_skills::run(&worker_id, &tenant, available_only, json).await,
        WorkerCmd::Elasticity(cmd) => match cmd {
            WorkerElasticityCmd::Status { text } => super::worker_elasticity::status(!text),
            WorkerElasticityCmd::Config { text } => super::worker_elasticity::config(!text),
        },
        WorkerCmd::Kpi(kpi_cmd) => match kpi_cmd {
            WorkerKpiCmd::Record {
                worker_id,
                kpi,
                value,
                tenant,
                note,
                json,
            } => {
                super::worker_kpi::record(&worker_id, &kpi, value, &tenant, note.as_deref(), json)
                    .await
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
        },
        WorkerCmd::Deps(deps_cmd) => match deps_cmd {
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
            } => super::worker_deps::list(&worker_id, &direction, &format).await,
            WorkerDepsCmd::Graph { format } => super::worker_deps::graph(&format).await,
            WorkerDepsCmd::Impact { worker_id, json } => {
                super::worker_deps::impact(&worker_id, json).await
            }
        },
        WorkerCmd::Okr(okr_cmd) => match okr_cmd {
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
            } => super::worker_okr::show(&worker_id, &tenant, period.as_deref(), &format).await,
            WorkerOkrCmd::Progress {
                worker_id,
                tenant,
                period,
                json,
            } => super::worker_okr::progress(&worker_id, &tenant, period.as_deref(), json).await,
        },
        WorkerCmd::Items(items_cmd) => match items_cmd {
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
        },
        WorkerCmd::Actions(actions_cmd) => {
            super::worker_actions_actions_dispatch_arms::run(actions_cmd).await
        }
    }
}

/// Pull the `worker_id` (if any) off the dispatched subcommand so the
/// [`enter_context`] guard at the top of [`run`] carries it. Returns
/// `None` for subcommands that don't address a specific worker (fleet
/// listing, elasticity status, etc.) — the
/// [`ContextInjectorLayer`](argo_telemetry::span_context) then emits
/// the configured `worker_id=-` marker.
fn worker_id_from_cmd(cmd: &WorkerCmd) -> Option<String> {
    match cmd {
        WorkerCmd::Show { worker_id, .. }
        | WorkerCmd::Portrait { worker_id, .. }
        | WorkerCmd::Update { worker_id, .. }
        | WorkerCmd::Resume { worker_id, .. }
        | WorkerCmd::Anomalies { worker_id, .. }
        | WorkerCmd::Rehydrate { worker_id, .. }
        | WorkerCmd::Delete { worker_id, .. }
        | WorkerCmd::Promote { worker_id, .. }
        | WorkerCmd::Level { worker_id, .. }
        | WorkerCmd::Skills { worker_id, .. } => Some(worker_id.clone()),
        WorkerCmd::Onboard { id, .. } => id.clone(),
        _ => None,
    }
}

#[cfg(test)]
#[path = "worker_actions_tests.rs"]
mod tests;
