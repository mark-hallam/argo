//! Environment-driven Zendesk Support authentication.

/// Zendesk account email env var.
pub const ZENDESK_EMAIL_ENV: &str = "ZENDESK_EMAIL";
/// Zendesk API token env var.
pub const ZENDESK_API_TOKEN_ENV: &str = "ZENDESK_API_TOKEN";
/// Zendesk subdomain env var, e.g. `acme` for `acme.zendesk.com`.
pub const ZENDESK_SUBDOMAIN_ENV: &str = "ZENDESK_SUBDOMAIN";

/// Zendesk API-token auth configuration.
#[derive(Debug, Clone)]
pub struct ZendeskConfig {
    /// Zendesk account email.
    pub email: String,
    /// Zendesk API token.
    pub api_token: String,
    /// Zendesk REST API base URL.
    pub base_url: String,
}

impl ZendeskConfig {
    /// Build config from `ZENDESK_EMAIL`, `ZENDESK_API_TOKEN`, and `ZENDESK_SUBDOMAIN`.
    pub fn from_env() -> Result<Self, String> {
        let email = required_env(ZENDESK_EMAIL_ENV)?;
        let api_token = required_env(ZENDESK_API_TOKEN_ENV)?;
        let subdomain = clean_subdomain(&required_env(ZENDESK_SUBDOMAIN_ENV)?)?;
        Ok(Self {
            email,
            api_token,
            base_url: format!("https://{subdomain}.zendesk.com/api/v2"),
        })
    }
}

fn required_env(name: &str) -> Result<String, String> {
    let value = argo_config::cred::get(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        return Err(format!("missing {name}"));
    }
    Ok(value)
}

fn clean_subdomain(raw: &str) -> Result<String, String> {
    let value = raw.trim().trim_end_matches('/');
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-'))
        && !value.starts_with('-')
        && !value.ends_with('-')
    {
        Ok(value.to_string())
    } else {
        Err(format!("unsafe Zendesk subdomain: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_is_clear() {
        std::env::remove_var(ZENDESK_EMAIL_ENV);
        let err = ZendeskConfig::from_env().unwrap_err();
        assert!(err.contains(ZENDESK_EMAIL_ENV));
    }

    #[test]
    fn rejects_unsafe_subdomains() {
        let err = clean_subdomain("https://acme.zendesk.com").unwrap_err();
        assert!(err.contains("unsafe Zendesk subdomain"));
    }
}
