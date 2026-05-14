//! Mini-dispatcher for SSO routes (`mission-sso-saml-oidc-gateway`).
//!
//! Fans `/auth/*` HTTP traffic out to [`super::sso_oidc::serve`] and
//! [`super::sso_saml::serve`]. Lives in its own module so the
//! `accept` loop in [`super::boot::start`] adds exactly one arm
//! instead of two — keeps boot.rs under the 500-LOC ceiling.
use argo_store::SessionStore;
use std::sync::Arc;
use tokio::net::TcpStream;

/// True when this request is a member of the SSO route surface.
pub(super) fn is_sso_request(req: &str) -> bool {
    super::sso_oidc::is_oidc_request(req) || super::sso_saml::is_saml_request(req)
}

/// Hand off to whichever SSO submodule owns the route. Both
/// handlers are stream-owning so they can emit `Location:` +
/// `Set-Cookie:` headers, which the `(status, body)` dispatch
/// tuple used everywhere else cannot do.
pub(super) async fn serve(req: &str, store: Option<&Arc<SessionStore>>, out: &mut TcpStream) {
    if super::sso_oidc::is_oidc_request(req) {
        let _ = super::sso_oidc::serve(req, store, out).await;
    } else if super::sso_saml::is_saml_request(req) {
        let _ = super::sso_saml::serve(req, store, out).await;
    }
}
