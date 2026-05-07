//! Environment-driven Trello API authentication.

/// Trello API key env var.
pub const TRELLO_KEY_ENV: &str = "TRELLO_KEY";
/// Trello API token env var.
pub const TRELLO_TOKEN_ENV: &str = "TRELLO_TOKEN";
/// Trello REST API base URL.
pub const TRELLO_BASE_URL: &str = "https://api.trello.com/1";

/// Trello API key/token pair.
#[derive(Clone)]
pub struct TrelloAuth {
    key: String,
    token: String,
}

impl TrelloAuth {
    /// Build auth from explicit key and token strings.
    pub fn new(key: String, token: String) -> Result<Self, String> {
        if key.trim().is_empty() {
            return Err(format!("missing {TRELLO_KEY_ENV}"));
        }
        if token.trim().is_empty() {
            return Err(format!("missing {TRELLO_TOKEN_ENV}"));
        }
        Ok(Self { key, token })
    }

    /// Append Trello query-param auth to a request query vector.
    pub fn with_auth(&self, mut query: Vec<(String, String)>) -> Vec<(String, String)> {
        query.push(("key".into(), self.key.clone()));
        query.push(("token".into(), self.token.clone()));
        query
    }
}

/// Trello connector configuration.
pub struct TrelloConfig {
    /// API key/token auth.
    pub auth: TrelloAuth,
    /// REST API base URL.
    pub base_url: String,
}

impl TrelloConfig {
    /// Build config from `TRELLO_KEY` and `TRELLO_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            auth: TrelloAuth::new(
                required_env(TRELLO_KEY_ENV)?,
                required_env(TRELLO_TOKEN_ENV)?,
            )?,
            base_url: TRELLO_BASE_URL.into(),
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
    fn with_auth_appends_key_and_token_query_params() {
        let auth = TrelloAuth::new("k".into(), "t".into()).unwrap();
        assert_eq!(
            auth.with_auth(vec![("filter".into(), "open".into())]),
            vec![
                ("filter".into(), "open".into()),
                ("key".into(), "k".into()),
                ("token".into(), "t".into()),
            ]
        );
    }

    #[test]
    fn rejects_empty_secret_values() {
        let err = match TrelloAuth::new("".into(), "tok".into()) {
            Ok(_) => panic!("empty key should fail"),
            Err(err) => err,
        };
        assert!(err.contains(TRELLO_KEY_ENV));
    }
}
