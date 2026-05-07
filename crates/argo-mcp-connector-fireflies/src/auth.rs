//! Authentication helpers for the Fireflies.ai connector.

/// Environment variable carrying the Fireflies.ai API key.
pub const FIREFLIES_API_KEY_ENV: &str = "FIREFLIES_API_KEY";

/// Environment variable overriding the Fireflies.ai GraphQL endpoint for tests.
pub const FIREFLIES_BASE_URL_ENV: &str = "FIREFLIES_BASE_URL";

/// Default Fireflies.ai GraphQL endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.fireflies.ai/graphql";

/// Read the Fireflies.ai API key from `FIREFLIES_API_KEY`.
pub fn api_key_from_env() -> Result<String, String> {
    let key = argo_config::cred::get(FIREFLIES_API_KEY_ENV)
        .map_err(|_| format!("missing {FIREFLIES_API_KEY_ENV}"))?;
    if key.trim().is_empty() {
        return Err(format!("missing {FIREFLIES_API_KEY_ENV}"));
    }
    Ok(key)
}

/// Read the optional GraphQL endpoint override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(FIREFLIES_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_is_clear() {
        std::env::remove_var(FIREFLIES_API_KEY_ENV);
        let err = api_key_from_env().unwrap_err();
        assert!(err.contains(FIREFLIES_API_KEY_ENV));
    }
}
