//! Environment-driven Typeform authentication.

/// Typeform personal access token env var.
pub const TYPEFORM_TOKEN_ENV: &str = "TYPEFORM_TOKEN";
/// Typeform REST API base URL.
pub const TYPEFORM_BASE_URL: &str = "https://api.typeform.com";

/// Typeform REST API configuration.
#[derive(Debug, Clone)]
pub struct TypeformConfig {
    /// Typeform personal access token.
    pub token: String,
    /// Typeform REST API base URL.
    pub base_url: String,
}

impl TypeformConfig {
    /// Build config from `TYPEFORM_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            token: required_env(TYPEFORM_TOKEN_ENV)?,
            base_url: TYPEFORM_BASE_URL.into(),
        })
    }
}

fn required_env(name: &str) -> Result<String, String> {
    // tenant-unscoped: MCP connector auth init pre-dates per-tenant scoping; follow-up: thread tenant_id through McpClientPool keying
    let value = argo_config::cred::get_unscoped(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        return Err(format!("missing {name}"));
    }
    Ok(value)
}
