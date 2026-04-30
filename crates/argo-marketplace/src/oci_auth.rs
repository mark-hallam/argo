//! OCI Distribution Spec auth-challenge helpers.
//!
//! When a registry replies `401 Unauthorized` to a v2 request,
//! it includes a `WWW-Authenticate: Bearer realm="…",service="…"
//! ,scope="…"` header. The client is expected to:
//!
//! 1. Parse `realm`, `service`, `scope` out of the header.
//! 2. `GET <realm>?service=…&scope=…` with HTTP Basic auth (the
//!    user's registry credentials — for GHCR a Personal Access
//!    Token, for Docker Hub username + access token).
//! 3. Receive `{"token": "…"}`.
//! 4. Retry the original request with `Authorization: Bearer
//!    <token>`.
//!
//! This module owns steps 1–3; the `OciDistributionRegistry`
//! does the retry. Pure functions where possible so the parser
//! is unit-testable without a network.
use argo_types::{ArgoError, ArgoResult};

/// Parsed `WWW-Authenticate: Bearer …` parameters. Only the
/// fields Argo needs; extra params (e.g. `error_description`)
/// are ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerChallenge {
    /// Token endpoint URL — caller GETs this with `service` +
    /// `scope` query params + HTTP Basic auth.
    pub realm: String,
    /// Registry service name (e.g. `ghcr.io`,
    /// `registry.docker.io`).
    pub service: Option<String>,
    /// Permission scope the token should grant (e.g.
    /// `repository:foo/bar:pull,push`).
    pub scope: Option<String>,
}

impl BearerChallenge {
    /// Parse a `WWW-Authenticate` header value. Returns `None`
    /// when the value isn't a `Bearer …` challenge.
    #[must_use]
    pub fn parse(header_value: &str) -> Option<Self> {
        let trimmed = header_value.trim();
        let rest = trimmed.strip_prefix("Bearer ")?.trim();
        let mut realm = None;
        let mut service = None;
        let mut scope = None;
        for kv in split_params(rest) {
            let (k, v) = kv;
            match k {
                "realm" => realm = Some(v.to_string()),
                "service" => service = Some(v.to_string()),
                "scope" => scope = Some(v.to_string()),
                _ => {}
            }
        }
        let realm = realm?;
        Some(Self {
            realm,
            service,
            scope,
        })
    }

    /// Build the token-endpoint URL: `<realm>?service=…&scope=…`.
    #[must_use]
    pub fn token_url(&self) -> String {
        let mut url = self.realm.clone();
        let mut params: Vec<String> = Vec::new();
        if let Some(s) = self.service.as_deref() {
            params.push(format!("service={s}"));
        }
        if let Some(s) = self.scope.as_deref() {
            params.push(format!("scope={s}"));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        url
    }
}

/// Token-endpoint response shape. Some registries return
/// `access_token` instead of `token`; we accept either.
#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    token: Option<String>,
    access_token: Option<String>,
}

/// Fetch a bearer token from the challenge's `realm` using HTTP
/// Basic auth. Returns the raw token string ready to pass into
/// `Authorization: Bearer <token>`.
pub async fn fetch_bearer_token(
    client: &reqwest::Client,
    challenge: &BearerChallenge,
    user: &str,
    password: &str,
) -> ArgoResult<String> {
    let url = challenge.token_url();
    let resp = client
        .get(&url)
        .basic_auth(user, Some(password))
        .send()
        .await
        .map_err(|e| ArgoError::Provider(format!("oci token GET {url}: {e}")))?;
    if !matches!(resp.status().as_u16(), 200..=299) {
        return Err(ArgoError::Provider(format!(
            "oci token GET {url}: {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    let body: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ArgoError::Provider(format!("oci token parse: {e}")))?;
    body.token
        .or(body.access_token)
        .ok_or_else(|| ArgoError::Provider("oci token response missing `token`".into()))
}

/// Split `key1="v1",key2="v2",key3=v3` into `[(key, value), …]`.
/// Quoted values keep escaping minimal — backslash-escapes are
/// not handled (the registry spec doesn't use them in practice).
fn split_params(s: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let Some(eq_idx) = rest.find('=') else {
            break;
        };
        let key = rest[..eq_idx].trim();
        let after_eq = &rest[eq_idx + 1..];
        let (value, advance) = if let Some(stripped) = after_eq.strip_prefix('"') {
            // Quoted value — find the closing quote.
            let close = stripped.find('"').unwrap_or(stripped.len());
            (&stripped[..close], eq_idx + 1 + 1 + close + 1) // +1 open quote, +1 close quote
        } else {
            // Bare value — until next comma or end.
            let end = after_eq.find(',').unwrap_or(after_eq.len());
            (after_eq[..end].trim(), eq_idx + 1 + end)
        };
        out.push((key, value));
        rest = if advance >= rest.len() {
            ""
        } else {
            rest[advance..].trim_start_matches([',', ' '])
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ghcr_style_challenge() {
        let h = r#"Bearer realm="https://ghcr.io/token",service="ghcr.io",scope="repository:argo/x:pull,push""#;
        let c = BearerChallenge::parse(h).unwrap();
        assert_eq!(c.realm, "https://ghcr.io/token");
        assert_eq!(c.service.as_deref(), Some("ghcr.io"));
        assert_eq!(c.scope.as_deref(), Some("repository:argo/x:pull,push"));
    }

    #[test]
    fn parses_docker_hub_style_challenge() {
        let h = r#"Bearer realm="https://auth.docker.io/token",service="registry.docker.io""#;
        let c = BearerChallenge::parse(h).unwrap();
        assert_eq!(c.realm, "https://auth.docker.io/token");
        assert_eq!(c.service.as_deref(), Some("registry.docker.io"));
        assert!(c.scope.is_none());
    }

    #[test]
    fn rejects_non_bearer_challenges() {
        assert!(BearerChallenge::parse("Basic realm=\"foo\"").is_none());
    }

    #[test]
    fn token_url_appends_query_params() {
        let c = BearerChallenge {
            realm: "https://ghcr.io/token".into(),
            service: Some("ghcr.io".into()),
            scope: Some("repository:argo/x:pull".into()),
        };
        assert_eq!(
            c.token_url(),
            "https://ghcr.io/token?service=ghcr.io&scope=repository:argo/x:pull"
        );
    }

    #[test]
    fn token_url_handles_realm_only_challenge() {
        let c = BearerChallenge {
            realm: "https://example/token".into(),
            service: None,
            scope: None,
        };
        assert_eq!(c.token_url(), "https://example/token");
    }
}
