//! Save pipeline for edited Skill IR graphs (Track L7).
//!
//! The canvas editor writes a new immutable IR version, keeps the
//! existing hash-chain guarantees from [`SqliteSkillIrStore`], and
//! opens a lightweight A/B run against the previous IR version when
//! one exists. The A/B row uses the shipped `skill_ab_test_run` table;
//! promotion remains the existing operator-controlled flow.

use crate::ir::{canonical_hash_hex, NodeKind, SkillGraph, SkillIrStore as _, SqliteSkillIrStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use chrono::Utc;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

const TARGET_AB_RUNS: i64 = 100;

/// Result returned after a canvas save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavePipelineOutcome {
    /// Newly persisted immutable IR version.
    pub version: String,
    /// Canonical SHA-256 hash of the saved graph, hex encoded.
    pub ir_hash_hex: String,
    /// Newly opened A/B run id when a previous IR version existed.
    pub ab_test_run_id: Option<String>,
}

/// Validate an edited graph before writing it.
///
/// This is stricter than L1 structural validation: an editor-save graph
/// must have a reachable entry node, at least one terminal node, and no
/// orphaned nodes outside the entry-reachable topology.
pub fn validate_editable_topology(graph: &SkillGraph) -> Result<(), String> {
    if graph.nodes.is_empty() {
        return Err("graph has zero nodes".into());
    }
    let ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    if !ids.contains(graph.entry.as_str()) {
        return Err(format!("entry '{}' is not a node", graph.entry.as_str()));
    }

    let mut outdegree: HashMap<&str, usize> =
        graph.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    let mut adjacency: HashMap<&str, Vec<&str>> = graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), Vec::new()))
        .collect();
    for edge in &graph.edges {
        let from = edge.from.as_str();
        let to = edge.to.as_str();
        if !ids.contains(from) {
            return Err(format!(
                "edge '{}' has unknown source '{}'",
                edge.id.as_str(),
                from
            ));
        }
        if !ids.contains(to) {
            return Err(format!(
                "edge '{}' has unknown destination '{}'",
                edge.id.as_str(),
                to
            ));
        }
        *outdegree.entry(from).or_insert(0) += 1;
        adjacency.entry(from).or_default().push(to);
    }
    for node in &graph.nodes {
        let from = node.id.as_str();
        for to in structural_targets(&node.kind) {
            if !ids.contains(to) {
                return Err(format!(
                    "node '{}' references unknown body node '{}'",
                    from, to
                ));
            }
            *outdegree.entry(from).or_insert(0) += 1;
            adjacency.entry(from).or_default().push(to);
        }
    }
    if !outdegree.values().any(|count| *count == 0) {
        return Err("graph must have at least one terminal node".into());
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue = VecDeque::from([graph.entry.as_str()]);
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        for next in adjacency.get(id).into_iter().flatten() {
            queue.push_back(next);
        }
    }
    if seen.len() != graph.nodes.len() {
        let mut orphans: Vec<&str> = ids.difference(&seen).copied().collect();
        orphans.sort_unstable();
        return Err(format!(
            "graph has orphaned node(s): {}",
            orphans.join(", ")
        ));
    }
    Ok(())
}

fn structural_targets(kind: &NodeKind) -> Vec<&str> {
    match kind {
        NodeKind::Loop { body, .. } => body.iter().map(|id| id.as_str()).collect(),
        NodeKind::TryCatch {
            try_body,
            catch_body,
        } => try_body
            .iter()
            .chain(catch_body.iter())
            .map(|id| id.as_str())
            .collect(),
        _ => Vec::new(),
    }
}

/// Persist an edited graph and schedule a promotion A/B run when there
/// is a previous IR version to compare against.
pub async fn save_edited_graph(
    store: &SessionStore,
    tenant_id: TenantId,
    skill_id: SkillId,
    graph: &SkillGraph,
) -> ArgoResult<SavePipelineOutcome> {
    crate::ir::validate_graph(graph)
        .map_err(|e| ArgoError::Invariant(format!("skill_ir save: {e}")))?;
    validate_editable_topology(graph)
        .map_err(|e| ArgoError::Invariant(format!("skill_ir save: {e}")))?;
    crate::ir::ensure_schema(store).await?;
    crate::ab_test::ensure_ab_schema(store).await?;

    let ir = SqliteSkillIrStore::new(store);
    let previous = ir
        .latest(tenant_id, skill_id)
        .await?
        .map(|(version, _)| version);
    let version = next_canvas_version();
    ir.put(tenant_id, skill_id, &version, graph).await?;
    let ir_hash_hex = canonical_hash_hex(graph)?;
    let ab_test_run_id = match previous {
        Some(prev) => Some(schedule_ab_test(store, tenant_id, skill_id, &prev, &version).await?),
        None => None,
    };
    Ok(SavePipelineOutcome {
        version,
        ir_hash_hex,
        ab_test_run_id,
    })
}

fn next_canvas_version() -> String {
    format!("canvas-{}", Utc::now().timestamp_micros())
}

async fn schedule_ab_test(
    store: &SessionStore,
    tenant_id: TenantId,
    skill_id: SkillId,
    previous_version: &str,
    next_version: &str,
) -> ArgoResult<String> {
    let id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO skill_ab_test_run \
         (id, tenant_id, skill_name, variant_a, variant_b, target_runs, started_at, status) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'running')",
    )
    .bind(&id)
    .bind(tenant_id.to_string())
    .bind(skill_id.to_string())
    .bind(previous_version)
    .bind(next_version)
    .bind(TARGET_AB_RUNS)
    .bind(Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_ir ab schedule: {e}")))?;
    Ok(id)
}

#[cfg(test)]
#[path = "save_pipeline_tests.rs"]
mod tests;
