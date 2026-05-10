//! Provider-manifest schema, deserialization, and validation.
//!
//! The manifest is the source of truth for *what* a provider can do and *how*
//! to talk to it. Quirks live in TOML, not Rust (Invariant 4 — see CLAUDE.md).
//!
//! Spec: `docs/plan/argo-build-plan.md` §8.1.
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod transport;
pub use transport::{StreamingMode, TransportSpec};

/// Top-level provider manifest deserialized from a `.toml` file.
///
/// ```
/// use argo_provider::ProviderManifest;
/// let toml = r#"
/// [provider]
/// id = "openai"
/// family = "openai-compatible"
/// display_name = "OpenAI"
/// base_url = "https://api.openai.com"
///
/// [auth]
/// api_key_env = "OPENAI_API_KEY"
/// header_name = "Authorization"
/// header_format = "Bearer {key}"
///
/// [transport]
/// streaming = "sse"
/// chat_path = "/v1/chat/completions"
/// connect_timeout_ms = 5000
/// read_timeout_ms = 60000
/// max_retries = 3
/// initial_backoff_ms = 200
/// max_backoff_ms = 4000
///
/// [tool_calls]
/// format = "openai_tool_calls"
///
/// [[models]]
/// id = "gpt-4o-mini"
/// context_window = 128000
/// "#;
/// let m: ProviderManifest = toml::from_str(toml).unwrap();
/// assert_eq!(m.provider.id, "openai");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManifest {
    /// Identity, family, and base URL.
    pub provider: ProviderHeader,
    /// Auth header derivation.
    pub auth: AuthSpec,
    /// Wire transport (streaming, retry, timeouts, paths, heartbeat).
    pub transport: TransportSpec,
    /// Tool-call wire format.
    pub tool_calls: ToolCallsSpec,
    /// Static model list (may be augmented by discovery at runtime).
    #[serde(default)]
    pub models: Vec<ModelSpec>,
    /// Optional `[discovery]` block for `model = "latest"` resolution (§8.2).
    #[serde(default)]
    pub discovery: Option<DiscoverySpec>,
    /// Optional `[reasoning]` block for thinking-token preservation.
    #[serde(default)]
    pub reasoning: Option<ReasoningSpec>,
    /// Declared capabilities — every public-facing call surface should
    /// reference these (Invariant 3).
    #[serde(default)]
    pub capabilities: CapabilitiesSpec,
    /// `[quirks]` table — provider-family-specific knobs that the
    /// transport may consume opaquely (e.g. Anthropic's
    /// `send-thinking-blocks` or `prompt-caching-breakpoints`). The
    /// argo-provider core never looks at these; only family transports
    /// do. Schema-flexible by design.
    ///
    /// Spec: `docs/plan/argo-build-plan.md` §8.1 (issue #2 / F-3).
    #[serde(default)]
    pub quirks: HashMap<String, toml::Value>,
    /// Optional `[hooks]` table — escape hatch for behaviours that
    /// `[quirks]` cannot express. Hook names are resolved by the host
    /// (plugin protocol — Phase 4); when absent, the family transport
    /// uses its default code path.
    ///
    /// Spec: `docs/plan/argo-build-plan.md` §8.1 (issue #2 / F-4).
    #[serde(default)]
    pub hooks: Option<HooksSpec>,
}

/// `[provider]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHeader {
    /// Stable id (e.g. `"openai"`, `"openrouter"`).
    pub id: String,
    /// Family — selects the transport implementation.
    /// In Phase 2, only `"openai-compatible"` is wired up; the rest stub.
    pub family: ProviderFamily,
    /// Human-readable name.
    pub display_name: String,
    /// Base URL (no trailing slash).
    pub base_url: String,
}

