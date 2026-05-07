//! Authentication helpers for the PagerDuty connector.

/// Environment variable carrying the PagerDuty REST API key.
pub const PAGERDUTY_API_KEY_ENV: &str = "PAGERDUTY_API_KEY";

/// Environment variable overriding the PagerDuty API base URL for tests.
pub const PAGERDUTY_BASE_URL_ENV: &str = "PAGERDUTY_BASE_URL";

/// Optional default `From` email for PagerDuty write requests.
pub const PAGERDUTY_FROM_EMAIL_ENV: &str = "PAGERDUTY_FROM_EMAIL";

/// Default PagerDuty REST API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.pagerduty.com";

/// Read the PagerDuty API key from `PAGERDUTY_API_KEY`.
pub fn api_key_from_env() -> Result<String, String> {
    let key = argo_config::cred::get(PAGERDUTY_API_KEY_ENV)
        .map_err(|_| format!("missing {PAGERDUTY_API_KEY_ENV}"))?;
    if key.trim().is_empty() {
        return Err(format!("missing {PAGERDUTY_API_KEY_ENV}"));
    }
    Ok(key)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(PAGERDUTY_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

/// Read the optional default `From` email for write requests.
#[must_use]
pub fn from_email_from_env() -> Option<String> {
    argo_config::cred::get(PAGERDUTY_FROM_EMAIL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_is_clear() {
        std::env::remove_var(PAGERDUTY_API_KEY_ENV);
        let err = api_key_from_env().unwrap_err();
        assert!(err.contains(PAGERDUTY_API_KEY_ENV));
    }
}
