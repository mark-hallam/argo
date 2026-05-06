//! Authentication helpers for the Blue Bubbles connector.

/// Blue Bubbles server URL environment variable.
pub const BLUE_BUBBLES_URL_ENV: &str = "BLUE_BUBBLES_URL";

/// Blue Bubbles server password environment variable.
pub const BLUE_BUBBLES_PASSWORD_ENV: &str = "BLUE_BUBBLES_PASSWORD";

/// Read `BLUE_BUBBLES_URL`.
pub fn url_from_env() -> Result<String, String> {
    required_env(BLUE_BUBBLES_URL_ENV)
}

/// Read `BLUE_BUBBLES_PASSWORD`.
pub fn password_from_env() -> Result<String, String> {
    required_env(BLUE_BUBBLES_PASSWORD_ENV)
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("missing {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_password_is_clear() {
        std::env::remove_var(BLUE_BUBBLES_PASSWORD_ENV);
        let err = password_from_env().unwrap_err();
        assert!(err.contains(BLUE_BUBBLES_PASSWORD_ENV));
    }
}
