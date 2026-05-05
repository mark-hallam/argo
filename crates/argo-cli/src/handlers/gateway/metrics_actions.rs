//! Worker action-ledger metrics (#757). Sibling of
//! `metrics.rs` so it stays under the per-file 500-LOC budget.
//!
//! Exposes Prometheus + JSON gauges for the action ledger:
//!
//! * `argo_worker_actions_total{state="pending"|...|"blocked"}` —
//!   total rows per state, deduped to the latest snapshot per
//!   `action_id`. Mirrors what `argo worker actions list` shows
//!   so dashboards align with operator views.
//! * `argo_worker_actions_pending_approval_total` — pending
//!   rows where `requires_approval=true`. The operator's
//!   review queue depth.
//! * `argo_worker_actions_pending_autonomous_total` — pending
//!   rows where `requires_approval=false`. Self-clearing (the
//!   worker advances these without the operator).
//!
//! Read-only — never mutates anything. Safe to scrape every
//! 15s. Walks memory entries via `MemoryStore::list_active`,
//! parses `worker_action` rows, dedupes latest-wins per
//! action_id (same logic as `worker_doctor`).

use crate::handlers::worker_actions::ActionRecord;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;

/// Counts surfaced into both the Prometheus + JSON variants.
/// Public so `metrics.rs` can render either format off the
/// same shape.
#[derive(Debug, Default, Clone)]
pub(super) struct ActionLedgerCounts {
    pub by_state: HashMap<&'static str, u64>,
    pub pending_approval: u64,
    pub pending_autonomous: u64,
    pub total: u64,
}

const STATES: &[&str] = &[
    "pending",
    "approved",
    "executing",
    "completed",
    "failed",
    "blocked",
];

pub(super) async fn gather(store: &Arc<SessionStore>) -> ActionLedgerCounts {
    let mut counts = ActionLedgerCounts::default();
    for s in STATES {
        counts.by_state.insert(*s, 0);
    }
    let memory = MemoryStore::new(Arc::clone(store));
    // Walk every workspace × every memory row and reduce to
    // latest-wins per action_id. The action ledger is small
    // for v1.0 deployments (single-digit thousands at most),
    // so an in-process walk is fine.
    let mut latest: HashMap<String, ActionRecord> = HashMap::new();
    let workspaces: Vec<argo_types::WorkspaceId> = match list_workspaces(store).await {
        Ok(v) => v,
        Err(_) => return counts, // partial DB → empty counts, never blow up the scrape
    };
    for ws in workspaces {
        let entries = match memory.list_active(ws, MemoryFile::Memory).await {
            Ok(v) => v,
            Err(_) => continue,
        };
        for e in entries {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
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
    }
    for rec in latest.values() {
        counts.total += 1;
        if let Some(slot) = counts.by_state.get_mut(rec.state.as_str()) {
            *slot += 1;
        }
        if rec.state == "pending" && rec.requires_approval {
            counts.pending_approval += 1;
        }
        if rec.state == "pending" && !rec.requires_approval {
            counts.pending_autonomous += 1;
        }
    }
    counts
}

async fn list_workspaces(
    store: &Arc<SessionStore>,
) -> Result<Vec<argo_types::WorkspaceId>, sqlx::Error> {
    let rows: Vec<(Vec<u8>,)> = sqlx::query_as("SELECT id FROM workspace")
        .fetch_all(store.pool())
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(b,)| uuid::Uuid::from_slice(&b).ok().map(argo_types::WorkspaceId))
        .collect())
}

pub(super) fn render_text(out: &mut String, counts: &ActionLedgerCounts) {
    let _ = writeln!(
        out,
        "\n# HELP argo_worker_actions_total Worker action ledger rows by state, latest-wins per action_id\n\
         # TYPE argo_worker_actions_total gauge"
    );
    for state in STATES {
        let n = counts.by_state.get(state).copied().unwrap_or(0);
        let _ = writeln!(out, "argo_worker_actions_total{{state=\"{state}\"}} {n}");
    }
    let _ = writeln!(
        out,
        "\n# HELP argo_worker_actions_pending_approval_total Pending rows that require operator approval\n\
         # TYPE argo_worker_actions_pending_approval_total gauge\n\
         argo_worker_actions_pending_approval_total {}",
        counts.pending_approval
    );
    let _ = writeln!(
        out,
        "\n# HELP argo_worker_actions_pending_autonomous_total Pending rows that the worker may advance autonomously\n\
         # TYPE argo_worker_actions_pending_autonomous_total gauge\n\
         argo_worker_actions_pending_autonomous_total {}",
        counts.pending_autonomous
    );
}

