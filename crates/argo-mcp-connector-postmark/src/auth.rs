//! Authentication helpers for the Postmark connector.

/// Environment variable carrying the Postmark server token.
pub const POSTMARK_SERVER_TOKEN_ENV: &str = "POSTMARK_SERVER_TOKEN";
/// Environment variable overriding the Postmark API base URL for tests.
pub const POSTMARK_BASE_URL_ENV: &str = "POSTMARK_BASE_URL";
/// Default Postmark API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.postmarkapp.com";

/// Read the Postmark server token from `POSTMARK_SERVER_TOKEN`.
pub fn server_token_from_env() -> Result<String, String> {
    let token = argo_config::cred::get(POSTMARK_SERVER_TOKEN_ENV)
        .map_err(|_| format!("missing {POSTMARK_SERVER_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {POSTMARK_SERVER_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(POSTMARK_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_server_token_is_clear() {
        std::env::remove_var(POSTMARK_SERVER_TOKEN_ENV);
        let err = server_token_from_env().unwrap_err();
        assert!(err.contains(POSTMARK_SERVER_TOKEN_ENV));
    }
}
