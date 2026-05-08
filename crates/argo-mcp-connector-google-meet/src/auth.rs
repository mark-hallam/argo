//! Authentication helpers for the Google Meet connector.

/// Environment variable carrying a Google OAuth bearer token.
pub const GOOGLE_OAUTH_TOKEN_ENV: &str = "GOOGLE_OAUTH_TOKEN";

/// Environment variable overriding the Google Meet API base URL for tests.
pub const GOOGLE_MEET_BASE_URL_ENV: &str = "GOOGLE_MEET_BASE_URL";

/// Default Google Meet API v2 base URL.
pub const DEFAULT_BASE_URL: &str = "https://meet.googleapis.com/v2";

/// Read the Google OAuth bearer token from `GOOGLE_OAUTH_TOKEN`.
pub fn token_from_env() -> Result<String, String> {
    let token = argo_config::cred::get(GOOGLE_OAUTH_TOKEN_ENV)
        .map_err(|_| format!("missing {GOOGLE_OAUTH_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {GOOGLE_OAUTH_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(GOOGLE_MEET_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_is_clear() {
        std::env::remove_var(GOOGLE_OAUTH_TOKEN_ENV);
        let err = token_from_env().unwrap_err();
        assert!(err.contains(GOOGLE_OAUTH_TOKEN_ENV));
    }
}
