//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditQueryCmd as AuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{verify_all, verify_session};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

pub async fn run(cmd: AuditCmd) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    match cmd {
        AuditCmd::Verify {
            from: _,
            to: _,
            session,
            json,
            format,
        } => {
            let scope = session.clone();
            let report = match session {
                Some(raw) => {
                    let id = uuid::Uuid::parse_str(&raw).map_err(|e| {
                        ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
                    })?;
                    verify_session(&store, SessionId(id)).await?
                }
                None => verify_all(&store).await?,
            };
            let pick = if json { "json" } else { format.as_str() };
            match pick {
                "json" => {
                    let breaks_arr: Vec<_> = report
                        .breaks
                        .iter()
                        .map(|b| {
                            serde_json::json!({
                                "seq": b.seq,
                                "kind": b.kind,
                                "reason": b.reason,
                            })
                        })
                        .collect();
                    let payload = serde_json::json!({
                        "ok": report.ok,
                        "rows": report.rows,
                        "kinds": report.kind_counts.len(),
                        "breaks": breaks_arr,
                    });
                    let serialized = serde_json::to_string(&payload)
                        .map_err(|e| ArgoError::Provider(format!("serialize verify: {e}")))?;
                    println!("{serialized}");
                }
                "markdown" => {
                    println!("# Audit verify");
                    println!();
                    if let Some(s) = &scope {
                        println!("- **Scope**: session `{s}`");
                    } else {
                        println!("- **Scope**: all sessions");
                    }
                    println!(
                        "- **Status**: {}",
                        if report.ok { "OK ✓" } else { "**BROKEN**" }
                    );
                    println!("- **Rows**: {}", report.rows);
                    println!("- **Kinds**: {}", report.kind_counts.len());
                    println!("- **Breaks**: {}", report.breaks.len());
                    if !report.breaks.is_empty() {
                        println!();
                        println!("## Break detail");
                        println!();
                        println!("| seq | kind | reason |");
                        println!("| ---: | --- | --- |");
                        for b in &report.breaks {
                            let reason = b.reason.replace('|', "\\|");
                            println!("| {} | `{}` | {reason} |", b.seq, b.kind);
                        }
                    }
                }
                "text" => {
                    if report.ok {
                        println!(
                            "audit chain ok: {} rows verified across {} kinds",
                            report.rows,
                            report.kind_counts.len()
                        );
                    } else {
                        println!(
                            "audit chain broken: {} break(s) detected across {} rows",
                            report.breaks.len(),
                            report.rows
                        );
                        for b in &report.breaks {
                            println!("  seq {} kind={} ({})", b.seq, b.kind, b.reason);
                        }
                    }
                }
                other => {
                    return Err(ArgoError::Provider(format!(
                        "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
                    )));
                }
            }
            if report.ok {
                Ok(())
            } else {
                Err(ArgoError::Provider(
                    "audit chain verification failed".into(),
                ))
            }
        }
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
        AuditCmd::Export {
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
        } => {
            super::audit_export::run(
                &store,
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
                },
            )
            .await
        }
        AuditCmd::Report {
            format,
            output,
            since,
            until,
            worker_id,
            tenant,
            json,
        } => {
            super::audit_report::run(
                &store,
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
    }
}

use super::audit_count::{print_breaks, print_count};

// Helpers live in `audit_helpers.rs` (#609); the `Show` arm and
// `watch_audit` were moved to `audit_show.rs` (claude-cli-handlers-
// audit-split) when this file approached the 500-LOC budget. Only
// the symbols `audit_md.rs` reaches for via `super::audit::*` are
// re-exported here.
pub(super) use super::audit_helpers::{payload_detail, SessionSummary};
