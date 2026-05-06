//! Authentication helpers for the Asana connector.

/// Environment variable carrying the Asana personal access token.
pub const ASANA_PAT_ENV: &str = "ASANA_PAT";

/// Environment variable overriding the Asana API base URL for tests.
pub const ASANA_BASE_URL_ENV: &str = "ASANA_BASE_URL";

/// Default Asana API base URL.
pub const DEFAULT_BASE_URL: &str = "https://app.asana.com/api/1.0";

/// Read the Asana personal access token from `ASANA_PAT`.
pub fn pat_from_env() -> Result<String, String> {
    let pat = std::env::var(ASANA_PAT_ENV).map_err(|_| format!("missing {ASANA_PAT_ENV}"))?;
    if pat.trim().is_empty() {
        return Err(format!("missing {ASANA_PAT_ENV}"));
    }
    Ok(pat)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(ASANA_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_pat_is_clear() {
        std::env::remove_var(ASANA_PAT_ENV);
        let err = pat_from_env().unwrap_err();
        assert!(err.contains(ASANA_PAT_ENV));
    }
}
