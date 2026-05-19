//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditQueryCmd as AuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run(cmd: AuditCmd) -> ArgoResult<()> {
    let store = open_legacy_store().await?;
    match cmd {
        AuditCmd::Verify {
            from: _,
            to: _,
            session,
            json,
            format,
        } => super::audit_verify::run(&store, session, json, format).await,
        AuditCmd::VerifyChain { json } => super::audit_verify_chain::run(&store, json).await,
        AuditCmd::Show {
            tail,
            json,
            format,
            session,
            kinds,
            since_minutes,
            watch,
        } => {
            super::audit_show::run(
                &store,
                super::audit_show::ShowArgs {
                    tail,
                    json,
                    format,
                    session,
                    kinds,
                    since_minutes,
                    watch,
                },
            )
            .await
        }
        AuditCmd::Query {
            decision,
            worker,
            session,
            escalation,
            since,
            until,
            limit,
            json,
        } => {
            super::audit_query::run(
                &store,
                super::audit_query::QueryArgs {
                    decision,
                    worker,
                    session,
                    escalation,
                    since,
                    until,
                    limit,
                    json,
                },
            )
            .await
        }
        other => dispatch_export_or_aggregate(&store, other).await,
    }
}

async fn dispatch_export_or_aggregate(
    store: &Arc<SessionStore>,
    cmd: AuditCmd,
) -> ArgoResult<()> {
    match cmd {
        AuditCmd::Export { .. } => dispatch_export(store, cmd).await,
        AuditCmd::Report { .. } => dispatch_report(store, cmd).await,
        AuditCmd::Archive {
            retention_days,
            prune,
            hard_keep_days,
            archive_dir,
            json,
        } => {
            super::audit_archive::run_archive(
                retention_days,
                prune,
                hard_keep_days,
                archive_dir.as_deref(),
                json,
            )
            .await
        }
        AuditCmd::ListArchives { archive_dir, json } => {
            super::audit_archive::run_list(archive_dir.as_deref(), json).await
        }
        AuditCmd::VerifyArchive {
            bundle_id,
            archive_dir,
            json,
        } => super::audit_archive::run_verify(&bundle_id, archive_dir.as_deref(), json).await,
        other => dispatch_rollups(other).await,
    }
}

async fn dispatch_export(store: &Arc<SessionStore>, cmd: AuditCmd) -> ArgoResult<()> {
    let AuditCmd::Export {
        format,
        bundle_out,
        since,
        until,
        worker_id,
        tenant,
        session,
        since_minutes,
        gdpr_redact_emails,
        json,
        kinds,
    } = cmd
    else {
        unreachable!("dispatch_export expects AuditCmd::Export");
    };
    super::audit_export::run(
        store,
        super::audit_export::ExportArgs {
            format,
            bundle_out,
            worker_id,
            tenant,
            session,
            since,
            until,
            since_minutes,
            gdpr_redact_emails,
            json,
            kinds,
        },
    )
    .await
}

async fn dispatch_report(store: &Arc<SessionStore>, cmd: AuditCmd) -> ArgoResult<()> {
    let AuditCmd::Report {
        format,
        output,
        since,
        until,
        worker_id,
        tenant,
        json,
    } = cmd
    else {
        unreachable!("dispatch_report expects AuditCmd::Report");
    };
    super::audit_report::run(
        store,
        super::audit_report::ReportArgs {
            format,
            output,
            since,
            until,
            worker_id,
            tenant,
            json,
        },
    )
    .await
}

async fn dispatch_rollups(cmd: AuditCmd) -> ArgoResult<()> {
    match cmd {
        AuditCmd::Breaks { json } => print_breaks(json).await,
        AuditCmd::Count {
            kind,
            since_minutes,
            session,
            json,
        } => print_count(kind, since_minutes, session, json).await,
        AuditCmd::ByKind {
            since_minutes,
            json,
        } => super::audit_by_kind::run(since_minutes, json).await,
        AuditCmd::ByVertical {
            since_minutes,
            json,
        } => super::audit_by_vertical::run(since_minutes, json).await,
        AuditCmd::ByDay { days, json } => super::audit_by_day::run(days, json).await,
        AuditCmd::BySession {
            limit,
            since_minutes,
            json,
        } => super::audit_by_session::run(limit, since_minutes, json).await,
        AuditCmd::ByTool {
            since_minutes,
            json,
        } => super::audit_by_tool::run(since_minutes, json).await,
        AuditCmd::ToolErrors {
            since_minutes,
            json,
        } => super::audit_tool_errors::run(since_minutes, json).await,
        AuditCmd::ByDayByKind { days, json } => super::audit_by_day_by_kind::run(days, json).await,
        AuditCmd::Replay {
            session_id,
            from_seq,
            until,
            format,
        } => super::audit_replay::run(&session_id, from_seq, until.as_deref(), &format).await,
        // The verify/show/query/export/report/archive arms are handled
        // in `run` and `dispatch_export_or_aggregate`; if a new
        // variant lands without a dispatch wiring this guard fires.
        _ => unreachable!("audit subcommand dispatched in earlier helper"),
    }
}

async fn open_legacy_store() -> ArgoResult<Arc<SessionStore>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    Ok(store)
}

use super::audit_count::{print_breaks, print_count};

// Helpers live in `audit_helpers.rs` (#609); the `Show` arm and
// `watch_audit` were moved to `audit_show.rs` (claude-cli-handlers-
// audit-split) when this file approached the 500-LOC budget. Only
// the symbols `audit_md.rs` reaches for via `super::audit::*` are
// re-exported here.
pub(super) use super::audit_helpers::{payload_detail, SessionSummary};
