//! Jira Cloud authentication config.

const JIRA_EMAIL_ENV: &str = "JIRA_EMAIL";
const JIRA_API_TOKEN_ENV: &str = "JIRA_API_TOKEN";
const JIRA_BASE_URL_ENV: &str = "JIRA_BASE_URL";

/// Jira Cloud authentication and endpoint configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JiraConfig {
    /// Atlassian account email used for Basic auth.
    pub email: String,
    /// Jira API token used for Basic auth.
    pub api_token: String,
    /// Jira Cloud base URL, without a trailing slash.
    pub base_url: String,
}

impl JiraConfig {
    /// Load Jira config from `JIRA_EMAIL`, `JIRA_API_TOKEN`, and `JIRA_BASE_URL`.
    ///
    /// ```no_run
    /// use argo_mcp_connector_jira::JiraConfig;
    /// let _cfg = JiraConfig::from_env();
    /// ```
    pub fn from_env() -> Result<Self, String> {
        Self::new(
            required_env(JIRA_EMAIL_ENV)?,
            required_env(JIRA_API_TOKEN_ENV)?,
            required_env(JIRA_BASE_URL_ENV)?,
        )
    }

    /// Build validated Jira config from explicit values.
    ///
    /// ```
    /// use argo_mcp_connector_jira::JiraConfig;
    /// let cfg = JiraConfig::new("op@example.com", "token", "https://example.atlassian.net").unwrap();
    /// assert_eq!(cfg.base_url, "https://example.atlassian.net");
    /// ```
    pub fn new(
        email: impl Into<String>,
        api_token: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, String> {
        let email = non_empty("JIRA_EMAIL", email.into())?;
        let api_token = non_empty("JIRA_API_TOKEN", api_token.into())?;
        let base_url = non_empty("JIRA_BASE_URL", base_url.into())?
            .trim_end_matches('/')
            .to_string();
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err("JIRA_BASE_URL must start with http:// or https://".into());
        }
        Ok(Self {
            email,
            api_token,
            base_url,
        })
    }
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("missing {name}"))
}

fn non_empty(name: &str, value: String) -> Result<String, String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Err(format!("missing {name}"))
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_trailing_slash() {
        let cfg = JiraConfig::new("op@example.com", "tok", "https://acme.atlassian.net/")
            .expect("config");
        assert_eq!(cfg.base_url, "https://acme.atlassian.net");
    }

    #[test]
    fn rejects_empty_token() {
        let err = JiraConfig::new("op@example.com", " ", "https://acme.atlassian.net")
            .expect_err("blank token should fail");
        assert!(err.contains("JIRA_API_TOKEN"));
    }
}
