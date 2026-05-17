//! `argo worker actions <subcommand>` dispatch arms.
//!
//! Extracted from `worker_actions.rs` under the
//! `codex-split-files-450-500-wave` brief — the parent `match cmd`
//! was approaching the 420-LOC headroom target. The arms here
//! forward to the same `super::worker_actions_dispatch` /
//! `super::worker_actions_query` helpers as before; behavior
//! unchanged.
use crate::cli::cli_worker_cmds::WorkerActionsCmd;
use argo_types::ArgoResult;

pub(super) async fn run(actions_cmd: WorkerActionsCmd) -> ArgoResult<()> {
    match actions_cmd {
        WorkerActionsCmd::List {
            state,
            job_id,
            vertical,
            item_id,
            action_type,
            platform,
            limit,
            json,
        } => {
            super::worker_actions_query::list(
                &state,
                job_id.as_deref(),
                vertical.as_deref(),
                item_id.as_deref(),
                action_type.as_deref(),
                platform.as_deref(),
                limit,
                json,
            )
            .await
        }
        WorkerActionsCmd::Show { action_id, json } => {
            super::worker_actions_query::show(&action_id, json).await
        }
        WorkerActionsCmd::Approve {
            action_id,
            approved_by,
            note,
            json,
        } => {
            super::worker_actions_dispatch::approve(
                &action_id,
                approved_by.as_deref(),
                note.as_deref(),
                json,
            )
            .await
        }
        WorkerActionsCmd::ApproveBatch {
            action_ids,
            approved_by,
            note,
            deny,
            confirm,
            dry_run,
            json,
        } => {
            super::worker_actions_dispatch::approve_batch(
                &action_ids,
                approved_by.as_deref(),
                note.as_deref(),
                deny,
                confirm,
                dry_run,
                json,
            )
            .await
        }
        WorkerActionsCmd::Execute {
            action_id,
            dry_run,
            json,
        } => super::worker_actions_dispatch::execute(&action_id, dry_run, json).await,
        WorkerActionsCmd::Complete {
            action_id,
            note,
            result_json,
            json,
        } => {
            super::worker_actions_dispatch::complete(
                &action_id,
                note.as_deref(),
                result_json.as_deref(),
                json,
            )
            .await
        }
        WorkerActionsCmd::Fail {
            action_id,
            reason,
            json,
        } => super::worker_actions_dispatch::fail(&action_id, &reason, json).await,
        WorkerActionsCmd::Cancel {
            action_id,
            cancelled_by,
            note,
            json,
        } => {
            super::worker_actions_dispatch::cancel(
                &action_id,
                cancelled_by.as_deref(),
                note.as_deref(),
                json,
            )
            .await
        }
        WorkerActionsCmd::Status { action_id, json } => {
            super::worker_actions_dispatch::status(&action_id, json).await
        }
        WorkerActionsCmd::Render { vertical } => {
            super::worker_actions_dispatch::render(vertical.as_deref()).await
        }
        WorkerActionsCmd::Retry {
            action_id,
            force,
            json,
        } => super::worker_actions_dispatch::retry(&action_id, force, json).await,
    }
}
