//! Portable Skill IR bundle format (Track L10).
//!
//! Bundles carry canonical graph JSON plus the graph hash, but never
//! tenant identity. Importers validate the schema tag and recompute the
//! hash before adopting the graph into their own tenant-scoped store.

use crate::ir::{canonical_hash_hex, validate_graph, SkillGraph};
use argo_types::{ArgoError, ArgoResult, SkillId};
use serde::{Deserialize, Serialize};

/// Schema tag for v1 portable Skill IR bundles.
pub const SKILL_IR_BUNDLE_SCHEMA: &str = "skycot.skill_ir.bundle.v1";

/// Tenant-portable Skill IR export.
///
/// # Example
/// ```
/// use argo_skill::ir::{
///     export_skill_ir_bundle, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph,
/// };
/// use argo_types::SkillId;
///
/// let graph = SkillGraph {
///     entry: NodeId::from("n1"),
///     nodes: vec![Node {
///         id: NodeId::from("n1"),
///         label: "start".into(),
///         kind: NodeKind::Action { instruction: "say hi".into() },
///         annotations: NodeAnnotations::default(),
///     }],
///     edges: vec![],
///     variables: vec![],
///     metadata: Default::default(),
/// };
/// let bundle = export_skill_ir_bundle(SkillId::new(), "v1", &graph).unwrap();
/// assert_eq!(bundle.schema_version, "skycot.skill_ir.bundle.v1");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillIrBundle {
    /// Bundle schema identifier. Import refuses unknown values.
    pub schema_version: String,
    /// Source skill id from the exporting workspace. Import endpoints
    /// treat this as provenance only; the URL skill id is the target.
    pub source_skill_id: String,
    /// Source IR version string.
    pub version: String,
    /// Canonical SHA-256 hash of [`graph`], hex encoded.
    pub ir_hash: String,
    /// Canonical skill graph payload.
    pub graph: SkillGraph,
}

/// Build a tenant-portable bundle from a graph.
///
/// # Example
/// ```
/// use argo_skill::ir::{export_skill_ir_bundle, SkillGraph};
/// use argo_types::SkillId;
///
/// let graph = SkillGraph::default();
/// let bundle = export_skill_ir_bundle(SkillId::new(), "draft", &graph).unwrap();
/// assert_eq!(bundle.version, "draft");
/// ```
pub fn export_skill_ir_bundle(
    skill_id: SkillId,
    version: &str,
    graph: &SkillGraph,
) -> ArgoResult<SkillIrBundle> {
    Ok(SkillIrBundle {
        schema_version: SKILL_IR_BUNDLE_SCHEMA.to_string(),
        source_skill_id: skill_id.to_string(),
        version: version.to_string(),
        ir_hash: canonical_hash_hex(graph)?,
        graph: graph.clone(),
    })
}

/// Validate a portable bundle and return its graph for adoption.
///
/// # Example
/// ```
/// use argo_skill::ir::{
///     export_skill_ir_bundle, graph_from_skill_ir_bundle, Node, NodeAnnotations, NodeId,
///     NodeKind, SkillGraph,
/// };
/// use argo_types::SkillId;
///
/// let graph = SkillGraph {
///     entry: NodeId::from("n1"),
///     nodes: vec![Node {
///         id: NodeId::from("n1"),
///         label: "start".into(),
///         kind: NodeKind::Action { instruction: "say hi".into() },
///         annotations: NodeAnnotations::default(),
///     }],
///     edges: vec![],
///     variables: vec![],
///     metadata: Default::default(),
/// };
/// let bundle = export_skill_ir_bundle(SkillId::new(), "draft", &graph).unwrap();
/// let imported = graph_from_skill_ir_bundle(&bundle).unwrap();
/// assert_eq!(imported, graph);
/// ```
pub fn graph_from_skill_ir_bundle(bundle: &SkillIrBundle) -> ArgoResult<SkillGraph> {
    if bundle.schema_version != SKILL_IR_BUNDLE_SCHEMA {
        return Err(ArgoError::Invariant(format!(
            "skill_ir bundle: unsupported schema '{}'",
            bundle.schema_version
        )));
    }
    validate_graph(&bundle.graph)
        .map_err(|e| ArgoError::Invariant(format!("skill_ir bundle: {e}")))?;
    let actual = canonical_hash_hex(&bundle.graph)?;
    if actual != bundle.ir_hash {
        return Err(ArgoError::Invariant(format!(
            "skill_ir bundle: hash mismatch (expected {}, got {actual})",
            bundle.ir_hash
        )));
    }
    Ok(bundle.graph.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Node, NodeAnnotations, NodeId, NodeKind};

    fn graph(label: &str) -> SkillGraph {
        SkillGraph {
            entry: NodeId::from("n1"),
            nodes: vec![Node {
                id: NodeId::from("n1"),
                label: label.into(),
                kind: NodeKind::Action {
                    instruction: format!("do {label}"),
                },
                annotations: NodeAnnotations::default(),
            }],
            edges: vec![],
            variables: vec![],
            metadata: Default::default(),
        }
    }

    #[test]
    fn bundle_round_trips_without_tenant_identity() {
        let skill = SkillId::new();
        let graph = graph("export");
        let bundle = export_skill_ir_bundle(skill, "v1", &graph).expect("export");

        assert_eq!(bundle.schema_version, SKILL_IR_BUNDLE_SCHEMA);
        assert_eq!(bundle.source_skill_id, skill.to_string());
        assert_eq!(bundle.version, "v1");
        assert_eq!(graph_from_skill_ir_bundle(&bundle).expect("import"), graph);
        let encoded = serde_json::to_string(&bundle).expect("json");
        assert!(!encoded.contains("tenant"));
    }

    #[test]
    fn import_rejects_unknown_schema() {
        let mut bundle = export_skill_ir_bundle(SkillId::new(), "v1", &graph("x")).unwrap();
        bundle.schema_version = "other".into();

        let err = graph_from_skill_ir_bundle(&bundle).expect_err("schema rejected");
        assert!(err.to_string().contains("unsupported schema"));
    }

    #[test]
    fn import_rejects_hash_mismatch() {
        let mut bundle = export_skill_ir_bundle(SkillId::new(), "v1", &graph("old")).unwrap();
        bundle.graph = graph("new");

        let err = graph_from_skill_ir_bundle(&bundle).expect_err("hash rejected");
        assert!(err.to_string().contains("hash mismatch"));
    }
}
