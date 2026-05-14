//! `[sso]` — Single Sign-On (OIDC + SAML) at the gateway edge.
//! Extracted from [`crate::sections`] to keep that module under the
//! 500-LOC ceiling. Re-exported through `sections` so the public API
//! is unchanged.
//!
//! Brief: `mission-sso-saml-oidc-gateway`. SOURCE: MISSION_BACKLOG.md
//! Section 7 #232. v1 ships an additive auth mechanism for human-
//! facing routes; bearer-token auth continues to work unchanged.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[sso]` parent. Off by default — when `enabled = false` the gateway
/// mounts none of the `/auth/*` routes (acceptance criterion: "default
/// — no SSO routes mounted; backward compat"). Per-tenant IdP
/// registrations live in the `[sso.oidc.<tenant>]` and
/// `[sso.saml.<tenant>]` tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SsoConfig {
    /// Master enable. When `false`, dispatch helpers short-circuit
    /// without inspecting any other field.
    pub enabled: bool,
    /// Session-cookie TTL (seconds). Default 8 hours.
    pub session_ttl_seconds: u64,
    /// Cookie name; configurable so operators behind a reverse proxy
    /// can disambiguate from their own session cookie.
    pub cookie_name: String,
    /// Per-tenant OIDC IdP configs, keyed by tenant id. The TOML
    /// surface is `[sso.oidc.<tenant>]`.
    pub oidc: BTreeMap<String, SsoOidcTenant>,
    /// Per-tenant SAML IdP configs, keyed by tenant id. The TOML
    /// surface is `[sso.saml.<tenant>]`.
    pub saml: BTreeMap<String, SsoSamlTenant>,
}

impl Default for SsoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            session_ttl_seconds: 8 * 3600,
            cookie_name: "argo_sso".into(),
            oidc: BTreeMap::new(),
            saml: BTreeMap::new(),
        }
    }
}

/// `[sso.oidc.<tenant>]` — one OpenID Connect IdP registration.
///
/// `client_secret_env` is env-indirect so the TOML never carries a
/// plaintext secret; operators rotate the secret without
/// restart-and-edit-config (same pattern as
/// `[channels.teams].client_secret_env`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SsoOidcTenant {
    /// IdP-issued client id for this deployment.
    pub client_id: String,
    /// Env-var name carrying the client secret. The gateway reads it
    /// at request time via `argo_config::cred::get`.
    pub client_secret_env: String,
    /// IdP authorization endpoint (URL).
    pub authorization_endpoint: String,
    /// IdP token endpoint (URL).
    pub token_endpoint: String,
    /// Our SP callback URL — the value we redirect the IdP to after
    /// auth, e.g. `https://gw.example.com/auth/callback`. Must be
    /// pre-registered with the IdP.
    pub redirect_uri: String,
    /// OAuth scopes joined by spaces. Default `openid email profile`.
    pub scopes: String,
}

/// `[sso.saml.<tenant>]` — one SAML IdP registration.
///
/// v1 ships SP-initiated flow only; IdP-initiated SSO is explicitly
/// out of scope per brief. XML signature verification of the SAML
/// Response is deferred to a follow-up brief (the `idp_cert_pem`
/// field is parsed but not yet consulted).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SsoSamlTenant {
    /// IdP-side entity id (the SAML `Issuer`).
    pub idp_entity_id: String,
    /// IdP SSO endpoint (URL the SP redirects to with `SAMLRequest`).
    pub idp_sso_url: String,
    /// SP-side entity id (what we put in our AuthnRequest `Issuer`).
    pub sp_entity_id: String,
    /// Our ACS callback URL — where the IdP POSTs the SAML Response,
    /// e.g. `https://gw.example.com/auth/saml/<tenant>/acs`. Must be
    /// pre-registered with the IdP.
    pub acs_url: String,
    /// IdP public-cert PEM block. Stored for forward-compat with the
    /// follow-up signature-verification brief; the v1 ACS handler
    /// parses the response structurally and does not consult this.
    pub idp_cert_pem: String,
}
