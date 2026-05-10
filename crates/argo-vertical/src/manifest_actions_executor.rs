//! Executor-declaration types extracted from `manifest_actions.rs`
//! to keep that file under the per-file 500-LOC budget.
//!
//! Owns:
//!
//! * `ExecutorDeclaration` — the `executor = ...` field on each
//!   action (Stream A `#A-executor-trait`).
//! * `ExecutorTagged` — the inline-table form of an executor
//!   declaration.
//!
//! Re-exported from `manifest_actions` (and thence from the crate
//! root via `lib::ExecutorDeclaration` etc.) so the public API
//! surface is unchanged.

use serde::{Deserialize, Serialize};

/// `executor = "..."` field on an `ActionDeclaration`. Accepts
/// either the legacy string form (`"human"`, `"mcp"`,
/// `"computer_use"`, `"api"`) or the new tagged table form. See
/// `ActionDeclaration::executor` for the full schema.
///
/// Untagged so serde tries `Legacy(String)` first (matches a
/// bare TOML string) and falls through to `Tagged(...)` for
/// inline-table forms. The kind string is then routed to the
/// right `Executor` impl by the dispatch shim in
/// `worker_actions_executor::resolve_executor`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ExecutorDeclaration {
    /// Legacy string form: `executor = "human"`. The string is
    /// the executor kind. Existing manifests authored before
    /// the tagged form continue to parse unchanged.
    Legacy(String),
    /// Tagged table form: `executor = { kind = "...", ... }`.
    /// Carries optional config fields used by the corresponding
    /// `Executor` impl (e.g. `profile` + `budget_usd` for
    /// `computer_use`, `server` + `tool` for `mcp`).
    Tagged(ExecutorTagged),
}

impl Default for ExecutorDeclaration {
    fn default() -> Self {
        Self::Legacy("human".to_string())
    }
}

impl ExecutorDeclaration {
    /// Stable kind string, regardless of which form the manifest
    /// used. The dispatch shim parses this into an `ExecutorKind`
    /// and routes to the right impl.
    ///
    /// # Example
    /// ```
    /// use argo_vertical::ExecutorDeclaration;
    /// let legacy = ExecutorDeclaration::Legacy("human".into());
    /// assert_eq!(legacy.kind_str(), "human");
    /// ```
    #[must_use]
    pub fn kind_str(&self) -> &str {
        match self {
            Self::Legacy(s) => s,
            Self::Tagged(t) => &t.kind,
        }
    }

    /// Optional computer-use profile (sandbox image / browser
    /// flavour). `None` for the legacy string form.
    ///
    /// # Example
    /// ```
    /// use argo_vertical::{ExecutorDeclaration, ExecutorTagged};
    /// let mut t = ExecutorTagged::default();
    /// t.kind = "computer_use".into();
    /// t.profile = Some("marketplace_browser".into());
    /// let decl = ExecutorDeclaration::Tagged(t);
    /// assert_eq!(decl.profile(), Some("marketplace_browser"));
    /// ```
    #[must_use]
    pub fn profile(&self) -> Option<&str> {
        match self {
            Self::Legacy(_) => None,
            Self::Tagged(t) => t.profile.as_deref(),
        }
    }

    /// Optional MCP server name (used by the MCP executor).
    ///
    /// # Example
    /// ```
    /// use argo_vertical::ExecutorDeclaration;
    /// assert_eq!(
    ///     ExecutorDeclaration::Legacy("mcp".into()).mcp_server(),
    ///     None,
    /// );
    /// ```
    #[must_use]
    pub fn mcp_server(&self) -> Option<&str> {
        match self {
            Self::Legacy(_) => None,
            Self::Tagged(t) => t.server.as_deref(),
        }
    }

    /// Optional MCP tool name within the server.
    ///
    /// # Example
    /// ```
    /// use argo_vertical::ExecutorDeclaration;
    /// assert_eq!(
    ///     ExecutorDeclaration::default().mcp_tool(),
    ///     None,
    /// );
    /// ```
    #[must_use]
    pub fn mcp_tool(&self) -> Option<&str> {
        match self {
            Self::Legacy(_) => None,
            Self::Tagged(t) => t.tool.as_deref(),
        }
    }

    /// Optional API binding key (used by the api executor).
    ///
    /// # Example
    /// ```
    /// use argo_vertical::ExecutorDeclaration;
    /// assert_eq!(
    ///     ExecutorDeclaration::Legacy("api".into()).api_binding(),
    ///     None,
    /// );
    /// ```
    #[must_use]
    pub fn api_binding(&self) -> Option<&str> {
        match self {
            Self::Legacy(_) => None,
            Self::Tagged(t) => t.binding.as_deref(),
        }
    }

    /// Optional per-action USD budget cap (used by the
    /// computer-use executor to refuse runaway sessions).
    ///
    /// # Example
    /// ```
    /// use argo_vertical::ExecutorDeclaration;
    /// assert_eq!(
    ///     ExecutorDeclaration::default().budget_usd(),
    ///     None,
    /// );
    /// ```
    #[must_use]
    pub fn budget_usd(&self) -> Option<f64> {
        match self {
            Self::Legacy(_) => None,
            Self::Tagged(t) => t.budget_usd,
        }
    }
}

/// Tagged-table form of `ExecutorDeclaration`. All fields beyond
/// `kind` are optional; the corresponding `Executor` impl reads
/// only the fields it cares about. See
/// `ActionDeclaration::executor` for examples.
///
/// # Example
/// ```
/// use argo_vertical::ExecutorTagged;
/// let mut t = ExecutorTagged::default();
/// t.kind = "mcp".into();
/// t.server = Some("gmail".into());
/// t.tool = Some("drafts.send".into());
/// assert_eq!(t.kind, "mcp");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ExecutorTagged {
    /// Executor kind: `human | mcp | computer_use | api`.
    pub kind: String,
    /// Computer-use profile (e.g. `marketplace_browser`,
    /// `desktop_full`). Picks the sandbox image + tool whitelist.
    #[serde(default)]
    pub profile: Option<String>,
    /// Per-action USD budget cap. The computer-use executor
    /// refuses to start a session that would exceed this; the
    /// MCP / API executors read it for cost-aware retries.
    #[serde(default)]
    pub budget_usd: Option<f64>,
    /// MCP server name (only meaningful when `kind = "mcp"`).
    #[serde(default)]
    pub server: Option<String>,
    /// MCP tool name within the server.
    #[serde(default)]
    pub tool: Option<String>,
    /// API binding key (only meaningful when `kind = "api"`).
    /// Resolves to a registered REST binding under
    /// `manifests/integrations/<binding>.toml`.
    #[serde(default)]
    pub binding: Option<String>,
}
