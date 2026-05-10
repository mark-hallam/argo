//! Manifest schema for vertical templates.
//!
//! A vertical is defined by a single `vertical.toml` file at the
//! root of its directory, plus referenced files (prompts, skills,
//! workflows, eval suites) at relative paths. This module hosts
//! the strongly-typed manifest representation + a TOML parser.
//!
//! Validation (file references resolve, IDs are well-formed,
//! allowed/denied tools don't conflict) lives in `validator.rs`.
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Manifest schema version that this argo build understands.
/// Loader rejects manifests with any other `schema_version` so a
/// future v2 schema must land via `argo vertical migrate` or via
/// reading the older shape additively.
pub const SCHEMA_VERSION: &str = "1";

/// Top-level `vertical.toml` manifest. The only required sections
/// are `schema_version` + `[vertical]`. Every other section is
/// optional, so the minimum viable manifest is a few lines.
///
/// `Eq` is intentionally not derived — `success_metrics.csat_threshold`
/// is `f64`, which doesn't impl `Eq`. Use field-level comparisons
/// (or `PartialEq`) in tests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct VerticalManifest {
    /// Schema version. Must match [`SCHEMA_VERSION`].
    pub schema_version: String,
    /// Identity + metadata.
    pub vertical: VerticalIdentity,
    /// Optional role definition (system prompt path + persona).
    #[serde(default)]
    pub role: Option<RoleSection>,
    /// Optional provider routing hint.
    #[serde(default)]
    pub provider: Option<ProviderSection>,
    /// Optional tool allow/deny lists.
    #[serde(default)]
    pub tools: Option<ToolsSection>,
    /// Optional memory schema + seed.
    #[serde(default)]
    pub memory: Option<MemorySection>,
    /// Optional auto-attached skills bundle.
    #[serde(default)]
    pub skills: Option<SkillsSection>,
    /// Optional workflow file references (Phase 5).
    #[serde(default)]
    pub workflows: Option<WorkflowsSection>,
    /// Optional escalation rules.
    #[serde(default)]
    pub escalation: Option<EscalationSection>,
    /// Optional success metrics.
    #[serde(default)]
    pub success_metrics: Option<SuccessMetricsSection>,
    /// Optional eval suite refs.
    #[serde(default)]
    pub evaluation: Option<EvaluationSection>,
    /// Optional Worker v3 action vocabulary (#729). When set,
    /// the worker may emit action proposals using the declared
    /// `action_type` names and the OS stamps the per-action
    /// `requires_approval` and `risk_level` from this section
    /// onto every persisted `worker_action` row. Verticals
    /// without this section behave as v2 — they produce
    /// sectioned deliverables but no ledger entries (action
    /// emission is opt-in per vertical).
    #[serde(default)]
    pub actions: Option<ActionsSection>,
    /// Optional MCP server declarations workers may call.
    #[serde(default)]
    pub mcp_servers: Option<McpServersSection>,
    /// Optional gateway exposure.
    #[serde(default)]
    pub gateway: Option<GatewaySection>,
    /// Optional TUI metadata.
    #[serde(default)]
    pub tui: Option<TuiSection>,
}

/// `[vertical]` — required. Globally identifies the vertical.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct VerticalIdentity {
    /// Globally unique slug. Lowercase, dashes, no slashes.
    /// Validator enforces the pattern.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Optional one-paragraph description.
    #[serde(default)]
    pub description: String,
    /// SemVer-style version.
    pub version: String,
    /// Optional contributor list.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Optional tag list (used for discovery).
    #[serde(default)]
    pub tags: Vec<String>,
}

/// `[role]` — system prompt + persona file references.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct RoleSection {
    /// Relative path to a markdown file holding the agent's
    /// system prompt.
    pub system_prompt: String,
    /// Optional relative path to a persona/voice doc layered on
    /// top of the system prompt.
    #[serde(default)]
    pub persona: Option<String>,
}

/// `[provider]` — provider routing hints. The vertical declares
/// a default + fallback model id; the cost-tier feeds into
/// existing argo-routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct ProviderSection {
    /// Primary model id, formatted `provider/model` (e.g.,
    /// `anthropic/claude-sonnet-4-5`).
    pub default: String,
    /// Optional fallback model id used when the default is
    /// unavailable.
    #[serde(default)]
    pub fallback: Option<String>,
    /// Cost-tier hint passed to argo-routing.
    #[serde(default = "default_cost_tier")]
    pub cost_tier: String,
}

fn default_cost_tier() -> String {
    "balanced".to_string()
}

