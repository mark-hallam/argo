//! Harness auth — Copilot OAuth, Cursor token, generic bearer.
//!
//! ACP harnesses authenticate before issuing requests. This module
//! ships the `AuthScheme` enum + a sync `verify()` that returns the
//! authenticated `Principal` for downstream capability checks.
//! Real network-bound auth (OAuth refresh, OIDC userinfo lookup)
//! lives in the wiring crate; the seam here lets us test the
//! per-scheme acceptance + rejection logic deterministically.
use argo_types::Principal;
use serde::{Deserialize, Serialize};

/// Auth scheme advertised by an ACP harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthScheme {
    /// GitHub Copilot OAuth flow — token format `gho_*` /
    /// `ghu_*` (an installation token).
    CopilotOAuth,
    /// Cursor agent CLI — opaque project token, prefix `cursor_`.
    CursorToken,
    /// Generic bearer — caller supplies a registered token.
    GenericBearer,
}

/// A token presented by a harness.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthToken(pub String);

/// Auth verification error.
#[derive(Debug, Clone, PartialEq, Eq, ::thiserror::Error)]
pub enum AuthError {
    /// Token is empty / blank.
    #[error("empty token")]
    Empty,
    /// Token does not match the expected prefix for the scheme.
    #[error("malformed token for scheme {scheme:?}: {detail}")]
    Malformed {
        /// Scheme that rejected the token.
        scheme: AuthScheme,
        /// Why.
        detail: String,
    },
    /// Token is well-formed but rejected by policy (e.g. revoked).
    #[error("token rejected by policy")]
    Rejected,
}

/// Verify a token against a scheme. On success returns the
/// `Principal` to use for capability checks. The shape of the
/// principal depends on scheme — Copilot/Cursor are
/// `Principal::Acp { harness, subject }`-style; generic bearer
/// resolves to a registered `Principal::Local`.
///
/// This is the **shape check** — real registry lookup happens upstream.
/// Use it as the entry point: callers pass the token in, get a
/// principal back, and feed that into [`argo_capability::CapabilityEngine`]
/// for permission decisions.
///
/// # Errors
/// Returns [`AuthError`] if the token is empty or the format does
/// not match the declared scheme.
pub fn verify(scheme: AuthScheme, token: &AuthToken) -> Result<Principal, AuthError> {
    if token.0.trim().is_empty() {
        return Err(AuthError::Empty);
    }
    match scheme {
        AuthScheme::CopilotOAuth => {
            if !(token.0.starts_with("gho_") || token.0.starts_with("ghu_")) {
                return Err(AuthError::Malformed {
                    scheme,
                    detail: "Copilot OAuth tokens start with gho_ / ghu_".into(),
                });
            }
            Ok(Principal::Acp {
                harness: "copilot".into(),
                identity: token.0[..8.min(token.0.len())].to_string(),
            })
        }
        AuthScheme::CursorToken => {
            if !token.0.starts_with("cursor_") {
                return Err(AuthError::Malformed {
                    scheme,
                    detail: "Cursor tokens start with cursor_".into(),
                });
            }
            Ok(Principal::Acp {
                harness: "cursor".into(),
                identity: token.0[..14.min(token.0.len())].to_string(),
            })
        }
        AuthScheme::GenericBearer => Ok(Principal::Acp {
            harness: "generic".into(),
            identity: token.0[..8.min(token.0.len())].to_string(),
        }),
    }
}
