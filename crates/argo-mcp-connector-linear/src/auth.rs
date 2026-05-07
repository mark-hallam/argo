//! Authentication helpers for the Linear connector.

/// Environment variable carrying the Linear API key.
pub const LINEAR_API_KEY_ENV: &str = "LINEAR_API_KEY";

/// Environment variable overriding the Linear GraphQL base URL for tests.
pub const LINEAR_BASE_URL_ENV: &str = "LINEAR_BASE_URL";

/// Default Linear GraphQL endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.linear.app/graphql";

/// Read the Linear API key from `LINEAR_API_KEY`.
pub fn api_key_from_env() -> Result<String, String> {
    let key = argo_config::cred::get(LINEAR_API_KEY_ENV)
        .map_err(|_| format!("missing {LINEAR_API_KEY_ENV}"))?;
    if key.trim().is_empty() {
        return Err(format!("missing {LINEAR_API_KEY_ENV}"));
    }
    Ok(key)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(LINEAR_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_is_clear() {
        std::env::remove_var(LINEAR_API_KEY_ENV);
        let err = api_key_from_env().unwrap_err();
        assert!(err.contains(LINEAR_API_KEY_ENV));
    }
}
