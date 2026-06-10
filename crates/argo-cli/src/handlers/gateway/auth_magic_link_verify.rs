use super::{
    emit_auth_failed, emit_auth_succeeded, mint_jwt, sha256_hex, verify_rate_limit_check, IpAddr,
    SessionStore, JWT_TTL_SECS, VERIFY_RATE_LIMIT_WINDOW,
};
use argo_identity::user::UserAccount;
use argo_identity::user_store::{get_user_by_email, insert_user, update_last_login};
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::user_tenant_store::list_tenants_for_user;
use argo_identity::{provision_for_user, ProvisionRequest};
use argo_store::user_account::{self, ConsumedToken};
use argo_types::ArgoResult;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

const SESSION_COOKIE_NAME: &str = "__Host-session";
const CSRF_COOKIE_NAME: &str = "__Host-csrf";

#[derive(Debug, Deserialize)]
struct VerifyBody {
    token: String,
}

/// Stream-owning verify handler that emits the `__Host-session`
/// cookie directly alongside the SPA-readable JSON envelope.
pub(crate) async fn serve_verify_stream(
    req: &str,
    src_ip: IpAddr,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let outcome = verify_outcome(req, src_ip, store).await;
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        outcome.status,
        outcome.body.len()
    );
    if let Some(cookie) = &outcome.set_cookie {
        head.push_str("Set-Cookie: ");
        head.push_str(cookie);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    head.push_str(&outcome.body);
    out.write_all(head.as_bytes()).await?;
    out.shutdown().await
}

pub(crate) struct VerifyOutcome {
    pub status: &'static str,
    pub body: String,
    pub set_cookie: Option<String>,
}

pub(crate) async fn verify_outcome(
    req: &str,
    src_ip: IpAddr,
    store: Option<&Arc<SessionStore>>,
) -> VerifyOutcome {
    // The 60/IP/min token-brute-force throttle keys on the trusted
    // `src_ip` the accept loop extracted (`boot_router` →
    // `proxy_guard::extract_real_ip`), never on a client-supplied
    // `X-Forwarded-For`. Rotating the forged header therefore cannot mint
    // a fresh verify bucket per request (fix-verify-route-xff-spoof-reland,
    // mirroring the signup route's `signup_limiter().check(src_ip)`).
    let source_ip = src_ip;
    let Some(s) = store else {
        return err_outcome("500 Internal Server Error", "store_unavailable");
    };
    if !verify_rate_limit_check(source_ip, Instant::now()) {
        emit_auth_failed(s, "rate_limited", source_ip).await;
        return VerifyOutcome {
            status: "429 Too Many Requests",
            body: json!({
                "error": "rate_limited",
                "scope": "verify",
                "retry_after_secs": VERIFY_RATE_LIMIT_WINDOW.as_secs(),
            })
            .to_string(),
            set_cookie: None,
        };
    }
    let parsed: VerifyBody =
        match serde_json::from_str(crate::handlers::gateway::body_from_request(req)) {
            Ok(v) => v,
            Err(e) => {
                emit_auth_failed(s, "missing", source_ip).await;
                return err_outcome("400 Bad Request", &format!("invalid body: {e}"));
            }
        };
    let token = parsed.token.trim();
    if token.is_empty() {
        emit_auth_failed(s, "missing", source_ip).await;
        return err_outcome("400 Bad Request", "token is required");
    }
    let now = Utc::now().timestamp();
    let email = match user_account::consume_magic_link(s.pool(), &sha256_hex(token), now).await {
        Ok(ConsumedToken::Ok { email }) => email,
        Ok(ConsumedToken::Expired) => {
            emit_auth_failed(s, "expired", source_ip).await;
            return err_outcome("401 Unauthorized", "expired");
        }
        Ok(ConsumedToken::AlreadyUsed) => {
            emit_auth_failed(s, "already_used", source_ip).await;
            return err_outcome("401 Unauthorized", "already_used");
        }
        Ok(ConsumedToken::NotFound) => {
            emit_auth_failed(s, "invalid", source_ip).await;
            return err_outcome("401 Unauthorized", "invalid");
        }
        Err(e) => return err_outcome("500 Internal Server Error", &format!("store: {e}")),
    };
    let user = match upsert_user(s, &email).await {
        Ok(u) => u,
        Err(e) => return err_outcome("500 Internal Server Error", &format!("user: {e}")),
    };
    let (tenant_id, role, needs_onboarding) = match resolve_session_tenant(s, &user).await {
        Ok(triple) => triple,
        Err(e) => return err_outcome("500 Internal Server Error", &format!("tenant: {e}")),
    };
    let (jwt, jti) = match mint_jwt(
        &user.id.to_string(),
        &email,
        tenant_id.as_deref(),
        role.as_deref(),
        now,
    ) {
        Ok(pair) => pair,
        Err(msg) => return err_outcome("500 Internal Server Error", &msg),
    };
    emit_auth_succeeded(s, &user.id.to_string(), tenant_id.as_deref(), source_ip).await;
    let body = json!({
        "needs_onboarding": needs_onboarding,
        "tenant_id": tenant_id,
    });
    // Two `Set-Cookie` headers in one wire response: the session JWT
    // (HttpOnly so JS can't read it) and the CSRF token (NOT
    // HttpOnly so the SPA can echo it in `X-CSRF-Token`). The
    // double-submit pattern in `middleware_csrf` compares the
    // cookie value to the header value. Emitted as one
    // `\r\nSet-Cookie: `-separated string so the stream writer
    // emits two header lines without needing a `Vec<String>`.
    VerifyOutcome {
        status: "200 OK",
        body: body.to_string(),
        set_cookie: Some(format!(
            "{SESSION_COOKIE_NAME}={jwt}; Max-Age={JWT_TTL_SECS}; Path=/; HttpOnly; Secure; SameSite=Lax\r\nSet-Cookie: {CSRF_COOKIE_NAME}={jti}; Max-Age={JWT_TTL_SECS}; Path=/; Secure; SameSite=Lax"
        )),
    }
}

