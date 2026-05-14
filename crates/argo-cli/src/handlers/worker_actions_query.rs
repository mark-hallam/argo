//! Query-side worker_action helpers (Stream
//! `claude-worker-actions-split`). Lifted out of
//! `worker_actions.rs` so that file stays under the per-file
//! 500 LOC budget.
//!
//! Owns the read-side surface — `list`, `show`,
//! `collect_latest_actions` — plus a couple of small helpers
//! (`short`, `lookup_max_attempts`). No behaviour change vs.
//! the historical inline implementations in
//! `worker_actions::{list, show, ...}`; the dispatcher in
//! `worker_actions::run` now calls into this module instead.
use super::vertical::resolve_verticals_dir;
use super::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn list(
    state_filter: &str,
    job_id: Option<&str>,
    vertical: Option<&str>,
    item_id_filter: Option<&str>,
    action_type_filter: Option<&str>,
    platform_filter: Option<&str>,
    limit: usize,
    json_out: bool,
) -> ArgoResult<()> {
    let actions = collect_latest_actions(vertical).await?;
    let mut rows = super::worker_actions_list::apply_list_filters(
        actions,
        state_filter,
        job_id,
        item_id_filter,
        action_type_filter,
        platform_filter,
    );
    // Newest-first by created_at; ties broken by action_id for determinism.
    rows.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    rows.truncate(limit);

    if json_out {
        for a in &rows {
            println!(
                "{}",
                serde_json::to_string(&a).unwrap_or_else(|_| "{}".into())
            );
        }
    } else if rows.is_empty() {
        println!("(no actions)");
    } else {
        println!("action_id                  state        type                   platform       created_at");
        for a in &rows {
            println!(
                "{:<26} {:<12} {:<22} {:<14} {}",
                short(&a.action_id),
                a.state,
                a.action_type,
                a.platform.as_deref().unwrap_or("-"),
                a.created_at,
            );
        }
    }
    Ok(())
}

pub(crate) async fn show(action_id: &str, json_out: bool) -> ArgoResult<()> {
    let actions = collect_latest_actions(None).await?;
    let Some(found) = actions.into_iter().find(|a| a.action_id == action_id) else {
        return Err(ArgoError::Provider(format!(
            "no action with action_id '{action_id}'"
        )));
    };
    let attempt_number = super::worker_actions_dispatch::count_retry_chain(&found);
    let max_attempts = lookup_max_attempts(&found.action_type).await.unwrap_or(1);
    if json_out {
        let mut v = serde_json::to_value(&found).unwrap_or(serde_json::json!({}));
        if let Some(map) = v.as_object_mut() {
            map.insert("retry_attempt".into(), serde_json::json!(attempt_number));
            map.insert("retry_max_attempts".into(), serde_json::json!(max_attempts));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("action_id     {}", found.action_id);
        println!("job_id        {}", found.job_id);
        if let Some(it) = &found.item_id {
            println!("item_id       {it}");
        }
        println!("action_type   {}", found.action_type);
        println!("state         {}", found.state);
        if let Some(p) = &found.platform {
            println!("platform      {p}");
        }
        println!("created_at    {}", found.created_at);
        println!("attempt       {attempt_number}/{max_attempts}");
        if let Some(by) = &found.approved_by {
            println!("approved_by   {by}");
        }
        if let Some(at) = &found.approved_at {
            println!("approved_at   {at}");
        }
        if let Some(at) = &found.executed_at {
            println!("executed_at   {at}");
        }
        if let Some(reason) = &found.blocked_reason {
            println!("blocked       {reason}");
        }
        if !found.evidence.is_empty() {
            println!("evidence:");
            for e in &found.evidence {
                println!("  - {e}");
            }
        }
        println!("payload:");
        println!(
            "{}",
            serde_json::to_string_pretty(&found.payload).unwrap_or_else(|_| "{}".into())
        );
        if let Some(r) = &found.result {
            println!("result:");
            println!(
                "{}",
                serde_json::to_string_pretty(r).unwrap_or_else(|_| "{}".into())
            );
        }
    }
    Ok(())
}

/// Walk every loaded vertical's memory namespace, collect every
/// `worker_action` row, and reduce to "latest snapshot per
/// `action_id`".
pub(crate) async fn collect_latest_actions(
    vertical: Option<&str>,
) -> ArgoResult<Vec<ActionRecord>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
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
    let store_arc = Arc::new(store);
    let memory = MemoryStore::new(store_arc.clone());

    let vertical_ids: Vec<String> = match vertical {
        Some(v) => vec![v.to_string()],
        None => {
            let dir = resolve_verticals_dir();
            let (registry, _errors) = argo_vertical::load_dir(&dir)?;
            registry.list().map(|(id, _)| id.to_string()).collect()
        }
    };

    let mut latest: HashMap<String, ActionRecord> = HashMap::new();
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
                Some(prev) if prev.created_at >= rec.created_at => {}
                _ => {
                    latest.insert(rec.action_id.clone(), rec);
                }
            }
        }
    }
    Ok(latest.into_values().collect())
}

pub(crate) fn short(id: &str) -> String {
    if id.len() > 24 {
        format!("{}…", &id[..23])
    } else {
        id.to_string()
    }
}

/// Lookup `[actions.<type>.retry] max_attempts` across every
/// loaded vertical. Returns the first match.
async fn lookup_max_attempts(action_type: &str) -> Option<u32> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errs) = argo_vertical::load_dir(&dir).ok()?;
    for (_id, loaded) in registry.list() {
        if let Some(decl) = loaded
            .manifest
            .actions
            .as_ref()
            .and_then(|a| a.actions.get(action_type))
        {
            return Some(decl.retry.clone().unwrap_or_default().max_attempts);
        }
    }
    None
}
