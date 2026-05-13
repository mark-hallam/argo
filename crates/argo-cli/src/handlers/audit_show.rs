//! `argo audit show` — chain inspection (per-session or global tail)
//! and `--watch` live tail. Extracted from `audit.rs` (#claude-cli-
//! handlers-audit-split) when that file approached the 500-LOC ceiling.
use argo_audit::{rows_for_session, tail_rows, verify_all};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::sync::Arc;

use super::audit_helpers::{parse_kinds, payload_detail, summarize_session_rows};

pub struct ShowArgs {
    pub tail: Option<u32>,
    pub json: bool,
    pub format: String,
    pub session: Option<String>,
    pub kinds: Option<String>,
    pub since_minutes: Option<i64>,
    pub watch: bool,
}

pub async fn run(store: &Arc<SessionStore>, args: ShowArgs) -> ArgoResult<()> {
    let ShowArgs {
        tail,
        json,
        format,
        session,
        kinds,
        since_minutes,
        watch,
    } = args;
    if watch {
        return watch_audit(store, session.as_deref(), kinds.as_deref()).await;
    }
    let kind_filter = parse_kinds(kinds.as_deref());
    let cutoff = since_minutes
        .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
    if let Some(raw) = session {
        let id = uuid::Uuid::parse_str(&raw)
            .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))?;
        let mut rows = rows_for_session(store, SessionId(id)).await?;
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
                    let line = serde_json::to_string(r)
                        .map_err(|e| ArgoError::Provider(format!("serialize row: {e}")))?;
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
    let report = verify_all(store).await?;
    let mut tail_rows_data = match tail {
        Some(n) => tail_rows(store, n).await?,
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

/// `tail -f`-style live tail of the audit chain. Polls the audit
/// rows every 250ms; prints any rows that newly appeared since
/// the last poll. Optional session filter + kinds filter mirror
/// the static-show path.
async fn watch_audit(
    store: &Arc<SessionStore>,
    session: Option<&str>,
    kinds: Option<&str>,
) -> ArgoResult<()> {
    let kind_filter = parse_kinds(kinds);
    let session_id = match session {
        Some(raw) => Some(SessionId(uuid::Uuid::parse_str(raw).map_err(|e| {
            ArgoError::Schema(format!("invalid session id '{raw}': {e}"))
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
