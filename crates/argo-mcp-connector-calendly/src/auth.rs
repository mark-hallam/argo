//! Authentication helpers for the Calendly connector.

/// Environment variable carrying the Calendly personal access token.
pub const CALENDLY_API_KEY_ENV: &str = "CALENDLY_API_KEY";

/// Environment variable overriding the Calendly API base URL for tests.
pub const CALENDLY_BASE_URL_ENV: &str = "CALENDLY_BASE_URL";

/// Default Calendly API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.calendly.com";

/// Read the Calendly personal access token from `CALENDLY_API_KEY`.
pub fn api_key_from_env() -> Result<String, String> {
    let token = argo_config::cred::get(CALENDLY_API_KEY_ENV)
        .map_err(|_| format!("missing {CALENDLY_API_KEY_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {CALENDLY_API_KEY_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(CALENDLY_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_is_clear() {
        std::env::remove_var(CALENDLY_API_KEY_ENV);
        let err = api_key_from_env().unwrap_err();
        assert!(err.contains(CALENDLY_API_KEY_ENV));
    }
}
