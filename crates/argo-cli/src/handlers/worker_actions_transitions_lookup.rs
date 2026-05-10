//! Read-side discovery helpers for `worker_actions_transitions`.
//!
//! Locating an action by `action_id` requires walking every loaded
//! vertical's memory namespace because the row is stored under the
//! vertical that emitted it. These helpers stay separate from the
//! transition writers so the write-side file can keep its focus on
//! state-machine logic.
//!
//! Both helpers share the same scan pattern:
//! 1. Resolve the verticals dir + load the registry.
//! 2. For each loaded vertical, list its `MemoryFile::Memory` rows.
//! 3. Decode each row body as `ActionRecord`, skip mismatches, and
//!    filter on `action_id`.
//!
//! `find_action` returns the latest snapshot (used by the write-
//! side as the "current state"). `full_history` returns every row in
//! time order (used by `status` to render the transition history).

use super::worker_actions::ActionRecord;
use crate::handlers::vertical::resolve_verticals_dir;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::ArgoResult;
use argo_types::WorkspaceId;

/// Lookup result: the latest snapshot of an action plus the
/// vertical_id its memory rows live under. Knowing the
/// vertical is required to write the next state-row back into
/// the correct namespace.
#[derive(Clone)]
pub(crate) struct LocatedAction {
    pub record: ActionRecord,
    pub vertical_id: String,
}

/// Walk every loaded vertical's memory namespace looking for
/// rows whose body's `action_id` matches. Returns the latest-
/// wins snapshot + the vertical_id that owns it.
pub(crate) async fn find_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<Option<LocatedAction>> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let mut latest: Option<LocatedAction> = None;
    for (vid, _loaded) in registry.list() {
        let rows = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in rows {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            if rec.action_id != action_id {
                continue;
            }
            let take = match &latest {
                None => true,
                Some(prev) => rec.created_at > prev.record.created_at,
            };
            if take {
                latest = Some(LocatedAction {
                    record: rec,
                    vertical_id: vid.to_string(),
                });
            }
        }
    }
    Ok(latest)
}

/// Walk every loaded vertical and return every persisted state-row
/// for `action_id` in time order (oldest first). Used by `status`
/// to render the full transition history. Empty when the
/// action_id is unknown.
pub(crate) async fn full_history(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<Vec<ActionRecord>> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let mut out = Vec::new();
    for (vid, _loaded) in registry.list() {
        let rows = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in rows {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            if rec.action_id != action_id {
                continue;
            }
            out.push(rec);
        }
    }
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(out)
}
