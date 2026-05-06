//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditQueryCmd as AuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{export_ndjson, rows_for_session, tail_rows, verify_all, verify_session};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::io::{BufWriter, Write};
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
            if watch {
                return watch_audit(&store, session.as_deref(), kinds.as_deref()).await;
            }
            let kind_filter = parse_kinds(kinds.as_deref());
            let cutoff = since_minutes
                .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
            if let Some(raw) = session {
                let id = uuid::Uuid::parse_str(&raw).map_err(|e| {
                    ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
                })?;
                let mut rows = rows_for_session(&store, SessionId(id)).await?;
                if let Some(allowed) = &kind_filter {
                    rows.retain(|r| allowed.contains(r.kind.as_str()));
                }
                if let Some(c) = cutoff {
                    rows.retain(|r| r.created_at_micros >= c);
                }
                let pick = if json { "json" } else { format.as_str() };
                match pick {
                    "json" => {
                        for r in &rows {
                            let line = serde_json::to_string(r).map_err(|e| {
                                ArgoError::Provider(format!("serialize row: {e}"))
                            })?;
                            println!("{line}");
                        }
                        return Ok(());
                    }
                    "markdown" => {
                        let summary = summarize_session_rows(&rows);
                        super::audit_md::render_session(&raw, &rows, &summary);
                        return Ok(());
                    }
                    "text" => {}
                    other => {
                        return Err(ArgoError::Provider(format!(
                            "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
                        )));
                    }
                }
                println!("audit chain for session {raw} — {} rows", rows.len());
                for r in &rows {
                    let detail = payload_detail(&r.kind, &r.payload);
                    println!("  seq={:<6} kind={:<22} {detail}", r.seq, r.kind);
                }
                let summary = summarize_session_rows(&rows);
                println!(
                    "\nsummary: {turns} turn(s), {tools} tool call(s) ({errs} error(s)), \
                     {tin} prompt + {tout} completion tokens",
                    turns = summary.turns,
                    tools = summary.tool_calls,
                    errs = summary.tool_errors,
                    tin = summary.tokens_in,
                    tout = summary.tokens_out,
                );
                return Ok(());
            }
            let report = verify_all(&store).await?;
            let mut tail_rows_data = match tail {
                Some(n) => tail_rows(&store, n).await?,
                None => Vec::new(),
            };
            if let Some(allowed) = &kind_filter {
                tail_rows_data.retain(|r| allowed.contains(r.kind.as_str()));
            }
            if let Some(c) = cutoff {
                tail_rows_data.retain(|r| r.created_at_micros >= c);
            }
            let pick = if json { "json" } else { format.as_str() };
            match pick {
                "json" => {
                    let kinds_obj: serde_json::Map<String, serde_json::Value> = report
                        .kind_counts
                        .iter()
                        .map(|(k, v)| (k.clone(), serde_json::json!(v)))
                        .collect();
                    let tail_arr: Vec<_> = tail_rows_data
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "seq": r.seq,
                                "kind": r.kind,
                                "session_id": r.session_id,
                                "payload": r.payload,
                                "created_at_micros": r.created_at_micros,
                            })
                        })
                        .collect();
                    let payload = serde_json::json!({
                        "rows": report.rows,
                        "ok": report.ok,
                        "breaks": report.breaks.len(),
                        "kinds": kinds_obj,
                        "tail": tail_arr,
                    });
                    let serialized = serde_json::to_string(&payload)
                        .map_err(|e| ArgoError::Provider(format!("serialize audit: {e}")))?;
                    println!("{serialized}");
                    return Ok(());
                }
                "markdown" => {
                    super::audit_md::render_global(&report, &tail_rows_data);
                    return Ok(());
                }
                "text" => {}
                other => {
                    return Err(ArgoError::Provider(format!(
                        "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
                    )));
                }
            }
            println!("audit log — {} rows", report.rows);
            let mut kinds: Vec<(&String, &usize)> = report.kind_counts.iter().collect();
            kinds.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            for (kind, count) in kinds {
                println!("  {count:>6}  {kind}");
            }
            if !report.ok {
                println!(
                    "(chain has {} break(s); run `argo audit verify` for detail)",
                    report.breaks.len()
                );
            }
            if !tail_rows_data.is_empty() {
                println!("\nlast {} rows:", tail_rows_data.len());
                for r in &tail_rows_data {
                    let detail = payload_detail(&r.kind, &r.payload);
                    // #691: vertical=<id|-> appears between session
                    // and detail so triage can scan binding without
                    // an extra drill-down. `-` for unbound rows.
                    println!(
                        "  seq={:<6} kind={:<22} session={} vertical={} {detail}",
                        r.seq,
                        r.kind,
                        r.session_id.as_deref().unwrap_or("-"),
                        r.vertical_id.as_deref().unwrap_or("-"),
                    );
                }
            }
            Ok(())
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
            out,
            session,
            since_minutes,
        } => {
            if format != "ndjson" {
                return Err(ArgoError::Provider(format!(
                    "unsupported audit export format '{format}' (only 'ndjson' for now)"
                )));
            }
            let mut writer: Box<dyn Write> = match out.as_deref() {
                Some(path) => {
                    let f = std::fs::File::create(path)
                        .map_err(|e| ArgoError::Provider(format!("create {path}: {e}")))?;
                    Box::new(BufWriter::new(f))
                }
                None => Box::new(BufWriter::new(std::io::stdout())),
            };
            let cutoff = since_minutes
                .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
            let n = if let Some(raw) = session {
                let id = uuid::Uuid::parse_str(&raw).map_err(|e| {
                    ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
                })?;
                let mut rows = rows_for_session(&store, SessionId(id)).await?;
                if let Some(c) = cutoff {
                    rows.retain(|r| r.created_at_micros >= c);
                }
                for row in &rows {
                    let line = serde_json::to_string(row)
                        .map_err(|e| ArgoError::Provider(format!("serialize row: {e}")))?;
                    writer
                        .write_all(line.as_bytes())
                        .and_then(|()| writer.write_all(b"\n"))
                        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
                }
                rows.len()
            } else if let Some(c) = cutoff {
                let mut count = 0_usize;
                export_ndjson(&store, |row| {
                    if row.created_at_micros < c {
                        return Ok(());
                    }
                    count += 1;
                    let line = serde_json::to_string(row)
                        .map_err(|e| ArgoError::Provider(format!("serialize row: {e}")))?;
                    writer
                        .write_all(line.as_bytes())
                        .and_then(|()| writer.write_all(b"\n"))
                        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
                    Ok(())
                })
                .await?;
                count
            } else {
                export_ndjson(&store, |row| {
                    let line = serde_json::to_string(row)
                        .map_err(|e| ArgoError::Provider(format!("serialize row: {e}")))?;
                    writer
                        .write_all(line.as_bytes())
                        .and_then(|()| writer.write_all(b"\n"))
                        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
                    Ok(())
                })
                .await?
            };
            writer
                .flush()
                .map_err(|e| ArgoError::Provider(format!("flush: {e}")))?;
            // Only print the summary if we wrote to a file (stdout
            // already has the rows).
            if out.is_some() {
                eprintln!("audit export: wrote {n} rows");
            }
            Ok(())
        }
        AuditCmd::Breaks { json } => print_breaks(json).await,
        AuditCmd::Count {
            kind,
            since_minutes,
            session,
            json,
        } => print_count(kind, since_minutes, session, json).await,
        AuditCmd::ByKind { since_minutes, json } => {
            super::audit_by_kind::run(since_minutes, json).await
        }
        AuditCmd::ByVertical { since_minutes, json } => {
            super::audit_by_vertical::run(since_minutes, json).await
        }
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
        AuditCmd::ByDayByKind { days, json } => {
            super::audit_by_day_by_kind::run(days, json).await
        }
        AuditCmd::Replay {
            session_id,
            from_seq,
            until,
            format,
        } => super::audit_replay::run(&session_id, from_seq, until.as_deref(), &format).await,
    }
}

