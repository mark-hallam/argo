//! Environment-driven SendGrid authentication.

/// SendGrid API key env var.
pub const SENDGRID_API_KEY_ENV: &str = "SENDGRID_API_KEY";

/// SendGrid REST API base URL.
pub const SENDGRID_BASE_URL: &str = "https://api.sendgrid.com/v3";

/// SendGrid REST API configuration.
#[derive(Debug, Clone)]
pub struct SendgridConfig {
    /// SendGrid API key.
    pub api_key: String,
    /// SendGrid REST API base URL.
    pub base_url: String,
}

impl SendgridConfig {
    /// Build config from `SENDGRID_API_KEY`.
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            api_key: required_env(SENDGRID_API_KEY_ENV)?,
            base_url: SENDGRID_BASE_URL.into(),
        })
    }
}

fn required_env(name: &str) -> Result<String, String> {
    let value = argo_config::cred::get(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        return Err(format!("missing {name}"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_mentions_env_name() {
        std::env::remove_var(SENDGRID_API_KEY_ENV);
        let err = SendgridConfig::from_env().unwrap_err();
        assert!(err.contains(SENDGRID_API_KEY_ENV));
    }
}
