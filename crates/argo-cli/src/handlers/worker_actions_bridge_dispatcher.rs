//! The `argo-cli` production [`DeliverDispatcher`] backing the in-turn
//! `enqueue_action` tool (WAVE-2 ⑥b — wires the chat→delivery bridge
//! primitive into a live agent tool).
//!
//! # Why this lives in `argo-cli`
//!
//! The bridge primitive ([`super::worker_actions_bridge::enqueue_action_for_delivery`])
//! is in the TOP `argo-cli` crate because it needs the bound
//! [`argo_vertical::VerticalManifest`], the [`MemoryStore`], the workspace,
//! and the job/worker context — none of which the lower
//! `argo-agent-runner` / `argo-tool` crates carry. The `enqueue_action`
//! TOOL lives in `argo-tool` and depends only on the
//! [`DeliverDispatcher`] trait. This struct is the bridge between the two:
//! `argo-cli` constructs it (capturing all the context the primitive needs)
//! and injects it down into the per-turn tool registry, so there is no
//! upward crate dependency.
//!
//! # Safety
//!
//! [`DeliverDispatcher::enqueue`] simply forwards to
//! `enqueue_action_for_delivery`, which fails closed (refuses undeclared /
//! `human` / un-gated actions) and ALWAYS parks the materialized row at the
//! approval gate (`state = "pending"`, `requires_approval = true`). This
//! struct adds NO execution path — it never runs an executor.

use super::worker_actions_bridge::{enqueue_action_for_delivery, BridgeOutcome};
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_tool::{DeliverDispatcher, DeliverOutcome};
use argo_types::WorkspaceId;
use async_trait::async_trait;
use std::sync::Arc;

/// Per-turn delivery backend that captures the worker run's context and
/// submits a model-proposed action through the approval gate.
///
/// Constructed once per worker turn in `process_job` (where the manifest,
/// memory store, workspace, job id, and worker id are all in hand) and
/// injected into the agent runner's per-turn tool registry. Holds owned /
/// `Arc` captures so it satisfies the `'static + Send + Sync` bound the
/// [`DeliverDispatcher`] trait requires.
pub(crate) struct BridgeDeliverDispatcher {
    store: Arc<SessionStore>,
    workspace: WorkspaceId,
    vertical: Arc<argo_vertical::VerticalManifest>,
    vertical_id: String,
    job_id: String,
    worker_id: argo_identity::WorkerId,
}

impl std::fmt::Debug for BridgeDeliverDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BridgeDeliverDispatcher")
            .field("workspace", &self.workspace)
            .field("vertical_id", &self.vertical_id)
            .field("job_id", &self.job_id)
            .field("worker_id", &self.worker_id)
            .finish()
    }
}

impl BridgeDeliverDispatcher {
    /// Capture the worker run's context for in-turn deliveries.
    ///
    /// `store` is the open session store (a fresh [`MemoryStore`] is built
    /// from it per call); `vertical` is the worker's bound manifest;
    /// `vertical_id` / `job_id` / `worker_id` scope the persisted action
    /// row exactly as the batch emission path does.
    pub(crate) fn new(
        store: Arc<SessionStore>,
        workspace: WorkspaceId,
        vertical: Arc<argo_vertical::VerticalManifest>,
        vertical_id: String,
        job_id: String,
        worker_id: argo_identity::WorkerId,
    ) -> Self {
        Self {
            store,
            workspace,
            vertical,
            vertical_id,
            job_id,
            worker_id,
        }
    }
}

#[async_trait]
impl DeliverDispatcher for BridgeDeliverDispatcher {
    async fn enqueue(
        &self,
        action_type: &str,
        payload: serde_json::Value,
    ) -> Result<DeliverOutcome, String> {
        let memory = MemoryStore::new(Arc::clone(&self.store));
        // The primitive fails closed + parks at the approval gate; it never
        // executes. We surface its typed error as a human-readable string
        // the tool reports back to the model.
        match enqueue_action_for_delivery(
            &memory,
            self.workspace,
            &self.vertical,
            &self.vertical_id,
            &self.job_id,
            self.worker_id,
            action_type,
            payload,
        )
        .await
        {
            Ok(BridgeOutcome::SubmittedForApproval {
                record,
                executor_kind,
            }) => Ok(DeliverOutcome {
                action_id: record.action_id,
                executor_kind,
            }),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "worker_actions_bridge_dispatcher_tests.rs"]
mod tests;