use super::audit_count::{print_breaks, print_count};

/// `tail -f`-style live tail of the audit chain. Polls the audit
/// rows every 250ms; prints any rows that newly appeared since
/// the last poll. Optional session filter + kinds filter mirror
/// the static-show path.
async fn watch_audit(
    store: &std::sync::Arc<argo_store::SessionStore>,
    session: Option<&str>,
    kinds: Option<&str>,
) -> ArgoResult<()> {
    let kind_filter = parse_kinds(kinds);
    let session_id = match session {
        Some(raw) => Some(SessionId(uuid::Uuid::parse_str(raw).map_err(|e| {
            ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
        })?)),
        None => None,
    };
    eprintln!("watching audit chain (Ctrl-C to exit)…");
    let mut last_seq = match session_id {
        Some(sid) => argo_audit::rows_for_session(store, sid)
            .await?
            .into_iter()
            .map(|r| r.seq)
            .max()
            .unwrap_or(0),
        None => sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM audit_record")
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("max seq: {e}")))?,
    };
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let rows = match session_id {
            Some(sid) => argo_audit::rows_for_session(store, sid).await?,
            None => argo_audit::tail_rows(store, 100).await?,
        };
        let new_rows: Vec<_> = rows.into_iter().filter(|r| r.seq > last_seq).collect();
        for r in &new_rows {
            if let Some(allowed) = &kind_filter {
                if !allowed.contains(r.kind.as_str()) {
                    continue;
                }
            }
            let detail = payload_detail(&r.kind, &r.payload);
            println!(
                "  seq={:<6} kind={:<22} session={} {detail}",
                r.seq,
                r.kind,
                r.session_id.as_deref().unwrap_or("-")
            );
        }
        if let Some(max) = new_rows.iter().map(|r| r.seq).max() {
            last_seq = last_seq.max(max);
        }
    }
}

// `parse_kinds`, `SessionSummary`, `summarize_session_rows`, and
// `payload_detail` were moved to `audit_helpers.rs` (#609) when this
// file went over the 500-LOC budget. They are re-exported below so
// existing callers (audit_md.rs, etc.) keep compiling.
pub(super) use super::audit_helpers::{
    parse_kinds, payload_detail, summarize_session_rows, SessionSummary,
};
