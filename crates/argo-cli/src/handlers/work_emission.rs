//! Worker v3 action-emission glue (#729).
//!
//! Sits between `process_job` (which already has the assistant
//! texts + the evidence-sufficient verdict in hand via the
//! artifact bundle) and the OS-generic `worker_actions_emit`
//! module (which owns extract / classify / build / evidence-
//! gate-downgrade rules per the design doc §2 + §5).
//!
//! Lifted out of `work.rs` to keep that file under the per-file
//! 500-LOC budget. The split is purely organisational; this
//! module is private to the handlers tree and has no public API
//! surface beyond `emit_and_persist_actions`.
//!
//! Failure semantics: a single bad action's persist or render
//! error is logged + swallowed so it can't tank the whole job's
//! artifact bundle. The session's message rows + the
//! `completed_jobs` memory row stay authoritative regardless.
//!
//! See `docs/design/worker_v3_execution.md` §2 (boundary
//! semantics) + §5 (CLI contract) for the full spec.

use super::vertical::resolve_verticals_dir;
use super::work_artifacts::ArtifactOutput;
use super::worker_actions::ActionRecord;
use super::worker_actions_emit;
use super::worker_actions_items;
use super::worker_actions_render;
use argo_memory::MemoryStore;
use argo_types::WorkspaceId;
use std::collections::HashSet;

/// Walk every assistant message text, hand each to the action-
/// emission orchestrator, persist every emitted record as a
/// `worker_action` memory row, and render `execution_plan.md`
/// alongside the rest of the artifact bundle.
///
/// Returns the persisted action records (empty when the vertical
/// isn't found, the manifest carries no `[actions]`, or no assistant
/// text contained an emission). The gateway run path
/// ([`super::work_dispatch_executor_inline`]) iterates the returned
/// records to inline-dispatch the auto-eligible ones; callers that
/// only need a count use `.len()`.
///
/// cu1f (DEC-050): the manifest resolves via
/// [`argo_vertical::load_for_worker`] — flat catalog first, then the
/// tenant-scoped `.genesis/<tenant>/<vid>/` tree (fail-closed
/// cross-tenant). The previous flat-only `load_dir` lookup silently
/// returned zero actions for EVERY Genesis-hired teammate, so their
/// proposals never reached the action ledger or `/approvals` — the
/// same flat-only-loader bug the Run-now gate fixed in
/// `routes_worker_run_exec::vertical_loads`.
pub(crate) async fn emit_and_persist_actions(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    tenant_id: argo_types::TenantId,
    vertical_id: &str,
    job_id: &str,
    artifact: &ArtifactOutput,
) -> Vec<ActionRecord> {
    let dir = resolve_verticals_dir();
    let Some(loaded) =
        argo_vertical::load_for_worker(&dir, &tenant_id.to_string(), vertical_id)
    else {
        eprintln!(
            "argo work: warning — vertical `{vertical_id}` not loadable for tenant \
             `{tenant_id}` (flat or .genesis); action emission skipped"
        );
        return Vec::new();
    };
    let manifest = &loaded.manifest;
    // #769: snapshot the sold-items set ONCE per emission pass.
    // Walking the ledger per assistant_text would re-read the
    // same memory rows; one snapshot for the duration of the
    // pass is correct because mark_item_sold rows can only land
    // via operator action, which doesn't race with the worker.
    let sold_items: HashSet<String> =
        match worker_actions_items::compute_sold_items(memory, workspace, vertical_id).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "argo work: warning — sold-items snapshot failed for `{vertical_id}`: {e}; \
                 emission gate inactive for this pass"
                );
                HashSet::new()
            }
        };
    let mut all_records = Vec::new();
    for text in &artifact.assistant_texts {
        // The work runtime is the platform-default worker context;
        // explicit per-worker binding (Stream
        // `claude-worker-instance-policy` follow-up) lands when the
        // runtime carries a `WorkerId` through the queue header.
        let records = worker_actions_emit::emit_actions_from_text(
            manifest,
            job_id,
            argo_identity::WorkerId::nil(),
            text,
            artifact.evidence_sufficient,
            &sold_items,
        );
        for rec in records {
            if let Err(e) =
                worker_actions_emit::persist_action_record(memory, workspace, vertical_id, &rec)
                    .await
            {
                eprintln!(
                    "argo work: warning — could not persist action {}: {e}",
                    rec.action_id
                );
                continue;
            }
            all_records.push(rec);
        }
    }
    let plan = worker_actions_emit::render_execution_plan(&all_records);
    let plan_path = artifact.output_dir.join("execution_plan.md");
    if let Err(e) = std::fs::write(&plan_path, &plan) {
        eprintln!("argo work: warning — execution_plan.md not written: {e}");
    }
    let actions_json = worker_actions_render::render_actions_json(
        job_id,
        vertical_id,
        artifact.evidence_sufficient,
        &all_records,
    );
    let actions_json_path = artifact.output_dir.join("actions.json");
    if let Err(e) = std::fs::write(&actions_json_path, &actions_json) {
        eprintln!("argo work: warning — actions.json not written: {e}");
    }
    augment_result_json(&artifact.output_dir, &all_records);
    all_records
}

/// Re-read `result.json`, splice in the per-job action artifact
/// pointers (`execution_plan_path`, `actions_path`, plus a small
/// `actions` summary block), and write it back.
///
/// Done as a post-emission augment because `write_artifacts`
/// runs BEFORE emission (it computes the evidence verdict that
/// emission consumes). The alternative — passing emission
/// results back into `write_artifacts` — would invert the
/// ordering and introduce a cycle.
///
/// Failure is non-fatal: a missing or malformed `result.json`
/// just logs a warning and leaves the bundle as-is. The
/// markdown + json artifacts on disk are still authoritative.
fn augment_result_json(
    output_dir: &std::path::Path,
    records: &[crate::handlers::worker_actions::ActionRecord],
) {
    let path = output_dir.join("result.json");
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("argo work: warning — result.json missing for action augment: {e}");
            return;
        }
    };
    let mut v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("argo work: warning — result.json unparseable for action augment: {e}");
            return;
        }
    };
    let map = match v.as_object_mut() {
        Some(m) => m,
        None => {
            eprintln!("argo work: warning — result.json not a JSON object; action augment skipped");
            return;
        }
    };
    let mut pending_approval = 0u32;
    let mut pending_autonomous = 0u32;
    let mut blocked = 0u32;
    for r in records {
        match (r.state.as_str(), r.requires_approval) {
            ("pending", true) => pending_approval += 1,
            ("pending", false) => pending_autonomous += 1,
            ("blocked", _) => blocked += 1,
            _ => {}
        }
    }
    map.insert(
        "execution_plan_path".to_string(),
        serde_json::Value::String(
            output_dir
                .join("execution_plan.md")
                .to_string_lossy()
                .into(),
        ),
    );
    map.insert(
        "actions_path".to_string(),
        serde_json::Value::String(output_dir.join("actions.json").to_string_lossy().into()),
    );
    map.insert(
        "actions".to_string(),
        serde_json::json!({
            "total": records.len(),
            "pending_approval": pending_approval,
            "pending_autonomous": pending_autonomous,
            "blocked": blocked,
        }),
    );
    if let Err(e) = std::fs::write(
        &path,
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into()),
    ) {
        eprintln!("argo work: warning — could not rewrite result.json with action info: {e}");
    }
}