pub(super) fn render_json(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    counts: &ActionLedgerCounts,
) {
    let mut by_state = serde_json::Map::new();
    for state in STATES {
        let n = counts.by_state.get(state).copied().unwrap_or(0);
        by_state.insert((*state).to_string(), serde_json::json!(n));
    }
    payload.insert(
        "argo_worker_actions_total".into(),
        serde_json::Value::Object(by_state),
    );
    payload.insert(
        "argo_worker_actions_pending_approval_total".into(),
        serde_json::json!(counts.pending_approval),
    );
    payload.insert(
        "argo_worker_actions_pending_autonomous_total".into(),
        serde_json::json!(counts.pending_autonomous),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_memory::MemoryFile;

    fn rec(
        action_id: &str,
        state: &str,
        requires_approval: bool,
        created_at: &str,
    ) -> ActionRecord {
        ActionRecord {
            schema_key: ActionRecord::SCHEMA_KEY.to_string(),
            action_id: action_id.to_string(),
            job_id: "job-1".to_string(),
            item_id: None,
            action_type: "verify_price".to_string(),
            risk_level: "low".to_string(),
            requires_approval,
            state: state.to_string(),
            payload: serde_json::json!({}),
            approved_by: None,
            approved_at: None,
            executed_at: None,
            platform: None,
            result: None,
            evidence: vec![],
            blocked_reason: None,
            created_at: created_at.to_string(),
        }
    }

    async fn open_test_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn gather_dedupes_to_latest_state_per_action_id() {
        let store = open_test_store().await;
        let memory = MemoryStore::new(Arc::clone(&store));
        let (_t, ws) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();

        // Two rows for one action: pending (older) → executing (newer).
        let r1 = rec("a-1", "pending", false, "2026-05-05T10:00:00Z");
        let r2 = rec("a-1", "executing", false, "2026-05-05T10:00:01Z");
        for r in [&r1, &r2] {
            memory
                .append_for_vertical(
                    ws,
                    MemoryFile::Memory,
                    serde_json::to_string(r).unwrap(),
                    None,
                    "demo",
                )
                .await
                .unwrap();
        }

        let counts = gather(&store).await;
        assert_eq!(counts.total, 1);
        assert_eq!(counts.by_state.get("executing"), Some(&1));
        assert_eq!(counts.by_state.get("pending"), Some(&0));
    }

    #[tokio::test]
    async fn gather_splits_pending_by_approval_class() {
        let store = open_test_store().await;
        let memory = MemoryStore::new(Arc::clone(&store));
        let (_t, ws) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
        for (id, ra) in [("a-1", true), ("a-2", false), ("a-3", true), ("a-4", true)] {
            let r = rec(id, "pending", ra, "2026-05-05T10:00:00Z");
            memory
                .append_for_vertical(
                    ws,
                    MemoryFile::Memory,
                    serde_json::to_string(&r).unwrap(),
                    None,
                    "demo",
                )
                .await
                .unwrap();
        }
        let counts = gather(&store).await;
        assert_eq!(counts.pending_approval, 3);
        assert_eq!(counts.pending_autonomous, 1);
        assert_eq!(counts.total, 4);
    }

    #[test]
    fn render_text_emits_one_line_per_state_plus_pending_breakdowns() {
        let mut counts = ActionLedgerCounts::default();
        for s in STATES {
            counts.by_state.insert(*s, 0);
        }
        counts.by_state.insert("pending", 4);
        counts.by_state.insert("completed", 12);
        counts.pending_approval = 3;
        counts.pending_autonomous = 1;
        counts.total = 16;
        let mut out = String::new();
        render_text(&mut out, &counts);
        assert!(out.contains("argo_worker_actions_total{state=\"pending\"} 4"));
        assert!(out.contains("argo_worker_actions_total{state=\"completed\"} 12"));
        assert!(out.contains("argo_worker_actions_pending_approval_total 3"));
        assert!(out.contains("argo_worker_actions_pending_autonomous_total 1"));
        // # HELP + # TYPE lines present.
        assert!(out.contains("# TYPE argo_worker_actions_total gauge"));
    }
}
