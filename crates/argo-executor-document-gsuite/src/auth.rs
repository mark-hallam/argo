//! Google Workspace service-account OAuth2 ("assertion grant") flow.
//!
//! Mirrors the
//! [Microsoft Graph teams.rs](https://github.com/mark-hallam/Argo-Agent/blob/main/crates/argo-channels/src/teams.rs)
//! token-cache + refresh-on-401 pattern. The credential
//! (`private_key_pem`) is loaded env-indirect and never serialised
//! back; only the bearer token (in memory) crosses module boundaries.
//!
//! ## Flow
//!
//! 1. Build a JWT (`iss=client_email`, `aud=token_uri`,
//!    `scope=drive+docs+sheets+slides`, `iat=now`, `exp=now+3600`).
//! 2. Sign with `RS256` using the service-account private key (PEM).
//! 3. POST `grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer` +
//!    `assertion=<signed JWT>` to the token endpoint.
//! 4. Cache `{access_token, expires_at}` per-process; refresh within
//!    [`TOKEN_REFRESH_WINDOW`] of expiry or on any 401 from the API.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Refresh the cached service-account token within this window of
/// stated expiry. Mirrors `teams.rs::TOKEN_REFRESH_WINDOW`.
pub const TOKEN_REFRESH_WINDOW: Duration = Duration::minutes(5);

/// Default Google OAuth2 token endpoint. Override via
/// [`GsuiteAuthConfig::token_uri`] for tests.
pub const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";

/// OAuth2 scopes the service-account requests. Drive scope covers
/// `files.create` (the parent op); per-API scopes cover the
/// `batchUpdate` writes.
pub const DEFAULT_SCOPES: &str = concat!(
    "https://www.googleapis.com/auth/drive ",
    "https://www.googleapis.com/auth/documents ",
    "https://www.googleapis.com/auth/spreadsheets ",
    "https://www.googleapis.com/auth/presentations",
);

/// Service-account credentials + token endpoint. Loaded once at
/// executor construction; the secret never leaves the process.
#[derive(Debug, Clone)]
pub struct GsuiteAuthConfig {
    /// Service-account email (`<sa>@<project>.iam.gserviceaccount.com`).
    pub client_email: String,
    /// PEM-encoded private key body (`-----BEGIN PRIVATE KEY-----...`).
    pub private_key_pem: String,
    /// OAuth2 token endpoint. Defaults to [`DEFAULT_TOKEN_URI`].
    pub token_uri: String,
    /// Space-separated scopes. Defaults to [`DEFAULT_SCOPES`].
    pub scopes: String,
}

impl GsuiteAuthConfig {
    /// Load `client_email` + `private_key_pem` from env. Env var names
    /// match the brief: `ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL` +
    /// `ARGO_GSUITE_SERVICE_ACCOUNT_KEY`.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when either env var is unset or
    /// empty. The error message names the missing var; the private
    /// key's content is never echoed.
    pub fn from_env() -> ArgoResult<Self> {
        let client_email = std::env::var("ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL")
            .map_err(|_| {
                ArgoError::Schema(
                    "ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL must be set".to_string(),
                )
            })?;
        if client_email.trim().is_empty() {
            return Err(ArgoError::Schema(
                "ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL must not be empty".to_string(),
            ));
        }
        let private_key_pem = std::env::var("ARGO_GSUITE_SERVICE_ACCOUNT_KEY")
            .map_err(|_| {
                ArgoError::Schema(
                    "ARGO_GSUITE_SERVICE_ACCOUNT_KEY must be set".to_string(),
                )
            })?;
        if private_key_pem.trim().is_empty() {
            return Err(ArgoError::Schema(
                "ARGO_GSUITE_SERVICE_ACCOUNT_KEY must not be empty".to_string(),
            ));
        }
        Ok(Self {
            client_email,
            private_key_pem,
            token_uri: DEFAULT_TOKEN_URI.into(),
            scopes: DEFAULT_SCOPES.into(),
        })
    }

    /// Test-only constructor pointing at a closed local port. The
    /// private key body is the canned RS256 fixture documented in
    /// [`test_fixtures`].
    #[doc(hidden)]
    #[must_use]
    pub fn disabled_for_tests() -> Self {
        Self {
            client_email: "test-sa@argo-test.iam.gserviceaccount.com".into(),
            private_key_pem: TEST_RSA_PRIVATE_KEY.into(),
            token_uri: "http://127.0.0.1:9/token".into(),
            scopes: DEFAULT_SCOPES.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

/// JWT claims emitted by [`GsuiteAuth::build_assertion`]. Pub-visible
/// for white-box tests; not stable API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// `iss` — service-account email.
    pub iss: String,
    /// `scope` — space-separated OAuth2 scopes.
    pub scope: String,
    /// `aud` — token endpoint URL.
    pub aud: String,
    /// `iat` — issued-at (epoch seconds).
    pub iat: u64,
    /// `exp` — expiry (epoch seconds). Capped at `iat + 3600`.
    pub exp: u64,
}

/// Token mint + cache. Holds the credential, the cached bearer
/// token, and the reqwest client used to talk to the token endpoint.
pub struct GsuiteAuth {
    config: GsuiteAuthConfig,
    client: reqwest::Client,
    token: RwLock<Option<CachedToken>>,
}

impl std::fmt::Debug for GsuiteAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redact the private key. The Debug impl is the easy way for
        // a secret to leak into stderr via `dbg!` or `tracing`.
        f.debug_struct("GsuiteAuth")
            .field("client_email", &self.config.client_email)
            .field("token_uri", &self.config.token_uri)
            .field("scopes", &self.config.scopes)
            .field("private_key_pem", &"<redacted>")
            .finish()
    }
}

