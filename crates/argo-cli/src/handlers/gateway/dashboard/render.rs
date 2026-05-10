//! Data fetching + HTML row rendering for the dashboard. Lifted
//! out of the parent `dashboard.rs` module to keep that file under
//! the 500-LOC ceiling. Pure rendering — every function returns an
//! HTML fragment string ready to slot into a parent template via
//! `<<rows>>` substitution.
use argo_store::SessionStore;
use chrono::{DateTime, Datelike, TimeZone, Utc};

pub(super) async fn worker_rows(store: &SessionStore) -> String {
    let day_start = day_start_micros(Utc::now());
    let rows: Vec<(String, i64, i64)> = match sqlx::query_as(
        "SELECT worker_id, COUNT(*) AS charges, COALESCE(SUM(usd_micros), 0) AS spent \
         FROM worker_budget WHERE charged_at_micros >= ? \
         GROUP BY worker_id ORDER BY spent DESC, worker_id LIMIT 100",
    )
    .bind(day_start)
    .fetch_all(store.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => return empty_row(5, &format!("worker_budget query failed: {e}")),
    };
    if rows.is_empty() {
        return empty_row(5, "no worker activity in the last 24h");
    }
    let last_actions = recent_action_per_worker(store).await;
    rows.into_iter()
        .map(|(worker_id, charges, micros)| {
            let last = last_actions
                .get(&worker_id)
                .cloned()
                .unwrap_or_else(|| "—".to_string());
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&worker_id),
                badge_for_status(charges),
                charges,
                esc(&format_usd_micros(u64::try_from(micros).unwrap_or(0))),
                esc(&last),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn recent_action_per_worker(
    store: &SessionStore,
) -> std::collections::HashMap<String, String> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT json_extract(payload_json, '$.worker_id'), kind, created_at \
         FROM audit_record \
         WHERE json_extract(payload_json, '$.worker_id') IS NOT NULL \
         ORDER BY seq DESC LIMIT 200",
    )
    .fetch_all(store.pool())
    .await
    .unwrap_or_default();
    let mut out = std::collections::HashMap::new();
    for (worker_id, kind, created_at) in rows {
        out.entry(worker_id).or_insert_with(|| {
            let when = DateTime::<Utc>::from_timestamp_micros(created_at)
                .map(|d| d.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "?".to_string());
            format!("{kind} @ {when}Z")
        });
    }
    out
}

