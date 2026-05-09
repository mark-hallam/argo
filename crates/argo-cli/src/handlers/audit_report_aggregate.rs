//! Row aggregation for `argo audit report`: counts, breakdowns, queue snapshots.
use super::worker_actions::ActionRecord;
use argo_audit::ExportedRow;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::BTreeMap;

#[derive(Debug, Default, serde::Serialize)]
pub(super) struct SummaryStats {
    pub(super) actions_executed: usize,
    pub(super) approvals_granted: usize,
    pub(super) escalations_triggered: usize,
    pub(super) anomalies_detected: usize,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub(super) struct ChannelBreakdown {
    pub(super) channel: String,
    pub(super) granted: usize,
    pub(super) revoked: usize,
    pub(super) consumed: usize,
    pub(super) total: usize,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct SpenderRow {
    pub(super) worker_id: String,
    pub(super) charges: i64,
    pub(super) usd_micros: i64,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct PendingApproval {
    pub(super) action_id: String,
    pub(super) worker_id: String,
    pub(super) action_type: String,
    pub(super) risk_level: String,
    pub(super) created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub(super) struct ChainVerdict {
    pub(super) ok: bool,
    pub(super) rows: usize,
    pub(super) breaks: usize,
}

pub(super) fn summarize_rows(rows: &[ExportedRow]) -> SummaryStats {
    let mut out = SummaryStats::default();
    for row in rows {
        match row.kind.as_str() {
            "tool_call" => out.actions_executed += 1,
            "approval.granted" => out.approvals_granted += 1,
            "escalation.required" => out.escalations_triggered += 1,
            _ => {}
        }
        if row.kind.contains("anomaly")
            || row
                .payload
                .get("kind")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.contains("anomaly"))
        {
            out.anomalies_detected += 1;
        }
    }
    out
}

pub(super) fn channel_breakdown(rows: &[ExportedRow]) -> Vec<ChannelBreakdown> {
    let mut by_channel: BTreeMap<String, ChannelBreakdown> = BTreeMap::new();
    for row in rows.iter().filter(|r| r.kind.starts_with("approval.")) {
        let channel = approval_channel(&row.payload);
        let entry = by_channel
            .entry(channel.clone())
            .or_insert_with(|| ChannelBreakdown {
                channel,
                ..ChannelBreakdown::default()
            });
        match row.kind.as_str() {
            "approval.granted" => entry.granted += 1,
            "approval.revoked" => entry.revoked += 1,
            "approval.consumed" => entry.consumed += 1,
            _ => {}
        }
        entry.total += 1;
    }
    by_channel.into_values().collect()
}

pub(super) async fn top_spenders(store: &SessionStore) -> ArgoResult<Vec<SpenderRow>> {
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT worker_id, COUNT(*) AS charges, COALESCE(SUM(usd_micros), 0) AS total \
         FROM worker_budget GROUP BY worker_id ORDER BY total DESC, worker_id LIMIT 10",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit report spenders: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(worker_id, charges, usd_micros)| SpenderRow {
            worker_id,
            charges,
            usd_micros,
        })
        .collect())
}

pub(super) async fn pending_approvals(store: &SessionStore) -> ArgoResult<Vec<PendingApproval>> {
    let bodies: Vec<(String,)> = sqlx::query_as(
        "SELECT body FROM memory_entry \
         WHERE file = 'MEMORY.md' AND superseded_by IS NULL ORDER BY learned_at DESC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit report pending queue: {e}")))?;
    let mut latest: BTreeMap<String, ActionRecord> = BTreeMap::new();
    for (body,) in bodies {
        let Ok(rec) = serde_json::from_str::<ActionRecord>(&body) else {
            continue;
        };
        if rec.schema_key != ActionRecord::SCHEMA_KEY {
            continue;
        }
        match latest.get(&rec.action_id) {
            Some(prev) if prev.created_at >= rec.created_at => {}
            _ => {
                latest.insert(rec.action_id.clone(), rec);
            }
        }
    }
    let mut rows: Vec<_> = latest
        .into_values()
        .filter(|rec| rec.state == "pending" && rec.requires_approval)
        .map(|rec| PendingApproval {
            action_id: rec.action_id,
            worker_id: rec.worker_id.to_string(),
            action_type: rec.action_type,
            risk_level: rec.risk_level,
            created_at: rec.created_at,
        })
        .collect();
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    rows.truncate(10);
    Ok(rows)
}

fn approval_channel(payload: &serde_json::Value) -> String {
    for ptr in ["/notify_channel", "/channel", "/source", "/resolved_by"] {
        if let Some(value) = payload.pointer(ptr).and_then(|v| v.as_str()) {
            return value.split(':').next().unwrap_or(value).to_string();
        }
    }
    "(unknown)".into()
}
