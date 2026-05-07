//! Authentication helpers for the Discord connector.

/// Environment variable carrying the Discord bot token.
pub const DISCORD_BOT_TOKEN_ENV: &str = "DISCORD_BOT_TOKEN";
/// Environment variable overriding the Discord API base URL for tests.
pub const DISCORD_BASE_URL_ENV: &str = "DISCORD_BASE_URL";
/// Default Discord API base URL.
pub const DEFAULT_BASE_URL: &str = "https://discord.com/api/v10";

/// Read the Discord bot token from `DISCORD_BOT_TOKEN`.
pub fn bot_token_from_env() -> Result<String, String> {
    let token = argo_config::cred::get(DISCORD_BOT_TOKEN_ENV)
        .map_err(|_| format!("missing {DISCORD_BOT_TOKEN_ENV}"))?;
    if token.trim().is_empty() {
        return Err(format!("missing {DISCORD_BOT_TOKEN_ENV}"));
    }
    Ok(token)
}

/// Read the optional test base URL override.
#[must_use]
pub fn base_url_from_env() -> String {
    std::env::var(DISCORD_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_bot_token_is_clear() {
        std::env::remove_var(DISCORD_BOT_TOKEN_ENV);
        let err = bot_token_from_env().unwrap_err();
        assert!(err.contains(DISCORD_BOT_TOKEN_ENV));
    }
}