impl GsuiteAuth {
    /// Construct an auth handle. Does not mint a token until
    /// [`Self::acquire_token`] is called.
    #[must_use]
    pub fn new(config: GsuiteAuthConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            token: RwLock::new(None),
        }
    }

    /// Construct with a custom reqwest client. Used by wiremock tests
    /// that want to install a mock server's base URL.
    #[doc(hidden)]
    #[must_use]
    pub fn with_client(config: GsuiteAuthConfig, client: reqwest::Client) -> Self {
        Self {
            config,
            client,
            token: RwLock::new(None),
        }
    }

    /// Seed the cache with a synthetic token (test-only helper for
    /// the proactive-refresh path).
    #[doc(hidden)]
    pub fn seed_token(&self, access_token: impl Into<String>, expires_at: DateTime<Utc>) {
        *self.token.write() = Some(CachedToken {
            access_token: access_token.into(),
            expires_at,
        });
    }

    /// Force the cache to empty. The next [`Self::acquire_token`]
    /// hits the token endpoint. Called by the executor on a 401 from
    /// any API surface.
    pub fn invalidate(&self) {
        *self.token.write() = None;
    }

    /// Return a bearer token, refreshing if cache is empty or near
    /// expiry. Pure read on the happy path (single `RwLock` read +
    /// timestamp check) — no token-endpoint round-trip.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] on network failure to the
    /// token endpoint, and [`ArgoError::Schema`] on a malformed
    /// `access_token` / `expires_in` response.
    pub async fn acquire_token(&self) -> ArgoResult<String> {
        if let Some(tok) = self.token.read().clone() {
            if tok.expires_at - Utc::now() > TOKEN_REFRESH_WINDOW {
                return Ok(tok.access_token);
            }
        }
        self.refresh_token().await
    }

    async fn refresh_token(&self) -> ArgoResult<String> {
        let assertion = self.build_assertion()?;
        let form = [
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:jwt-bearer",
            ),
            ("assertion", assertion.as_str()),
        ];
        let resp = self
            .client
            .post(&self.config.token_uri)
            .form(&form)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("gsuite token: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| ArgoError::Provider(format!("gsuite token body: {e}")))?;
        if !status.is_success() {
            return Err(ArgoError::Provider(format!(
                "gsuite token {status}: {body}"
            )));
        }
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ArgoError::Schema(format!("gsuite token JSON: {e}")))?;
        let access_token = parsed
            .get("access_token")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ArgoError::Schema("gsuite token missing access_token".to_string())
            })?
            .to_string();
        let expires_in = parsed
            .get("expires_in")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(3600);
        *self.token.write() = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at: Utc::now() + Duration::seconds(expires_in),
        });
        Ok(access_token)
    }

    /// Build the signed JWT assertion. Pub-visible for white-box
    /// tests that round-trip the claim set.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when the PEM body is malformed.
    /// The error never echoes the private-key body.
    pub fn build_assertion(&self) -> ArgoResult<String> {
        let now = Utc::now().timestamp() as u64;
        let claims = JwtClaims {
            iss: self.config.client_email.clone(),
            scope: self.config.scopes.clone(),
            aud: self.config.token_uri.clone(),
            iat: now,
            exp: now + 3600,
        };
        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(self.config.private_key_pem.as_bytes())
            .map_err(|e| {
                ArgoError::Schema(format!(
                    "gsuite private key (PEM): {e} — check the env var carries the full \
                     -----BEGIN PRIVATE KEY----- body"
                ))
            })?;
        encode(&header, &claims, &key)
            .map_err(|e| ArgoError::Schema(format!("gsuite JWT sign: {e}")))
    }
}

