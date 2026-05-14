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
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use std::collections::{BTreeMap, HashMap, HashSet};
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

/// `argo worker items show <item_id>` — full state-row history
/// for one item across every action it owns. Walks every
/// `worker_action` memory row whose `item_id` matches and sorts
/// chronologically. Unlike `list`, this does NOT dedupe to
/// latest-per-action-id — the operator wants the complete
/// transition timeline (who approved when, executed when,
/// completed/failed when).
pub(crate) async fn show(
    item_id: &str,
    vertical: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    let rows = collect_all_rows_for_item(item_id, vertical).await?;
    if rows.is_empty() {
        if json_out {
            // Empty array — caller can `length == 0` test it.
            println!("[]");
        } else {
            println!("(no actions for item_id `{item_id}`)");
        }
        return Ok(());
    }
    if json_out {
        for (_v, r) in &rows {
            println!(
                "{}",
                serde_json::to_string(r).unwrap_or_else(|_| "{}".into())
            );
        }
    } else {
        print_show_timeline(item_id, &rows);
    }
    Ok(())
}

/// Walk every memory row for the given `item_id` (no
/// latest-per-action-id dedup), tagged with the originating
/// vertical. Returns rows sorted chronologically by
/// `created_at`. Pulled out of `show()` so the timeline shape is
/// unit-testable independent of the live store.
async fn collect_all_rows_for_item(
    item_id: &str,
    vertical: Option<&str>,
) -> ArgoResult<Vec<(String, ActionRecord)>> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
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

    let mut rows: Vec<(String, ActionRecord)> = Vec::new();
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
            if rec.item_id.as_deref() != Some(item_id) {
                continue;
            }
            rows.push((vid.clone(), rec));
        }
    }
    rows.sort_by(|a, b| {
        a.1.created_at
            .cmp(&b.1.created_at)
            .then_with(|| a.1.action_id.cmp(&b.1.action_id))
    });
    Ok(rows)
}

fn print_show_timeline(item_id: &str, rows: &[(String, ActionRecord)]) {
    // Header: which item, which vertical(s), how many rows.
    let verticals: std::collections::BTreeSet<&str> =
        rows.iter().map(|(v, _)| v.as_str()).collect();
    let vertical_label = verticals.iter().copied().collect::<Vec<_>>().join(", ");
    let unique_actions: std::collections::BTreeSet<&str> =
        rows.iter().map(|(_, r)| r.action_id.as_str()).collect();
    println!(
        "item: {item_id}   vertical: {vertical_label}   rows: {}   actions: {}",
        rows.len(),
        unique_actions.len()
    );
    println!("{:<27} {:<20} {:<10} {:<14} action_id", "created_at", "action_type", "state", "platform");
    for (_v, r) in rows {
        let platform = r.platform.as_deref().unwrap_or("-");
        println!(
            "{:<27} {:<20} {:<10} {:<14} {}",
            r.created_at,
            trunc(&r.action_type, 20),
            trunc(&r.state, 10),
            trunc(platform, 14),
            r.action_id,
        );
    }
}

/// Walk the memory store, collect (vertical, latest-per-action-id
/// ActionRecord) tuples, then aggregate to per-item summaries.
/// Pulled out of `list` so it's unit-testable: a fixture vec of
/// (vertical, ActionRecord) tuples plus the pure aggregation
/// step is enough to lock the rollup contract.
pub(crate) async fn aggregate(vertical: Option<&str>) -> ArgoResult<Vec<ItemSummary>> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
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

/// Compute the set of item_ids that are "sold" within the given
/// vertical's namespace, where "sold" means: at least one
/// `mark_item_sold` action has reached `completed` state in the
/// memory store. Used by the emission gate (#769) to refuse new
/// emissions for inventory the operator has already closed.
///
/// Walks the memory store directly for efficiency — does NOT
/// need the full per-item rollup. Caller already has the
/// (memory, workspace, vertical_id) trio (typically from the
/// worker's pre-emission glue in `work_emission.rs`), so this
/// avoids re-opening the store from cwd.
pub(crate) async fn compute_sold_items(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
) -> ArgoResult<HashSet<String>> {
    let entries = memory
        .list_for_vertical(workspace, MemoryFile::Memory, vertical_id)
        .await?;
    let mut sold: HashSet<String> = HashSet::new();
    for e in entries {
        let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
            continue;
        };
        if rec.schema_key != ActionRecord::SCHEMA_KEY {
            continue;
        }
        if rec.action_type != "mark_item_sold" || rec.state != "completed" {
            continue;
        }
        if let Some(item_id) = rec.item_id {
            sold.insert(item_id);
        }
    }
    Ok(sold)
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
#[path = "worker_actions_items_tests.rs"]
mod tests;
