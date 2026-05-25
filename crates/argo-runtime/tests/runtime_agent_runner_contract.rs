//! Relationship contract between `argo-runtime` and
//! `argo-agent-runner`.
//!
//! The broad `argo-runtime` surface contract is already covered by
//! PR #2228 while it is in review. This focused pin locks the part
//! this brief resolves: `argo-runtime` stays an upstream, store-free
//! substrate crate, and `argo-agent-runner` stays the downstream live
//! turn loop that consumes runtime primitives.

use argo_runtime::prompt_cache_affinity::PromptCacheAffinity;
use argo_runtime::{
    context_compress::{CompressionConfig, TokenCounter},
    context_truncate::{truncate_to_fit, TruncationResult},
    record_decomposition, DecomposeGate, HeuristicGate, PlannedSubtask, Planner, PlannerContext,
    PlannerError, SubtaskSink, TemplatePlanner,
};
use argo_types::{ArgoError, Message};
use async_trait::async_trait;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn dependency_direction_is_runtime_upstream_agent_runner_downstream() -> Result<(), Box<dyn Error>>
{
    let runtime_manifest = read_workspace_manifest("argo-runtime")?;
    assert!(
        !runtime_manifest.contains("argo-agent-runner"),
        "argo-runtime must not depend on the downstream live turn loop"
    );
    assert!(
        !runtime_manifest.contains("argo-store"),
        "argo-runtime must stay free of store/db dependencies"
    );
    assert!(
        !runtime_manifest.contains("argo-audit"),
        "argo-runtime must stay free of audit dependencies to avoid workspace cycles"
    );

    let agent_runner_manifest = read_workspace_manifest("argo-agent-runner")?;
    assert!(
        agent_runner_manifest.contains("argo-runtime"),
        "argo-agent-runner should consume runtime primitives explicitly"
    );
    Ok(())
}

#[tokio::test]
async fn planner_gate_and_sink_surface_stays_consumable_by_turn_loop() -> Result<(), Box<dyn Error>>
{
    let gate = HeuristicGate::new();
    let planner = TemplatePlanner::shipped_defaults();
    let sink = CountingSink::default();
    let mut ctx = PlannerContext::for_worker("wkr-contract").with_vertical("gtm-engineer");
    ctx.add_hint("caller", "argo-agent-runner::turn_planner");

    assert!(
        gate.should_decompose("research the launch and then draft follow-up", &ctx)
            .await?
    );
    let result = planner
        .decompose("promote the launch announcement", &ctx)
        .await?;
    let children = result.workflow.tasks.len();
    record_decomposition(
        &sink,
        "promote the launch announcement",
        None,
        &ctx,
        &result,
        None,
    )
    .await?;

    assert_eq!(sink.rows.load(Ordering::SeqCst), children);
    Ok(())
}

#[test]
fn context_and_affinity_primitives_stay_store_free_runtime_surfaces() {
    let cfg = CompressionConfig::default();
    assert!(cfg.enabled);

    let affinity = PromptCacheAffinity::disabled();
    assert!(!affinity.is_enabled());

    let _: fn(&[Message], usize, &dyn TokenCounter) -> Result<TruncationResult, ArgoError> =
        truncate_to_fit;
}

#[derive(Default)]
struct CountingSink {
    rows: AtomicUsize,
}

#[async_trait]
impl SubtaskSink for CountingSink {
    async fn record_subtask(
        &self,
        _parent_goal: &str,
        _parent_session_id: Option<&str>,
        _subtask: &PlannedSubtask,
    ) -> Result<(), PlannerError> {
        self.rows.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn read_workspace_manifest(crate_dir: &str) -> Result<String, Box<dyn Error>> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(crate_dir);
    path.push("Cargo.toml");
    Ok(std::fs::read_to_string(path)?)
}
