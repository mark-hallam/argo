//! Authentication helpers for the Airtable connector.

/// Environment variable carrying the Airtable personal access token.
pub const AIRTABLE_PAT_ENV: &str = "AIRTABLE_PAT";

/// Environment variable overriding the Airtable API base URL for tests.
pub const AIRTABLE_BASE_URL_ENV: &str = "AIRTABLE_BASE_URL";

/// Default Airtable API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.airtable.com/v0";

/// Read the Airtable personal access token from `AIRTABLE_PAT`.
pub fn pat_from_env() -> Result<String, String> {
    let pat = argo_config::cred::get(AIRTABLE_PAT_ENV)
        .map_err(|_| format!("missing {AIRTABLE_PAT_ENV}"))?;
    if pat.trim().is_empty() {
        return Err(format!("missing {AIRTABLE_PAT_ENV}"));
    }
    Ok(pat)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(AIRTABLE_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_pat_is_clear() {
        std::env::remove_var(AIRTABLE_PAT_ENV);
        let err = pat_from_env().unwrap_err();
        assert!(err.contains(AIRTABLE_PAT_ENV));
    }
}