pub(super) async fn session_rows(store: &SessionStore) -> String {
    let rows = match store.list_all_sessions(50).await {
        Ok(r) => r,
        Err(e) => return empty_row(6, &format!("list_all_sessions failed: {e}")),
    };
    if rows.is_empty() {
        return empty_row(6, "no sessions yet");
    }
    rows.into_iter()
        .map(|s| {
            let started = s.started_at.format("%Y-%m-%d %H:%M:%SZ").to_string();
            let status = if s.ended_at.is_some() { "ended" } else { "open" };
            let worker = s
                .worker_id
                .map(|w| w.0.to_string())
                .unwrap_or_else(|| "—".to_string());
            let title = s.title.clone().unwrap_or_default();
            format!(
                "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td>\
                 <td>{}/{}</td><td>{}</td><td>{}</td></tr>",
                esc(&started),
                esc(&short_id(&s.id.0.to_string())),
                esc(&short_id(&worker)),
                esc(&s.provider),
                esc(&s.model),
                esc(&title),
                badge_for_session_status(status),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) async fn approval_rows(store: &SessionStore) -> String {
    let pendings = match pending_approvals(store).await {
        Ok(p) => p,
        Err(e) => return empty_div(&format!("pending-approvals query failed: {e}")),
    };
    if pendings.is_empty() {
        return "<p class=\"muted\">No pending approvals.</p>".to_string();
    }
    pendings
        .into_iter()
        .map(|p| {
            format!(
                "<div class=\"approval-row\" data-approval-id=\"{aid}\">\
                   <div class=\"meta\">\
                     <strong>{atype}</strong> {risk}\
                     <div class=\"id\">action: {aid_short} · worker: {worker} · {at}</div>\
                   </div>\
                   <button class=\"btn btn-approve\" \
                     hx-post=\"/inbox/approve/{aid}\" \
                     hx-target=\"closest .approval-row\" hx-swap=\"outerHTML\">Approve</button>\
                   <button class=\"btn btn-deny\" \
                     hx-post=\"/inbox/reject/{aid}\" \
                     hx-target=\"closest .approval-row\" hx-swap=\"outerHTML\">Deny</button>\
                 </div>",
                aid = esc(&p.action_id),
                aid_short = esc(&short_id(&p.action_id)),
                atype = esc(&p.action_type),
                risk = badge_for_risk(&p.risk_level),
                worker = esc(&short_id(&p.worker_id)),
                at = esc(&p.created_at),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone)]
struct PendingApproval {
    action_id: String,
    worker_id: String,
    action_type: String,
    risk_level: String,
    created_at: String,
}

async fn pending_approvals(store: &SessionStore) -> Result<Vec<PendingApproval>, sqlx::Error> {
    let bodies: Vec<(String,)> = sqlx::query_as(
        "SELECT body FROM memory_entry \
         WHERE file = 'MEMORY.md' AND superseded_by IS NULL \
         ORDER BY learned_at DESC LIMIT 500",
    )
    .fetch_all(store.pool())
    .await?;
    let mut latest: std::collections::BTreeMap<String, PendingApproval> =
        std::collections::BTreeMap::new();
    for (body,) in bodies {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
            continue;
        };
        if v.get("schema_key").and_then(|x| x.as_str()) != Some("argo.action.v1") {
            continue;
        }
        let state = v.get("state").and_then(|x| x.as_str()).unwrap_or("");
        let requires = v
            .get("requires_approval")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        if state != "pending" || !requires {
            continue;
        }
        let action_id = v
            .get("action_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if action_id.is_empty() {
            continue;
        }
        let entry = PendingApproval {
            action_id: action_id.clone(),
            worker_id: v
                .get("worker_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            action_type: v
                .get("action_type")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            risk_level: v
                .get("risk_level")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: v
                .get("created_at")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        };
        latest.entry(action_id).or_insert(entry);
    }
    let mut out: Vec<_> = latest.into_values().collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out.truncate(20);
    Ok(out)
}

pub(super) async fn spend_chart(store: &SessionStore) -> (String, String) {
    let now = Utc::now();
    let cutoff = day_start_micros(now) - 30 * 86_400_000_000_i64;
    let rows: Vec<(i64, i64)> = match sqlx::query_as(
        "SELECT charged_at_micros, amount_usd_micros FROM spend_ledger \
         WHERE charged_at_micros >= ? ORDER BY charged_at_micros",
    )
    .bind(cutoff)
    .fetch_all(store.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                empty_div(&format!("spend_ledger query failed: {e}")),
                "—".into(),
            );
        }
    };
    let mut buckets: std::collections::BTreeMap<String, u64> =
        std::collections::BTreeMap::new();
    let mut grand_total: u64 = 0;
    for (charged_at, amount) in rows {
        let day = DateTime::<Utc>::from_timestamp_micros(charged_at)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        if day.is_empty() {
            continue;
        }
        let micros = u64::try_from(amount).unwrap_or(0);
        *buckets.entry(day).or_default() += micros;
        grand_total = grand_total.saturating_add(micros);
    }
    if buckets.is_empty() {
        return (empty_div("no spend in the last 30 days"), "$0.00".into());
    }
    let max = buckets.values().copied().max().unwrap_or(1).max(1);
    let html = buckets
        .into_iter()
        .rev()
        .map(|(day, micros)| {
            let pct = (micros as f64 / max as f64 * 100.0).round();
            format!(
                "<div class=\"spend-row\"><span class=\"muted\">{}</span>\
                 <div class=\"spend-bar\"><div class=\"spend-bar-fill\" \
                   style=\"width:{}%\"></div></div>\
                 <span>{}</span></div>",
                esc(&day),
                pct,
                esc(&format_usd_micros(micros)),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    (html, format_usd_micros(grand_total))
}

pub(super) async fn live_action_rows(store: &SessionStore) -> String {
    let filters = argo_audit::TailFilters::default();
    let rows = argo_audit::tail_rows_filtered(store, 15, &filters)
        .await
        .unwrap_or_default();
    if rows.is_empty() {
        return "<li class=\"muted\">no recent actions</li>".to_string();
    }
    rows.into_iter()
        .rev()
        .map(|r| {
            let when = DateTime::<Utc>::from_timestamp_micros(r.created_at_micros)
                .map(|d| d.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| "?".to_string());
            let worker = r
                .payload
                .get("worker_id")
                .and_then(|v| v.as_str())
                .unwrap_or("—");
            format!(
                "<li><span class=\"muted\">{when}Z</span> \
                 <span class=\"kind\">{kind}</span> \
                 <span class=\"muted\">{worker}</span></li>",
                when = esc(&when),
                kind = esc(&r.kind),
                worker = esc(&short_id(worker)),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn empty_row(cols: usize, msg: &str) -> String {
    format!(
        "<tr><td colspan=\"{cols}\" class=\"muted\">{}</td></tr>",
        esc(msg)
    )
}

pub(super) fn empty_div(msg: &str) -> String {
    format!("<p class=\"muted\">{}</p>", esc(msg))
}

pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn short_id(s: &str) -> String {
    if s.len() > 12 {
        format!("{}…", &s[..12])
    } else {
        s.to_string()
    }
}

pub(super) fn format_usd_micros(micros: u64) -> String {
    let dollars = micros as f64 / 1_000_000.0;
    if dollars >= 100.0 {
        format!("${dollars:.2}")
    } else if dollars >= 1.0 {
        format!("${dollars:.3}")
    } else {
        format!("${dollars:.4}")
    }
}

fn badge_for_status(charges: i64) -> String {
    if charges > 0 {
        "<span class=\"badge badge-ok\">active</span>".to_string()
    } else {
        "<span class=\"badge badge-warn\">idle</span>".to_string()
    }
}

fn badge_for_session_status(s: &str) -> String {
    let class = match s {
        "open" => "badge-ok",
        "ended" => "badge-warn",
        _ => "badge-warn",
    };
    format!("<span class=\"badge {class}\">{}</span>", esc(s))
}

pub(super) fn badge_for_risk(s: &str) -> String {
    let class = match s {
        "high" | "critical" => "badge-danger",
        "medium" => "badge-warn",
        _ => "badge-ok",
    };
    format!("<span class=\"badge {class}\">{}</span>", esc(s))
}

fn day_start_micros(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(
        now.year_ce().1.try_into().unwrap_or(1),
        now.month(),
        now.day(),
        0,
        0,
        0,
    )
    .single()
    .map(|d| d.timestamp_micros())
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_blocks_script_tags() {
        let out = esc("<script>alert('x')</script>");
        assert!(!out.contains("<script>"));
        assert!(out.contains("&lt;script&gt;"));
        assert!(out.contains("&#39;"));
    }

    #[test]
    fn format_usd_micros_buckets_by_magnitude() {
        assert_eq!(format_usd_micros(500), "$0.0005");
        assert_eq!(format_usd_micros(2_000_000), "$2.000");
        assert_eq!(format_usd_micros(150_000_000), "$150.00");
    }

    #[test]
    fn short_id_truncates_long_strings() {
        assert_eq!(short_id("abc"), "abc");
        assert_eq!(short_id("abcdefghijklmnop"), "abcdefghijkl…");
    }

    #[test]
    fn badge_for_risk_classifies_correctly() {
        assert!(badge_for_risk("high").contains("badge-danger"));
        assert!(badge_for_risk("critical").contains("badge-danger"));
        assert!(badge_for_risk("medium").contains("badge-warn"));
        assert!(badge_for_risk("low").contains("badge-ok"));
    }

    #[test]
    fn empty_row_renders_colspan() {
        let html = empty_row(4, "boom");
        assert!(html.contains("colspan=\"4\""));
        assert!(html.contains("boom"));
    }
}
