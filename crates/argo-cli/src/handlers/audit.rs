//! `argo audit` — chain inspection / verification / export.
use crate::cli::AuditCmd;
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
        } => {
            let report = match session {
                Some(raw) => {
                    let id = uuid::Uuid::parse_str(&raw).map_err(|e| {
                        ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
                    })?;
                    verify_session(&store, SessionId(id)).await?
                }
                None => verify_all(&store).await?,
            };
            if json {
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
                if !report.ok {
                    return Err(ArgoError::Provider(
                        "audit chain verification failed".into(),
                    ));
                }
                return Ok(());
            }
            if report.ok {
                println!(
                    "audit chain ok: {} rows verified across {} kinds",
                    report.rows,
                    report.kind_counts.len()
                );
                Ok(())
            } else {
                println!(
                    "audit chain broken: {} break(s) detected across {} rows",
                    report.breaks.len(),
                    report.rows
                );
                for b in &report.breaks {
                    println!("  seq {} kind={} ({})", b.seq, b.kind, b.reason);
                }
                Err(ArgoError::Provider(
                    "audit chain verification failed".into(),
                ))
            }
        }
        AuditCmd::Show {
            tail,
            json,
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
                if json {
                    for r in &rows {
                        let line = serde_json::to_string(r).map_err(|e| {
                            ArgoError::Provider(format!("serialize row: {e}"))
                        })?;
                        println!("{line}");
                    }
                    return Ok(());
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
            if json {
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
                    println!(
                        "  seq={:<6} kind={:<22} session={} {detail}",
                        r.seq,
                        r.kind,
                        r.session_id.as_deref().unwrap_or("-"),
                    );
                }
            }
            Ok(())
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

/// Parse the `--kinds <a,b,c>` flag into a `HashSet` for fast row
/// filtering. Returns `None` when the flag is absent (i.e. no filter).
/// Empty / whitespace-only entries are skipped.
fn parse_kinds(spec: Option<&str>) -> Option<std::collections::HashSet<String>> {
    let raw = spec?;
    let set: std::collections::HashSet<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    Some(set)
}

/// Summary counts derived from a session's audit chain.
struct SessionSummary {
    turns: u32,
    tool_calls: u32,
    tool_errors: u32,
    tokens_in: u64,
    tokens_out: u64,
}

fn summarize_session_rows(rows: &[argo_audit::ExportedRow]) -> SessionSummary {
    let mut s = SessionSummary {
        turns: 0,
        tool_calls: 0,
        tool_errors: 0,
        tokens_in: 0,
        tokens_out: 0,
    };
    for r in rows {
        match r.kind.as_str() {
            "TurnStart" => s.turns += 1,
            "ToolCall" => s.tool_calls += 1,
            "ToolResult"
                if r.payload.get("is_error").and_then(|v| v.as_bool()) == Some(true) =>
            {
                s.tool_errors += 1;
            }
            "AssistantMessage" => {
                if let Some(n) = r.payload.get("tokens_in").and_then(|v| v.as_u64()) {
                    s.tokens_in += n;
                }
                if let Some(n) = r.payload.get("tokens_out").and_then(|v| v.as_u64()) {
                    s.tokens_out += n;
                }
            }
            _ => {}
        }
    }
    s
}

/// One-line "interesting field" extract from a row's payload, keyed
/// by audit kind. Keeps `audit show --tail` readable without the
/// reader having to dig the payload column out of sqlite.
fn payload_detail(kind: &str, payload: &serde_json::Value) -> String {
    let s = |k: &str| payload.get(k).and_then(|v| v.as_str()).unwrap_or("");
    let n = |k: &str| payload.get(k).and_then(serde_json::Value::as_u64);
    let b = |k: &str| payload.get(k).and_then(serde_json::Value::as_bool);
    match kind {
        "TurnStart" => format!("provider={} model={}", s("provider"), s("model")),
        "AssistantMessage" => format!(
            "round={} tool_calls={} in={} out={}",
            n("round").unwrap_or(0),
            n("tool_calls").unwrap_or(0),
            n("tokens_in").unwrap_or(0),
            n("tokens_out").unwrap_or(0),
        ),
        "ToolCall" => format!("tool={} call={}", s("tool"), s("call_id")),
        "ToolResult" => {
            let tag = if b("is_error") == Some(true) {
                " [error]"
            } else {
                ""
            };
            format!(
                "tool={} duration_ms={}{tag}",
                s("tool"),
                n("duration_ms").unwrap_or(0)
            )
        }
        "TurnEnd" | "Halt" => String::new(),
        _ => String::new(),
    }
}
