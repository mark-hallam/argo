//! Filter / sort / render helpers extracted from
//! [`super::session_list::list_sessions`]. Each phase lives in its
//! own helper so `list_sessions` stays a thin orchestrator and
//! every helper sits well under the 80-LOC ceiling.
use argo_store::SessionHeader;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashMap;

/// Trim `sessions` in place by the time / provider / model /
/// vertical / status / title filter dimensions. Returns a typed
/// error on an unknown `--status` value to keep the caller's
/// error-surface unchanged.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_session_filters(
    sessions: &mut Vec<SessionHeader>,
    since_minutes: Option<i64>,
    older_than_minutes: Option<i64>,
    provider: Option<&str>,
    model: Option<&str>,
    vertical: Option<&str>,
    status: &str,
    title_contains: Option<&str>,
) -> ArgoResult<()> {
    if let Some(minutes) = since_minutes {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
        sessions.retain(|s| s.started_at >= cutoff);
    }
    if let Some(minutes) = older_than_minutes {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
        sessions.retain(|s| s.started_at < cutoff);
    }
    if let Some(want) = provider {
        sessions.retain(|s| s.provider == want);
    }
    if let Some(want) = model {
        sessions.retain(|s| s.model == want);
    }
    if let Some(want) = vertical {
        sessions.retain(|s| s.vertical_id.as_deref() == Some(want));
    }
    match status {
        "open" => sessions.retain(|s| s.ended_at.is_none()),
        "ended" => sessions.retain(|s| s.ended_at.is_some()),
        "all" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --status '{other}' — use open|ended|all"
            )));
        }
    }
    if let Some(needle) = title_contains {
        let lower = needle.to_lowercase();
        sessions.retain(|s| {
            s.title
                .as_deref()
                .map(|t| t.to_lowercase().contains(&lower))
                .unwrap_or(false)
        });
    }
    Ok(())
}

/// Sort `sessions` by `--sort-by` and filter by spend bounds in
/// place. Returns a typed error on an unknown sort key so the
/// caller's error-surface stays unchanged.
pub(super) fn apply_sort_and_spent(
    sessions: &mut Vec<SessionHeader>,
    spend_by_sid: &HashMap<Vec<u8>, u64>,
    sort_by: &str,
    spent_min: Option<u64>,
    spent_max: Option<u64>,
) -> ArgoResult<()> {
    match sort_by {
        "started-at" => {} // already sorted by store
        "spend" => {
            sessions.sort_by(|a, b| {
                let sa = spend_by_sid
                    .get(a.id.0.as_bytes().as_slice())
                    .copied()
                    .unwrap_or(0);
                let sb = spend_by_sid
                    .get(b.id.0.as_bytes().as_slice())
                    .copied()
                    .unwrap_or(0);
                sb.cmp(&sa)
            });
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --sort-by '{other}' — use started-at|spend"
            )));
        }
    }
    if spent_min.is_some() || spent_max.is_some() {
        sessions.retain(|s| {
            let spent = spend_by_sid
                .get(s.id.0.as_bytes().as_slice())
                .copied()
                .unwrap_or(0);
            if let Some(min) = spent_min {
                if spent < min {
                    return false;
                }
            }
            if let Some(max) = spent_max {
                if spent > max {
                    return false;
                }
            }
            true
        });
    }
    Ok(())
}

/// `format=json` — one JSON object per line, augmented with
/// `spent_usd_micros` + `tool_call_count`.
pub(super) fn render_json(
    sessions: &[SessionHeader],
    spend_by_sid: &HashMap<Vec<u8>, u64>,
    tool_calls_by_sid: &HashMap<Vec<u8>, u64>,
) -> ArgoResult<()> {
    for s in sessions {
        let spent = spend_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        let tcs = tool_calls_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        let mut value = serde_json::to_value(s)
            .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
        if let serde_json::Value::Object(ref mut m) = value {
            m.insert("spent_usd_micros".into(), serde_json::Value::from(spent));
            m.insert("tool_call_count".into(), serde_json::Value::from(tcs));
        }
        let line = serde_json::to_string(&value)
            .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
        println!("{line}");
    }
    Ok(())
}

/// `format=markdown` — header line + pipe-delimited table.
pub(super) fn render_markdown(
    sessions: &[SessionHeader],
    spend_by_sid: &HashMap<Vec<u8>, u64>,
    tool_calls_by_sid: &HashMap<Vec<u8>, u64>,
) {
    println!(
        "# Sessions ({} match{})",
        sessions.len(),
        if sessions.len() == 1 { "" } else { "es" }
    );
    println!();
    if sessions.is_empty() {
        return;
    }
    // #690: vertical column (after model, before spend) so
    // operators can scan which sessions are bound at a glance.
    // Unbound rows render as `-` to keep the table tight.
    println!("| status | session_id | provider | model | vertical | spend (USD) | tools | title |");
    println!("| :---: | --- | --- | --- | --- | ---: | ---: | --- |");
    for s in sessions {
        let spent = spend_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        let tcs = tool_calls_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        let marker = if s.ended_at.is_none() { "●" } else { "○" };
        let title = s.title.as_deref().unwrap_or("").replace('|', "\\|");
        let vertical = s.vertical_id.as_deref().unwrap_or("-");
        println!(
            "| {marker} | `{}` | `{}` | `{}` | `{vertical}` | ${:.4} | {tcs} | {title} |",
            s.id.0,
            s.provider,
            s.model,
            spent as f64 / 1_000_000.0
        );
    }
}

/// `format=text` (default) — 7-column header + per-session line.
/// Caller resolved the empty-sessions branch already.
pub(super) fn render_text(
    sessions: &[SessionHeader],
    spend_by_sid: &HashMap<Vec<u8>, u64>,
    tool_calls_by_sid: &HashMap<Vec<u8>, u64>,
) {
    // #690: 7-column header now includes vertical between model
    // and spend. `-` for unbound keeps the column compact when
    // most rows are legacy.
    println!(
        "  {sid:<38} {prov:<12} {model:<24} {vert:<10} {spend:>10} {tools:>5} title",
        sid = "session_id",
        prov = "provider",
        model = "model",
        vert = "vertical",
        spend = "spend",
        tools = "tools",
    );
    for s in sessions {
        let spent = spend_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        let tcs = tool_calls_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        // ● for open (no ended_at), ○ for ended — mirrors the
        // TUI session-list marker so `argo session list` and the
        // viewer agree at a glance.
        let marker = if s.ended_at.is_none() { "●" } else { "○" };
        let vertical = s.vertical_id.as_deref().unwrap_or("-");
        println!(
            "{marker} {:<38} {:<12} {:<24} {:<10} {:>10.4} {:>5} {}",
            s.id.0,
            short(&s.provider, 12),
            short(&s.model, 24),
            short(vertical, 10),
            spent as f64 / 1_000_000.0,
            tcs,
            s.title.as_deref().unwrap_or("")
        );
    }
}

fn short(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
