//! OAuth helpers for Google Calendar.
use argo_mcp::{McpError, McpResult};
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};

/// Google Calendar OAuth scopes required by the connector tools.
pub const GCAL_OAUTH_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/calendar.events",
    "https://www.googleapis.com/auth/calendar.freebusy",
];

/// OAuth client settings for the Google Calendar connector.
#[derive(Debug, Clone)]
pub struct GoogleOAuthConfig {
    /// OAuth client id.
    pub client_id: String,
    /// OAuth client secret.
    pub client_secret: String,
    /// Redirect URL registered in Google Cloud Console.
    pub redirect_url: String,
}

impl GoogleOAuthConfig {
    /// Build an authorization URL with the connector scopes.
    pub fn authorization_url(&self) -> McpResult<String> {
        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(parse_auth_url(
                "https://accounts.google.com/o/oauth2/v2/auth",
            )?)
            .set_token_uri(parse_token_url("https://oauth2.googleapis.com/token")?)
            .set_redirect_uri(parse_redirect_url(&self.redirect_url)?);
        let (url, _csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(GCAL_OAUTH_SCOPES.iter().map(|s| Scope::new((*s).into())))
            .url();
        Ok(url.to_string())
    }
}

fn parse_auth_url(url: &str) -> McpResult<AuthUrl> {
    AuthUrl::new(url.into()).map_err(|e| McpError::Initialize(format!("auth url: {e}")))
}

fn parse_token_url(url: &str) -> McpResult<TokenUrl> {
    TokenUrl::new(url.into()).map_err(|e| McpError::Initialize(format!("token url: {e}")))
}

fn parse_redirect_url(url: &str) -> McpResult<RedirectUrl> {
    RedirectUrl::new(url.into()).map_err(|e| McpError::Initialize(format!("redirect url: {e}")))
}
