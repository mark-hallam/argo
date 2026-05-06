//! Authentication helpers for the Monday.com connector.

/// Environment variable carrying the Monday.com API token.
pub const MONDAY_API_TOKEN_ENV: &str = "MONDAY_API_TOKEN";

/// Environment variable overriding the Monday.com GraphQL endpoint for tests.
pub const MONDAY_BASE_URL_ENV: &str = "MONDAY_BASE_URL";

/// Default Monday.com GraphQL API endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.monday.com/v2";

/// Read the Monday.com API token from `MONDAY_API_TOKEN`.
pub fn token_from_env() -> Result<String, String> {
    let token = std::env::var(MONDAY_API_TOKEN_ENV)
        .map_err(|_| format!("missing {MONDAY_API_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {MONDAY_API_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(MONDAY_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_is_clear() {
        std::env::remove_var(MONDAY_API_TOKEN_ENV);
        let err = token_from_env().unwrap_err();
        assert!(err.contains(MONDAY_API_TOKEN_ENV));
    }
}