/// Provider families known to argo-provider.
///
/// ```
/// use argo_provider::ProviderFamily;
/// let f: ProviderFamily = serde_json::from_str("\"openai-compatible\"").unwrap();
/// assert!(matches!(f, ProviderFamily::OpenaiCompatible));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderFamily {
    /// OpenAI Chat Completions wire (OpenAI, OpenRouter, vLLM, Together, ...).
    OpenaiCompatible,
    /// OpenAI Responses API.
    OpenaiResponses,
    /// Anthropic Messages API.
    Anthropic,
    /// AWS Bedrock InvokeModel.
    Bedrock,
}

/// `[auth]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSpec {
    /// Env var to read the API key from (Capability::EnvRead).
    pub api_key_env: String,
    /// HTTP header name (default: `Authorization`).
    #[serde(default = "default_auth_header")]
    pub header_name: String,
    /// Header value template; `{key}` is substituted.
    #[serde(default = "default_auth_format")]
    pub header_format: String,
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}

fn default_auth_format() -> String {
    "Bearer {key}".to_string()
}

/// `[tool_calls]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallsSpec {
    /// Wire format identifier.
    pub format: ToolCallFormat,
}

/// One of the six tool-call wire formats Argo speaks.
///
/// Spec: `docs/plan/argo-build-plan.md` §8.5 (named in Phase 2 plan).
///
/// ```
/// use argo_provider::ToolCallFormat;
/// let f: ToolCallFormat = serde_json::from_str("\"openai_tool_calls\"").unwrap();
/// assert!(matches!(f, ToolCallFormat::OpenaiToolCalls));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallFormat {
    /// OpenAI Chat Completions `tool_calls` array.
    OpenaiToolCalls,
    /// OpenAI Responses API tool blocks.
    OpenaiResponseTools,
    /// Anthropic content-block `type = "tool_use"`.
    AnthropicToolUse,
    /// AWS Bedrock InvokeModel tool-use envelope.
    BedrockToolUse,
    /// Hermes-style `<tool_call>{...}</tool_call>` XML in a text body.
    XmlToolCall,
    /// LLM-extracted plain-text format requiring a second-pass parse.
    LlmExtracted,
}

/// `[[models]]` array entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Provider-side model id (e.g. `"gpt-4o-mini"`).
    pub id: String,
    /// Context window in tokens, if known.
    #[serde(default)]
    pub context_window: Option<u32>,
    /// USD per 1k input tokens (optional, used by cost router — Agent V).
    #[serde(default)]
    pub price_per_1k_in: Option<f64>,
    /// USD per 1k output tokens.
    #[serde(default)]
    pub price_per_1k_out: Option<f64>,
    /// Free-form tier label (e.g. `"simple"`, `"complex"`).
    #[serde(default)]
    pub tier: Option<String>,
}

/// `[discovery]` table — `model = "latest"` resolution (Amendment 8 / §8.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySpec {
    /// URL template; `{base_url}` is substituted at runtime.
    pub endpoint: String,
    /// Regex for model ids that should be considered.
    pub include_pattern: String,
    /// Regex for model ids that should be excluded (e.g. `preview|deprecated`).
    #[serde(default)]
    pub exclude_pattern: Option<String>,
    /// Lexicographic preference rule.
    pub prefer: PreferRule,
    /// Cache TTL.
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_hours: u32,
}

fn default_refresh_interval() -> u32 {
    24
}

/// Discovery preference rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferRule {
    /// Prefer the highest minor version that matches `include_pattern`.
    HighestMinor,
    /// Prefer the lexicographically greatest match.
    LexicographicMax,
}

/// `[reasoning]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningSpec {
    /// Field on the response carrying reasoning content (e.g. `"thinking"`).
    pub content_field: String,
    /// Whether to round-trip the reasoning across turns.
    #[serde(default)]
    pub preserve_across_turns: bool,
}

/// `[hooks]` table — name-resolved hook references. Both fields default
/// to `"default"`, meaning "use the family transport's built-in code path".
///
/// ```
/// use argo_provider::HooksSpec;
/// let h: HooksSpec = toml::from_str(r#"
/// build_request = "anthropic-thinking-shim"
/// "#).unwrap();
/// assert_eq!(h.build_request, "anthropic-thinking-shim");
/// assert_eq!(h.parse_response, "default");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksSpec {
    /// Hook name applied before request body construction.
    #[serde(default = "default_hook")]
    pub build_request: String,
    /// Hook name applied after response parsing.
    #[serde(default = "default_hook")]
    pub parse_response: String,
}