/// `[tools]` — allow/deny lists derived into a CapabilityProfile
/// at session create. Deny-wins precedence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct ToolsSection {
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub denied: Vec<String>,
}

/// `[memory]` — schema validation + seed-row directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct MemorySection {
    /// Optional path to a TOML file describing what shape memory
    /// writes must conform to.
    #[serde(default)]
    pub schema: Option<String>,
    /// Optional path to a directory of markdown files preloaded
    /// into memory on session start.
    #[serde(default)]
    pub seed: Option<String>,
}

/// `[skills]` — directory of skill markdown files auto-attached
/// to every session bound to this vertical.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct SkillsSection {
    pub bundle: String,
}

/// `[workflows]` — file references for Phase 5's workflow
/// primitive. Carried in the schema today so v1 manifests can
/// declare them; runtime is a no-op until #653.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkflowsSection {
    #[serde(default)]
    pub files: Vec<String>,
}

/// `[escalation]` — when to hand off to a human. Mostly carried
/// today for declarative completeness; enforcement is Phase 6.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct EscalationSection {
    #[serde(default)]
    pub human_approval_required: Vec<String>,
    #[serde(default)]
    pub escalate_after_failures: Option<u32>,
    #[serde(default)]
    pub escalate_to: Option<String>,
}

/// `[success_metrics]` — release-gate thresholds. Enforced by the
/// eval CI integration in Phase 7. `Eq` not derived (csat_threshold
/// is `f64`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct SuccessMetricsSection {
    #[serde(default)]
    pub deflection_target_pct: Option<u32>,
    #[serde(default)]
    pub median_response_seconds: Option<u32>,
    #[serde(default)]
    pub csat_threshold: Option<f64>,
    #[serde(default)]
    pub required_audit_chain_intact: Option<bool>,
}

/// `[evaluation]` — eval suite file refs (JSONL, one task per
/// line).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct EvaluationSection {
    #[serde(default)]
    pub suites: Vec<String>,
}

/// `[actions]` — Worker v3 action vocabulary (#729).
///
/// Each entry is a per-action declaration. The action's name is
/// the table key (e.g. `[actions.create_listing_draft]`). The
/// declaration says how the action is classified (risk level,
/// approval requirement) and how it executes (which executor,
/// what platforms it can target, what payload shape it expects).
///
/// The OS uses this to:
///   1. Validate every action emission against the declared
///      vocabulary — undeclared `action_type` strings are
///      rejected at emission time, NOT at the prompt level.
///   2. Stamp `requires_approval` + `risk_level` on every
///      persisted `worker_action` row. The model can't override
///      the manifest's classification.
///   3. Render the human-executable artifact bundle via the
///      per-action `executor` + `payload_schema`.
///
/// See `docs/design/worker_v3_execution.md` for the full design.
///
/// `ActionsSection`, `ActionDeclaration`, `ActionRiskLevel`, and
/// `RetryPolicy` live in `manifest_actions.rs` so this file stays
/// under the 500-LOC budget; they're re-exported here so the
/// public API surface (`argo_vertical::ActionsSection`, etc.) is
/// unchanged.
pub use crate::manifest_actions::{
    ActionDeclaration, ActionRiskLevel, ActionsSection, RetryPolicy,
};
pub use crate::manifest_mcp::{McpServerDeclaration, McpServersSection};

/// `[gateway]` — public HTTP endpoints to mount when the vertical
/// is loaded. Wired by Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct GatewaySection {
    #[serde(default)]
    pub public_endpoints: Vec<String>,
    #[serde(default)]
    pub auth_required: Option<bool>,
}

/// `[tui]` — optional UI metadata for the existing TUI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct TuiSection {
    #[serde(default)]
    pub overlay_label: Option<String>,
    #[serde(default)]
    pub default_filters: BTreeMap<String, String>,
}

/// Parse a manifest from a TOML string. Schema-version + structure
/// enforced; cross-references against on-disk files are checked by
/// the validator instead.
pub fn parse(src: &str) -> ArgoResult<VerticalManifest> {
    let m: VerticalManifest =
        toml::from_str(src).map_err(|e| ArgoError::Schema(format!("vertical manifest: {e}")))?;
    if m.schema_version != SCHEMA_VERSION {
        return Err(ArgoError::Schema(format!(
            "vertical manifest schema_version '{}' is not supported by this argo build (expected '{}')",
            m.schema_version, SCHEMA_VERSION
        )));
    }
    crate::manifest_validate::validate_strict(&m)?;
    Ok(m)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
