//! Vertical-declared follow-up emission on action completion
//! (#771).
//!
//! When the operator runs `argo worker actions complete <id>`,
//! the OS persists the new `completed` row, then walks the bound
//! vertical's manifest for that action's `on_complete_emit` list.
//! Each declared action_type is emitted as a fresh
//! `worker_action` row with:
//!
//! - `item_id` and `platform` inherited from the source
//! - `evidence` chain referencing the source's
//!   `argo://action/<action_id>`
//! - empty `payload` (the operator / model can refine before
//!   approval if needed)
//!
//! The emission goes through the SAME safety gates the model
//! emission path uses:
//!
//! - manifest classification (undeclared types land `blocked`)
//! - sold-item gate (#769) — refuses follow-ups for already-sold
//!   inventory
//! - the OS-level `requires_approval` stamp from the manifest
//!
//! The evidence gate (#729) does NOT apply: a manifest-declared
//! follow-up has the operator's blessing of the source action;
//! it isn't a model emission on top of weak research.
//!
//! v1 emits synchronously on completion. Time-delayed
//! follow-ups ("emit boost_listing 72h after post_listing
//! complete") are a future extension that will use the same
//! emission path with a `scheduled_at` field on the emitted row.

use super::worker_actions::ActionRecord;
use super::worker_actions_emit::{build_emission, ActionEmissionInputs};
use super::worker_actions_items;
use super::worker_actions_transitions::LocatedAction;
use crate::handlers::vertical::resolve_verticals_dir;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::WorkspaceId;
use std::collections::HashSet;

/// Emit every `on_complete_emit` follow-up declared by the
/// source action's manifest entry. Logs warnings on failure
/// but never propagates errors — a failed follow-up should not
/// undo the operator's `complete` transition.
pub(crate) async fn emit_on_complete_followups(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    source: &LocatedAction,
) {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = match argo_vertical::load_dir(&dir) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "argo worker: warning — vertical reload for follow-up emission failed: {e}"
            );
            return;
        }
    };
    let Some(loaded) = registry.get(&source.vertical_id) else {
        return;
    };
    let Some(actions_section) = loaded.manifest.actions.as_ref() else {
        return;
    };
    let Some(decl) = actions_section.actions.get(&source.record.action_type) else {
        return;
    };
    if decl.on_complete_emit.is_empty() {
        return;
    }

    // Snapshot the sold-set so each follow-up sees the current
    // inventory state. mark_item_sold completing → its own
    // follow-ups must see "this item is now sold."
    let sold_items: HashSet<String> = match worker_actions_items::compute_sold_items(
        memory,
        workspace,
        &source.vertical_id,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "argo worker: warning — sold-items snapshot failed during follow-up: {e}"
            );
            HashSet::new()
        }
    };

    for follow_up_type in &decl.on_complete_emit {
        let mut rec = build_followup_record(
            &loaded.manifest,
            source,
            follow_up_type,
        );
        apply_sold_gate(&mut rec, &sold_items);
        if let Err(e) = persist(memory, workspace, &source.vertical_id, &rec).await {
            eprintln!(
                "argo worker: warning — could not persist follow-up `{}` for source `{}`: {e}",
                follow_up_type, source.record.action_id
            );
        }
    }
}

fn build_followup_record(
    manifest: &argo_vertical::VerticalManifest,
    source: &LocatedAction,
    follow_up_type: &str,
) -> ActionRecord {
    let inputs = ActionEmissionInputs {
        vertical: manifest,
        job_id: &source.record.job_id,
        action_type: follow_up_type,
        payload: serde_json::json!({}),
        item_id: source.record.item_id.as_deref(),
        platform: source.record.platform.as_deref(),
        evidence: vec![format!("argo://action/{}", source.record.action_id)],
    };
    build_emission(&inputs)
}

/// Apply the sold-item gate to a freshly-built follow-up. Same
/// shape as the gate in `worker_actions_emit::emit_actions_from_text`
/// but local to this path: a follow-up is a manifest-declared
/// emission, not a model emission, so the evidence gate doesn't
/// fire. The sold-item gate still does — closed inventory stays
/// closed.
fn apply_sold_gate(rec: &mut ActionRecord, sold_items: &HashSet<String>) {
    let item_is_sold = rec
        .item_id
        .as_deref()
        .is_some_and(|id| sold_items.contains(id));
    if item_is_sold && rec.action_type != "mark_item_sold" && rec.state != "blocked" {
        rec.state = "blocked".to_string();
        rec.blocked_reason = Some(format!(
            "item `{}` already marked sold — follow-up `{}` refused. \
             Inventory is closed; the worker should not act on sold items.",
            rec.item_id.as_deref().unwrap_or("?"),
            rec.action_type,
        ));
    }
}

async fn persist(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_id: &str,
    rec: &ActionRecord,
) -> argo_types::ArgoResult<()> {
    let body = serde_json::to_string(rec)
        .map_err(|e| argo_types::ArgoError::Provider(format!("serialize follow-up: {e}")))?;
    memory
        .append_for_vertical(workspace, MemoryFile::Memory, body, None, vertical_id)
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "worker_actions_followup_tests.rs"]
mod tests;
