//! Retry handler for the worker-actions dispatch surface.
//! Owns `argo worker actions retry <id>` — emits a fresh
//! pending action with the original payload + a retry-chain
//! evidence link, respecting manifest `retry.max_attempts`
//! unless `--force`. Split out of `worker_actions_dispatch.rs`
//! (claude-worker-actions-dispatch-split) to keep that file
//! under the 500-LOC HARD RULE 6 ceiling. Pure refactor; no
//! behaviour change.

use super::worker_actions::ActionRecord;
use super::worker_actions_dispatch_ledger::auto_refresh_ledger;
use super::worker_actions_emit;
use super::worker_actions_transitions;
use argo_types::{ArgoError, ArgoResult};

/// Retry a failed action by emitting a fresh pending action with
/// the same payload, platform, item_id, and retry-chain evidence.
/// Respects manifest `retry.max_attempts` unless `--force` is set.
pub(crate) async fn retry(action_id: &str, force: bool, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let located = worker_actions_transitions::find_action(&memory, ws, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    if located.record.state != "failed" {
        return Err(ArgoError::Provider(format!(
            "action {action_id} is in state '{}', not 'failed'; \
             retry only applies to terminal failures",
            located.record.state
        )));
    }
    // Reload the manifest so build_emission can classify the
    // retried action against the current declarations.
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(&located.vertical_id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' not loaded; cannot retry",
            located.vertical_id
        ))
    })?;

    // Count existing retry-chain evidence, including the source
    // action as attempt #1.
    let prior_attempts = count_retry_chain(&located.record);
    let next_attempt_number = prior_attempts + 1;
    let policy = loaded
        .manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get(&located.record.action_type))
        .and_then(|d| d.retry.clone())
        .unwrap_or_default();
    if next_attempt_number > policy.max_attempts && !force {
        return Err(ArgoError::Provider(format!(
            "retry refused: action_type '{}' has max_attempts = {} and \
             this would be attempt #{}; pass --force to override (the \
             override is logged for audit)",
            located.record.action_type, policy.max_attempts, next_attempt_number
        )));
    }
    if force && next_attempt_number > policy.max_attempts {
        eprintln!(
            "argo worker: WARN — --force bypassed retry max_attempts={} \
             for '{}' (attempt #{} of action_id {})",
            policy.max_attempts,
            located.record.action_type,
            next_attempt_number,
            located.record.action_id,
        );
    }

    let mut new_evidence = located.record.evidence.clone();
    new_evidence.push(format!("argo://action/{}", located.record.action_id));

    let inputs = worker_actions_emit::ActionEmissionInputs {
        vertical: &loaded.manifest,
        job_id: &located.record.job_id,
        worker_id: located.record.worker_id,
        action_type: &located.record.action_type,
        payload: located.record.payload.clone(),
        item_id: located.record.item_id.as_deref(),
        platform: located.record.platform.as_deref(),
        evidence: new_evidence,
        risk_override: None,
    };
    let new_rec = worker_actions_emit::build_emission(&inputs);
    worker_actions_emit::persist_action_record(&memory, ws, &located.vertical_id, &new_rec).await?;
    if json {
        let payload = serde_json::json!({
            "retried_from": located.record.action_id,
            "new_action": new_rec,
            "retry_attempt": next_attempt_number,
            "max_attempts": policy.max_attempts,
            "forced": force && next_attempt_number > policy.max_attempts,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "retried {} → new action {} (state {}, attempt {}/{})",
            located.record.action_id,
            new_rec.action_id,
            new_rec.state,
            next_attempt_number,
            policy.max_attempts,
        );
    }
    auto_refresh_ledger(&memory, ws).await;
    Ok(())
}

/// Count prior attempts in the retry chain. Every
/// `argo://action/<id>` reference in the action's `evidence`
/// is one prior attempt; the action itself counts as the
/// initial attempt, so the total chain length is `1 +
/// retry_marker_count`. Pure function on the ActionRecord —
/// no I/O, no side effects.
pub(crate) fn count_retry_chain(rec: &ActionRecord) -> u32 {
    let prior_retries = rec
        .evidence
        .iter()
        .filter(|e| e.starts_with("argo://action/"))
        .count() as u32;
    1 + prior_retries
}
