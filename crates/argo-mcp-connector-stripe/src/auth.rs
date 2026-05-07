//! Authentication helpers for the Stripe connector.

/// Environment variable carrying the Stripe secret API key.
pub const STRIPE_API_KEY_ENV: &str = "STRIPE_API_KEY";

/// Environment variable overriding the Stripe API base URL for tests.
pub const STRIPE_BASE_URL_ENV: &str = "STRIPE_BASE_URL";

/// Default Stripe API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.stripe.com";

/// Read the Stripe API key from `STRIPE_API_KEY`.
pub fn api_key_from_env() -> Result<String, String> {
    let key = argo_config::cred::get(STRIPE_API_KEY_ENV)
        .map_err(|_| format!("missing {STRIPE_API_KEY_ENV}"))?;
    if key.trim().is_empty() {
        return Err(format!("missing {STRIPE_API_KEY_ENV}"));
    }
    Ok(key)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(STRIPE_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_is_clear() {
        std::env::remove_var(STRIPE_API_KEY_ENV);
        let err = api_key_from_env().unwrap_err();
        assert!(err.contains(STRIPE_API_KEY_ENV));
    }
}
