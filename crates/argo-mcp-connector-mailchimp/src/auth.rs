//! Authentication helpers for the Mailchimp connector.

/// Environment variable carrying the Mailchimp API key.
pub const MAILCHIMP_API_KEY_ENV: &str = "MAILCHIMP_API_KEY";

/// Environment variable carrying the Mailchimp datacenter, such as `us19`.
pub const MAILCHIMP_DC_ENV: &str = "MAILCHIMP_DC";

/// Mailchimp API configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailchimpConfig {
    /// API key used as the Basic auth password.
    pub api_key: String,
    /// Region-derived API base URL.
    pub base_url: String,
}

/// Read Mailchimp API key and datacenter from the environment.
pub fn config_from_env() -> Result<MailchimpConfig, String> {
    let api_key = required(MAILCHIMP_API_KEY_ENV)?;
    let dc = required(MAILCHIMP_DC_ENV)?;
    Ok(MailchimpConfig {
        api_key,
        base_url: base_url_from_dc(&dc)?,
    })
}

/// Compose the Mailchimp API base URL from a datacenter identifier.
pub fn base_url_from_dc(dc: &str) -> Result<String, String> {
    let trimmed = dc.trim().trim_matches('.');
    if trimmed.is_empty() || trimmed.contains('/') {
        return Err(format!("invalid {MAILCHIMP_DC_ENV}"));
    }
    Ok(format!("https://{trimmed}.api.mailchimp.com/3.0"))
}

fn required(name: &str) -> Result<String, String> {
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
    fn base_url_uses_datacenter_region() {
        assert_eq!(
            base_url_from_dc("us19").unwrap(),
            "https://us19.api.mailchimp.com/3.0"
        );
    }

    #[test]
    fn missing_api_key_is_clear() {
        std::env::remove_var(MAILCHIMP_API_KEY_ENV);
        let err = config_from_env().unwrap_err();
        assert!(err.contains(MAILCHIMP_API_KEY_ENV));
    }
}
