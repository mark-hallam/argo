//! Authentication helpers for the Figma connector.

/// Environment variable carrying the Figma personal access token.
pub const FIGMA_TOKEN_ENV: &str = "FIGMA_TOKEN";

/// Environment variable overriding the Figma API base URL for tests.
pub const FIGMA_BASE_URL_ENV: &str = "FIGMA_BASE_URL";

/// Default Figma REST API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.figma.com/v1";

/// Read the Figma personal access token from `FIGMA_TOKEN`.
pub fn token_from_env() -> Result<String, String> {
    let token = argo_config::cred::get(FIGMA_TOKEN_ENV)
        .map_err(|_| format!("missing {FIGMA_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {FIGMA_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(FIGMA_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_is_clear() {
        std::env::remove_var(FIGMA_TOKEN_ENV);
        let err = token_from_env().unwrap_err();
        assert!(err.contains(FIGMA_TOKEN_ENV));
    }
}
