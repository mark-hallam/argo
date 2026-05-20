//! `argo doctor` — config / workspace sanity report. Default
//! output is human-readable text; `--json` emits a single JSON
//! object that monitoring pipelines can ingest.
use crate::cli::DoctorCmd;
use argo_types::ArgoResult;

#[path = "doctor_audit_chain.rs"]
mod doctor_audit_chain;
#[path = "doctor_channels.rs"]
mod doctor_channels;
#[path = "doctor_h_stack.rs"]
mod doctor_h_stack;
#[path = "doctor_network.rs"]
mod doctor_network;
#[path = "doctor_runtime.rs"]
mod doctor_runtime;
#[path = "doctor_summary.rs"]
mod doctor_summary;
#[path = "doctor_workers.rs"]
mod doctor_workers;

pub(crate) use doctor_runtime::collect;
#[cfg(test)]
pub(crate) use doctor_runtime::collect_with_skip;
pub use doctor_runtime::run;
#[cfg(test)]
pub(crate) use doctor_summary::SummaryState;
pub(crate) use doctor_summary::{print_build_info, summarize};

/// Single entry point for every `argo doctor [...]` invocation —
/// keeps the per-subcommand wiring out of `main.rs` so the dispatch
/// can grow without ballooning the binary entry file.
pub async fn dispatch(
    cmd: Option<DoctorCmd>,
    json: bool,
    format: String,
    skip_network: bool,
) -> ArgoResult<()> {
    match cmd {
        None => run(json, &format, skip_network).await,
        Some(DoctorCmd::Fix {
            auto,
            issue,
            json: fix_json,
        }) => super::doctor_fix::run(auto, issue.as_deref(), json || fix_json).await,
        Some(DoctorCmd::Watch { interval, clear }) => {
            super::doctor_watch::run(&interval, clear).await
        }
        Some(DoctorCmd::WalStats { json: ws_json }) => {
            super::doctor_storage::wal_stats(json || ws_json).await
        }
        Some(DoctorCmd::Approvals {
            since,
            json: cmd_json,
        }) => super::doctor_approvals::run(&since, json || cmd_json).await,
        Some(DoctorCmd::InstallHooks {
            force,
            json: ih_json,
        }) => super::doctor_install_hooks::run(force, json || ih_json),
        Some(DoctorCmd::Throttles { json: t_json }) => {
            super::doctor_throttles::run(json || t_json).await
        }
    }
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod doctor_tests;