fn default_hook() -> String {
    "default".to_string()
}

impl Default for HooksSpec {
    fn default() -> Self {
        Self {
            build_request: default_hook(),
            parse_response: default_hook(),
        }
    }
}

/// `[capabilities]` table — declares net-egress capabilities the transport
/// requires when invoked. Loaded into a `CapabilitySet` for §10 enforcement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitiesSpec {
    /// Host globs the transport may dial.
    #[serde(default)]
    pub net_egress: Vec<String>,
    /// Env vars the transport may read (in addition to `auth.api_key_env`).
    #[serde(default)]
    pub env_read: Vec<String>,
}

impl CapabilitiesSpec {
    /// Materialize as a `CapabilitySet` covering this transport's needs.
    #[must_use]
    pub fn to_set(&self, also_read_env: &str) -> CapabilitySet {
        let mut set = CapabilitySet::empty();
        for host in &self.net_egress {
            set.push(Capability::NetEgress {
                host_glob: host.clone(),
                port: None,
            });
        }
        set.push(Capability::EnvRead {
            var_glob: also_read_env.to_string(),
        });
        for v in &self.env_read {
            set.push(Capability::EnvRead {
                var_glob: v.clone(),
            });
        }
        set
    }
}

impl ProviderManifest {
    /// Parse a manifest from a TOML string.
    ///
    /// ```
    /// use argo_provider::ProviderManifest;
    /// let toml = include_str!("../../../../manifests/providers/openai-compatible.toml");
    /// let m = ProviderManifest::from_toml_str(toml).unwrap();
    /// assert_eq!(m.provider.id, "openai-compatible");
    /// ```
    pub fn from_toml_str(s: &str) -> ArgoResult<Self> {
        let m: Self =
            toml::from_str(s).map_err(|e| ArgoError::Schema(format!("manifest parse: {e}")))?;
        m.validate()?;
        Ok(m)
    }

    /// Validate cross-field invariants (§8.1).
    pub fn validate(&self) -> ArgoResult<()> {
        if self.provider.id.is_empty() {
            return Err(ArgoError::Schema("provider.id is empty".into()));
        }
        if !self.provider.base_url.starts_with("http") {
            return Err(ArgoError::Schema(format!(
                "provider.base_url must start with http(s); got {}",
                self.provider.base_url
            )));
        }
        if self.provider.base_url.ends_with('/') {
            return Err(ArgoError::Schema(
                "provider.base_url must not end with '/'".into(),
            ));
        }
        if !self.auth.header_format.contains("{key}") {
            return Err(ArgoError::Schema(
                "auth.header_format must contain `{key}`".into(),
            ));
        }
        if self.transport.max_backoff_ms < self.transport.initial_backoff_ms {
            return Err(ArgoError::Schema(
                "transport.max_backoff_ms < initial_backoff_ms".into(),
            ));
        }
        if matches!(self.transport.streaming, StreamingMode::Sse)
            && self.transport.heartbeat_event.is_some()
            && self.transport.heartbeat_grace_ms.is_none()
        {
            return Err(ArgoError::Schema(
                "transport.heartbeat_event set without heartbeat_grace_ms".into(),
            ));
        }
        Ok(())
    }

    /// Resolve a model id, treating `"latest"` as a discovery sentinel (§8.2).
    /// Returns the configured id when discovery is absent or the sentinel
    /// is not used.
    #[must_use]
    pub fn resolve_model_id<'a>(&'a self, requested: &'a str) -> &'a str {
        if requested != "latest" {
            return requested;
        }
        match self.models.first() {
            Some(m) => &m.id,
            None => requested,
        }
    }
}

#[cfg(test)]
mod tests;
