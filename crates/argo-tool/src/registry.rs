//! Tool registry: name → `Tool`. Dispatching goes through here so the
//! capability engine sees every call (Invariant 3, §10).
use crate::tool::{Tool, ToolContext, ToolSchema};
use argo_capability::CapabilityEngine;
use argo_types::{ArgoError, ArgoResult, Capability, ToolCallResult};
use std::collections::HashMap;
use std::sync::Arc;

/// In-process registry of named tools.
///
/// ```
/// # use std::sync::Arc;
/// # use std::path::PathBuf;
/// # async fn doc() -> argo_types::ArgoResult<()> {
/// use argo_tool::{builtin_tools, ToolRegistry};
/// let mut reg = ToolRegistry::new();
/// for tool in builtin_tools(PathBuf::from("/tmp")) {
///     reg.register(tool);
/// }
/// assert!(reg.get("file_read").is_some());
/// # Ok(()) }
/// ```
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("count", &self.tools.len())
            .field("names", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ToolRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a tool. Last writer wins on duplicate name.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Lookup by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// All registered tool names (sorted for stable output).
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.tools.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Schemas for every registered tool, sorted by name.
    #[must_use]
    pub fn schemas(&self) -> Vec<ToolSchema> {
        let mut entries: Vec<(&String, &Arc<dyn Tool>)> = self.tools.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        entries.into_iter().map(|(_, t)| t.schema()).collect()
    }

    /// Dispatch a call: capability check → invoke → return result.
    ///
    /// The agent loop calls this; it's the single seam where every tool
    /// invocation is gated against the principal's capability set
    /// (Invariant 3, §10).
    pub async fn dispatch(
        &self,
        name: &str,
        args: serde_json::Value,
        ctx: &ToolContext,
        engine: &CapabilityEngine,
    ) -> ArgoResult<ToolCallResult> {
        let tool = self.get(name).ok_or_else(|| ArgoError::Tool {
            tool: name.to_string(),
            msg: "no such tool".into(),
        })?;
        // Always require dispatch authority on the tool name itself.
        engine.check(
            &ctx.principal,
            &Capability::ToolDispatch {
                tool_name_glob: name.to_string(),
            },
        )?;
        // Then check the per-arg fine-grained caps the tool declares.
        let needs = tool.required_capabilities(&args)?;
        for cap in &needs.0 {
            engine.check(&ctx.principal, cap)?;
        }
        tool.dispatch(args, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;
    use std::path::PathBuf;

    #[derive(Debug)]
    struct Noop;

    #[async_trait::async_trait]
    impl Tool for Noop {
        fn name(&self) -> &str {
            "noop"
        }
        fn description(&self) -> &str {
            "no-op"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type":"object"})
        }
        fn required_capabilities(
            &self,
            _args: &serde_json::Value,
        ) -> ArgoResult<argo_types::CapabilitySet> {
            Ok(argo_types::CapabilitySet::empty())
        }
        async fn dispatch(
            &self,
            _args: serde_json::Value,
            ctx: &ToolContext,
        ) -> ArgoResult<ToolCallResult> {
            Ok(crate::tool::success_text(ctx.call_id, "ok".into(), 0))
        }
    }

    #[tokio::test]
    async fn unknown_tool_yields_tool_error() {
        let reg = ToolRegistry::new();
        let engine = CapabilityEngine::with_defaults();
        engine
            .bind(
                Principal::Local {
                    os_user: "u".into(),
                },
                "owner".into(),
            )
            .unwrap();
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            PathBuf::from("/tmp"),
        );
        let err = reg
            .dispatch("does-not-exist", serde_json::json!({}), &ctx, &engine)
            .await
            .unwrap_err();
        assert!(matches!(err, ArgoError::Tool { .. }));
    }

    #[tokio::test]
    async fn dispatch_denies_when_principal_lacks_tool_dispatch() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(Noop));
        let engine = CapabilityEngine::with_defaults();
        engine
            .bind(
                Principal::Local {
                    os_user: "g".into(),
                },
                "guest".into(),
            )
            .unwrap();
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "g".into(),
            },
            PathBuf::from("/tmp"),
        );
        let err = reg
            .dispatch("noop", serde_json::json!({}), &ctx, &engine)
            .await
            .unwrap_err();
        assert!(matches!(err, ArgoError::RbacDenied { .. }));
    }

    #[tokio::test]
    async fn schemas_are_sorted_by_name() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(Noop));
        let schemas = reg.schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "noop");
    }
}
