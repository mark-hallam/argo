//! Authentication helpers for the ClickUp connector.

/// Environment variable carrying the ClickUp personal token.
pub const CLICKUP_TOKEN_ENV: &str = "CLICKUP_TOKEN";

/// Environment variable overriding the ClickUp API base URL for tests.
pub const CLICKUP_BASE_URL_ENV: &str = "CLICKUP_BASE_URL";

/// Default ClickUp API v2 base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.clickup.com/api/v2";

/// Read the ClickUp personal token from `CLICKUP_TOKEN`.
pub fn token_from_env() -> Result<String, String> {
    let token =
        std::env::var(CLICKUP_TOKEN_ENV).map_err(|_| format!("missing {CLICKUP_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {CLICKUP_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(CLICKUP_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_is_clear() {
        std::env::remove_var(CLICKUP_TOKEN_ENV);
        let err = token_from_env().unwrap_err();
        assert!(err.contains(CLICKUP_TOKEN_ENV));
    }
}
