//! Authentication helpers for the GitHub Issues connector.

/// GitHub personal access token env var.
pub const GITHUB_TOKEN_ENV: &str = "GITHUB_TOKEN";
/// Optional GitHub API base URL override for tests or enterprise hosts.
pub const GITHUB_API_BASE_URL_ENV: &str = "GITHUB_API_BASE_URL";
/// Default GitHub REST API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.github.com";

/// Read the GitHub personal access token.
pub fn token_from_env() -> Result<String, String> {
    let token = argo_config::cred::get(GITHUB_TOKEN_ENV)
        .map_err(|_| format!("missing {GITHUB_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {GITHUB_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the API base URL.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(GITHUB_API_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_is_clear() {
        std::env::remove_var(GITHUB_TOKEN_ENV);
        let err = token_from_env().unwrap_err();
        assert!(err.contains(GITHUB_TOKEN_ENV));
    }
}
