//! `argo worker items list` — aggregated per-`item_id` view over
//! the action ledger (#767). First-class operator surface for
//! the multi-day skate-selling proof: answers "what's in my
//! inventory, what's sold, what needs attention?" in one shot.
//!
//! Read-only — walks `worker_action` rows latest-wins per
//! `action_id`, groups by (vertical, item_id), and surfaces a
//! per-item rollup. Never mutates the ledger.
//!
//! `is_sold` semantics: an item is "sold" if there exists at
//! least one `mark_item_sold` action whose latest snapshot is in
//! `completed` state. Pending / approved / executing
//! mark_item_sold actions don't flip the bit — only completion
//! does. This mirrors how the operator thinks about it: until
//! the operator runs `complete <mark-sold-action-id>`, the item
//! is not yet sold.

use crate::handlers::vertical::resolve_verticals_dir;
use crate::handlers::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Per-item rollup. Serialised verbatim to JSON via serde.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ItemSummary {
    pub item_id: String,
    pub vertical: String,
    pub is_sold: bool,
    pub actions_total: usize,
    pub by_state: BTreeMap<String, usize>,
    pub latest_action_type: String,
    pub latest_state: String,
    pub last_updated_at: String,
}

const STATES: &[&str] = &[
    "pending",
    "approved",
    "executing",
    "completed",
    "failed",
    "blocked",
];

/// CLI entrypoint.
pub(crate) async fn list(
    vertical: Option<&str>,
    state_filter: &str,
    limit: usize,
    json_out: bool,
) -> ArgoResult<()> {
    let summaries = aggregate(vertical).await?;
    let mut rows: Vec<ItemSummary> = summaries
        .into_iter()
        .filter(|s| match state_filter {
            "all" => true,
            "sold" => s.is_sold,
            "active" => !s.is_sold,
            other => {
                eprintln!(
                    "warning: --state '{other}' not recognized for items; \
                     valid values are sold/active/all. Showing all."
                );
                true
            }
        })
        .collect();
    // Sort by last_updated_at descending — the operator wants
    // the most-recently-touched item up top.
    rows.sort_by(|a, b| {
        b.last_updated_at
            .cmp(&a.last_updated_at)
            .then_with(|| a.item_id.cmp(&b.item_id))
    });
    rows.truncate(limit);

    if json_out {
        for s in &rows {
            println!(
                "{}",
                serde_json::to_string(s).unwrap_or_else(|_| "{}".into())
            );
        }
    } else if rows.is_empty() {
        println!("(no items)");
    } else {
        print_text_table(&rows);
    }
    Ok(())
}

fn print_text_table(rows: &[ItemSummary]) {
    println!(
        "item_id                  vertical          sold  pend  apr  exec  done  fail  blk  latest_state  latest_action_type    last_updated_at"
    );
    for s in rows {
        let g = |k: &str| s.by_state.get(k).copied().unwrap_or(0);
        let item_id = trunc(&s.item_id, 24);
        let vertical = trunc(&s.vertical, 16);
        let sold = if s.is_sold { "yes" } else { "no" };
        let latest_state = trunc(&s.latest_state, 12);
        let latest_action = trunc(&s.latest_action_type, 20);
        println!(
            "{item_id:<24} {vertical:<17} {sold:<4} {p:>4}  {a:>3}  {e:>4}  {d:>4}  {f:>4}  {b:>3}  {ls:<12}  {la:<20}  {ts}",
            p = g("pending"),
            a = g("approved"),
            e = g("executing"),
            d = g("completed"),
            f = g("failed"),
            b = g("blocked"),
            ls = latest_state,
            la = latest_action,
            ts = s.last_updated_at,
        );
    }
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

/// Walk the memory store, collect (vertical, latest-per-action-id
/// ActionRecord) tuples, then aggregate to per-item summaries.
/// Pulled out of `list` so it's unit-testable: a fixture vec of
/// (vertical, ActionRecord) tuples plus the pure aggregation
/// step is enough to lock the rollup contract.
pub(crate) async fn aggregate(vertical: Option<&str>) -> ArgoResult<Vec<ItemSummary>> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let memory = MemoryStore::new(Arc::new(store));

    let vertical_ids: Vec<String> = match vertical {
        Some(v) => vec![v.to_string()],
        None => {
            let dir = resolve_verticals_dir();
            let (registry, _errors) = argo_vertical::load_dir(&dir)?;
            registry.list().map(|(id, _)| id.to_string()).collect()
        }
    };

    // Pass 1 — walk every vertical, collect every row, dedupe to
    // latest-per-action-id (the same as `collect_latest_actions`)
    // but tagged with the originating vertical.
    let mut latest: HashMap<String, (String, ActionRecord)> = HashMap::new();
    for vid in &vertical_ids {
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in entries {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            match latest.get(&rec.action_id) {
                Some((_v, prev)) if prev.created_at >= rec.created_at => {}
                _ => {
                    latest.insert(rec.action_id.clone(), (vid.clone(), rec));
                }
            }
        }
    }

    let tuples: Vec<(String, ActionRecord)> = latest.into_values().collect();
    Ok(rollup_items(tuples))
}