pub(crate) fn err_outcome(status: &'static str, msg: &str) -> VerifyOutcome {
    VerifyOutcome {
        status,
        body: serde_json::json!({ "error": msg }).to_string(),
        set_cookie: None,
    }
}

async fn upsert_user(store: &SessionStore, email: &str) -> ArgoResult<UserAccount> {
    if let Some(u) = get_user_by_email(store, email).await? {
        update_last_login(store, u.id, Utc::now()).await?;
        return Ok(u);
    }
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await?;
    Ok(user)
}

/// Resolve the tenant binding to stamp into the verifying user's session
/// JWT. Returns `(tenant_id, role, needs_onboarding)`.
///
/// - **Returning user** (already linked to ≥1 tenant): bind their newest
///   tenant + the role they hold there; `needs_onboarding = false`.
/// - **First-time user** (no tenant yet): auto-provision their OWN tenant
///   here (WS1-3) and bind it as `owner`; `needs_onboarding = true`.
///
/// Why provision on verify: the per-request resolver
/// ([`super::resolve_tenant::resolve_tenant_for_request`]) reads the
/// `tnt` claim from the `__Host-session` JWT. A freshly-verified user
/// with no tenant would mint a `tnt`-less JWT, so the resolver would
/// fall back to the gateway's cwd/demo tenant — the user's very first
/// data read could momentarily resolve to shared demo data. Provisioning
/// here guarantees the minted JWT always carries the user's real tenant.
/// [`provision_for_user`] is idempotent, so a re-verify reuses the
/// existing tenant rather than creating a second one.
async fn resolve_session_tenant(
    store: &SessionStore,
    user: &UserAccount,
) -> ArgoResult<(Option<String>, Option<String>, bool)> {
    if let Some(t) = list_tenants_for_user(store, user.id).await?.into_iter().next() {
        return Ok((
            Some(t.tenant_id.to_string()),
            Some(t.role.as_str().to_string()),
            false,
        ));
    }
    let tenant_id = provision_on_verify(store, user).await?;
    Ok((
        Some(tenant_id),
        Some(UserTenantRole::Owner.as_str().to_string()),
        true,
    ))
}

/// Auto-provision the verifying user's own tenant + workspace and return
/// the provisioned `tenant_id` as a string for the JWT `tnt` claim.
async fn provision_on_verify(store: &SessionStore, user: &UserAccount) -> ArgoResult<String> {
    let workspace_name = default_workspace_name(&user.email);
    let outcome = provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: &workspace_name,
            company_name: None,
        },
    )
    .await?;
    Ok(outcome.tenant_id.to_string())
}

/// A non-empty default workspace label derived from the verified email's
/// local-part (`alice@acme.com` → `alice's Workspace`). Only needs to be
/// non-empty so [`provision_for_user`] accepts it; the user can rename
/// their workspace in onboarding / settings later.
fn default_workspace_name(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("").trim();
    if local.is_empty() {
        "My Workspace".to_string()
    } else {
        format!("{local}'s Workspace")
    }
}

#[cfg(test)]
#[path = "auth_magic_link_verify_provision_tests.rs"]
mod provision_tests;
