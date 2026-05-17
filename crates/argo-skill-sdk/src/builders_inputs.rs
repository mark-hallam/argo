//! Per-node options + builder-input data shapes for the Skill SDK
//! fluent builder.
//!
//! Extracted from `builders.rs` under the
//! `codex-split-files-450-500-wave` brief to keep the parent file
//! under the 420-LOC headroom target. Behavior is unchanged; the
//! crate root re-exports both types so the public path is the
//! same as before.
use argo_skill::ir::{NodeAnnotations, SkillGraphMetadata, ToolBinding};

/// Per-node options accepted by the fluent builder.
///
/// # Example
/// ```
/// use argo_skill_sdk::{BuilderNodeOptions, ToolBinding};
///
/// let options = BuilderNodeOptions::default().tool_binding(ToolBinding {
///     mcp_server: "slack".into(),
///     tool_name: "chat.postMessage".into(),
/// });
/// assert_eq!(options.annotations.tool_binding.unwrap().tool_name, "chat.postMessage");
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BuilderNodeOptions {
    /// Optional observability and policy metadata for the node.
    pub annotations: NodeAnnotations,
}

impl BuilderNodeOptions {
    /// Attach an MCP tool binding to the node annotations.
    pub fn tool_binding(mut self, binding: ToolBinding) -> Self {
        self.annotations.tool_binding = Some(binding);
        self
    }

    /// Set a confidence annotation on the node.
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.annotations.confidence = Some(confidence);
        self
    }
}

/// Optional metadata used to initialize a [`crate::SkillBuilder`].
///
/// # Example
/// ```
/// use argo_skill_sdk::SkillBuilderInput;
///
/// let input = SkillBuilderInput::default().name("recruiter-screen").version("1.0.0");
/// assert_eq!(input.name.as_deref(), Some("recruiter-screen"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillBuilderInput {
    /// Skill name.
    pub name: Option<String>,
    /// Skill version.
    pub version: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Provenance tag such as `hand-authored` or `sdk`.
    pub source: Option<String>,
}

impl SkillBuilderInput {
    /// Set the skill name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the skill version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the skill description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the graph source/provenance tag.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl From<SkillBuilderInput> for SkillGraphMetadata {
    fn from(input: SkillBuilderInput) -> Self {
        Self {
            name: input.name,
            version: input.version,
            description: input.description,
            source: input.source,
        }
    }
}