/// Pure aggregator. Takes the (vertical, latest-snapshot row)
/// tuples and produces a Vec of `ItemSummary` keyed on
/// `(vertical, item_id)`. Item-less rows (where `item_id` is
/// `None`) are dropped — they don't represent inventory.
pub(crate) fn rollup_items(tuples: Vec<(String, ActionRecord)>) -> Vec<ItemSummary> {
    let mut groups: HashMap<(String, String), Vec<ActionRecord>> = HashMap::new();
    for (vertical, rec) in tuples {
        let Some(item_id) = rec.item_id.clone() else {
            continue;
        };
        groups.entry((vertical, item_id)).or_default().push(rec);
    }

    let mut summaries: Vec<ItemSummary> = Vec::with_capacity(groups.len());
    for ((vertical, item_id), records) in groups {
        let mut by_state: BTreeMap<String, usize> = BTreeMap::new();
        for s in STATES {
            by_state.insert((*s).to_string(), 0);
        }
        let mut is_sold = false;
        let mut latest: Option<&ActionRecord> = None;
        for r in &records {
            *by_state.entry(r.state.clone()).or_insert(0) += 1;
            if r.action_type == "mark_item_sold" && r.state == "completed" {
                is_sold = true;
            }
            if latest.is_none_or(|l| r.created_at > l.created_at) {
                latest = Some(r);
            }
        }
        // Group is keyed by `(vertical, item_id)` and only
        // populated by `entry().or_default().push(rec)` above —
        // empty groups can't reach this point. Defensive-default
        // to the first record if `latest` ever ends up `None`
        // (would only happen on a future refactor that breaks the
        // invariant); avoids a panic in the live ledger walker.
        let Some(latest) = latest.or_else(|| records.first()) else {
            continue;
        };
        summaries.push(ItemSummary {
            item_id,
            vertical,
            is_sold,
            actions_total: records.len(),
            by_state,
            latest_action_type: latest.action_type.clone(),
            latest_state: latest.state.clone(),
            last_updated_at: latest.created_at.clone(),
        });
    }
    summaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(
        action_id: &str,
        item_id: Option<&str>,
        action_type: &str,
        state: &str,
        created_at: &str,
    ) -> ActionRecord {
        ActionRecord {
            schema_key: ActionRecord::SCHEMA_KEY.to_string(),
            action_id: action_id.to_string(),
            job_id: "job-1".to_string(),
            item_id: item_id.map(String::from),
            action_type: action_type.to_string(),
            risk_level: "low".to_string(),
            requires_approval: false,
            state: state.to_string(),
            payload: json!({}),
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

    #[test]
    fn rollup_groups_per_item_id_within_vertical() {
        let tuples = vec![
            ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
            ("gtm".to_string(), rec("a-2", Some("skates-A"), "post_listing", "pending", "2026-05-06T10:01:00Z")),
            ("gtm".to_string(), rec("a-3", Some("skates-B"), "verify_price", "completed", "2026-05-06T10:02:00Z")),
        ];
        let mut summaries = rollup_items(tuples);
        summaries.sort_by(|a, b| a.item_id.cmp(&b.item_id));
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].item_id, "skates-A");
        assert_eq!(summaries[0].actions_total, 2);
        assert_eq!(summaries[0].latest_action_type, "post_listing");
        assert_eq!(summaries[0].latest_state, "pending");
        assert_eq!(summaries[1].item_id, "skates-B");
        assert_eq!(summaries[1].actions_total, 1);
    }

    #[test]
    fn rollup_marks_sold_when_mark_item_sold_completed_exists() {
        let tuples = vec![
            ("gtm".to_string(), rec("a-1", Some("skates-A"), "post_listing", "completed", "2026-05-06T10:00:00Z")),
            ("gtm".to_string(), rec("a-2", Some("skates-A"), "mark_item_sold", "completed", "2026-05-06T10:01:00Z")),
        ];
        let summaries = rollup_items(tuples);
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].is_sold, "completed mark_item_sold must flip is_sold");
    }

    #[test]
    fn rollup_does_not_mark_sold_for_pending_mark_item_sold() {
        // Operator has approved nothing yet — item is still in
        // inventory. Locks the "only completion flips the bit"
        // rule from the module docs.
        let tuples = vec![
            ("gtm".to_string(), rec("a-1", Some("skates-A"), "mark_item_sold", "pending", "2026-05-06T10:00:00Z")),
            ("gtm".to_string(), rec("a-2", Some("skates-A"), "mark_item_sold", "approved", "2026-05-06T10:01:00Z")),
        ];
        let summaries = rollup_items(tuples);
        assert_eq!(summaries.len(), 1);
        assert!(!summaries[0].is_sold, "pre-completion mark_item_sold rows must not flip is_sold");
    }

    #[test]
    fn rollup_skips_item_less_rows() {
        // Item-less actions (e.g. system maintenance) are not
        // inventory; they shouldn't appear in the items view.
        let tuples = vec![
            ("gtm".to_string(), rec("a-1", None, "verify_price", "completed", "2026-05-06T10:00:00Z")),
            ("gtm".to_string(), rec("a-2", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:01:00Z")),
        ];
        let summaries = rollup_items(tuples);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].item_id, "skates-A");
    }

    #[test]
    fn rollup_by_state_counts_match_per_action_states() {
        // 3 actions for skates-A: 1 completed, 1 pending, 1
        // blocked. Counts should reflect that.
        let tuples = vec![
            ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
            ("gtm".to_string(), rec("a-2", Some("skates-A"), "post_listing", "pending", "2026-05-06T10:01:00Z")),
            ("gtm".to_string(), rec("a-3", Some("skates-A"), "boost_listing", "blocked", "2026-05-06T10:02:00Z")),
        ];
        let summaries = rollup_items(tuples);
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        assert_eq!(s.by_state.get("completed"), Some(&1));
        assert_eq!(s.by_state.get("pending"), Some(&1));
        assert_eq!(s.by_state.get("blocked"), Some(&1));
        assert_eq!(s.by_state.get("approved"), Some(&0));
    }

    #[test]
    fn rollup_separates_same_item_id_under_different_verticals() {
        // skates-A in two different vertical namespaces is two
        // separate items. Locks the cross-vertical isolation
        // contract for the items view.
        let tuples = vec![
            ("gtm".to_string(), rec("a-1", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:00:00Z")),
            ("other".to_string(), rec("a-2", Some("skates-A"), "verify_price", "completed", "2026-05-06T10:01:00Z")),
        ];
        let summaries = rollup_items(tuples);
        assert_eq!(summaries.len(), 2);
    }
}