/// RSA-2048 private key used by the test suite. Generated specifically
/// for tests with `openssl genpkey`; not a real credential. Kept
/// inline so the `wiremock` tests need no external fixture file.
#[doc(hidden)]
pub const TEST_RSA_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCcslIMjzbYEOhS
TzokxU1yMhBg3ucjJjfh0TIDbUmfDAqSwqShAxKwSsBm36QsUUUgTXe/DeAZy2bn
3A8jXj4Uk2I77GqzKFjr0W5k762qaOy++ErWbGRXGDb4vHFqAwXi99MtKx3oVswM
0FOKHZkRxqkIRSMRSc1PW592V0gSl7IAXqc4UJSj2G0L7v88LYhPZ+8TzVkK9BHZ
UkW5uObg3nehap7zPWwhfaQw/WeE0zgT54IViA4TGWBu3FwzyMYAQjLq7W8Mcwn3
Tg1KJsomMW6DAEwZAG/z+2yZf3Uya3P0iJkiwAiOx2ZBdcpTSnIqe6Uu232guSPA
/7lNmadPAgMBAAECggEAOfsZtSABES4gaOR8BUMkhw7TVKTmLfFkWCCKOw5jYYCZ
IPj45cM3VRgz3Vz/aI6h33QKzR0bQTAsYbrhtxqWKhLnFHlEegqB7LBQAvVU82tj
vr4tIu50qm2BWcSQysNXmEZPU7VSmbgUROGJJ+AP1a4GudKrSShf2e9zL9zYBFic
+pQWtDkvk7Y6iHG9uCTvaa0OMGtEeukdVNkFebmcrUh/1x/6jtLGa6e9LQHB2BCb
i3ET3EdOyMPfSxCCD6iEEggnQ6CcmmitkGgo+1OowbQ4CDLJIIWWqHYNPtPbXhAq
HKqJjUudbEjPJ5jYE9XZEy+NGn0D/bQubedRAVtwMQKBgQDMUZGBsX4GGTMYOtiS
TUs4HQtqdbbhSSyVqf7Nk4tV4whj8bVDRFI8vct1qoegkeyWa698y1jhZbCu7NEy
Kk1N1dJxGIQMqQNGithE5NzNdnYCye63XmP0FT2QreedjU22MvplYent6KXPXIcZ
JfZa6QUVRKoxc2pRu1yvFTiuCwKBgQDEVQl5o59REizABgQY7JNlY1xUVjPOUey6
AVvDc+fzhLI8UADXhqmYAHYa92LEHxuLuEIplywqSnGKDuoRT73aUPHtvXft1OhX
oNho8ofpaqjtoRipPMSQ1c2c3ctgzGnEzJdHU8CPk3bqI98XvSH1QJHb1pOzuwpT
verWRi6qTQKBgD9otSKt5fGYkn3bjWjiKgzxHqDchXQnVHniNxyvxJdUDT+PPPTy
WB9Xu7P5x3uyrd0E8IJnTV8uQwYWotnRvQoB6PyGYjzXmxeicXn4GPp+4CL/bmFk
HtsaWG/1OKNxvgNkkMcz+viFsifHjHnmiS0vkg9GrxcHvaA96IbBrER/AoGAR7fY
kjqXBAJjnnGUJZal8S9ultA2dHOwyfGdF7TLUkVy5l1DMXfbk873Vbpwtbu3UD6q
zI6Qb5JqD0iLcG90pr9S4MmJebOAfCIha5TM2RNBSizC2W99TnsLdPYASQfiyyUB
TejmOoxfsAASipVoRjKkFW443jrA9v41Pri8azECgYBEYc+SmD0kswlasQ7r/jwJ
iIDkFAwIRtl6TwfHDwEEOmk4kq/uKQcEmaXxQPhr4f1PDMR5ZH8NvMASmya3vSYE
PY7/nCZ+OQvKCXptCUvV6cElifcg0wUCvN8qsfbNWuWXovH4QdSSc5vVkBQbRU65
BoQUU1DFgmNU7+ERSaOZvg==
-----END PRIVATE KEY-----
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_assertion_roundtrips_claims() {
        let auth = GsuiteAuth::new(GsuiteAuthConfig {
            client_email: "sa@x.iam.gserviceaccount.com".into(),
            private_key_pem: TEST_RSA_PRIVATE_KEY.into(),
            token_uri: "http://127.0.0.1:9/token".into(),
            scopes: "scope-a scope-b".into(),
        });
        let jwt = auth.build_assertion().unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have header.body.signature");
        use base64::Engine;
        let claims_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&claims_bytes).unwrap();
        assert_eq!(claims["iss"].as_str().unwrap(), "sa@x.iam.gserviceaccount.com");
        assert_eq!(claims["scope"].as_str().unwrap(), "scope-a scope-b");
        assert!(claims["aud"].as_str().unwrap().ends_with("/token"));
    }

    #[test]
    fn debug_impl_redacts_private_key() {
        let auth = GsuiteAuth::new(GsuiteAuthConfig {
            client_email: "sa@x.iam.gserviceaccount.com".into(),
            private_key_pem:
                "-----BEGIN PRIVATE KEY-----\nSECRETBODY\n-----END PRIVATE KEY-----\n".into(),
            token_uri: "http://127.0.0.1:9/token".into(),
            scopes: "scope-a".into(),
        });
        let rendered = format!("{auth:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(
            !rendered.contains("SECRETBODY"),
            "private key body leaked into Debug: {rendered}"
        );
    }
}
