use super::{
    emit_auth_failed, emit_auth_succeeded, mint_jwt, sha256_hex, signup_source_ip,
    verify_rate_limit_check, SessionStore, VERIFY_RATE_LIMIT_WINDOW,
};
use argo_identity::user::UserAccount;
use argo_identity::user_store::{get_user_by_email, insert_user, update_last_login};
use argo_identity::user_tenant_store::list_tenants_for_user;
use argo_store::user_account::{self, ConsumedToken};
use argo_types::ArgoResult;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

const JWT_TTL_SECS: i64 = 7 * 24 * 60 * 60;
const SESSION_COOKIE_NAME: &str = "__Host-session";

#[derive(Debug, Deserialize)]
struct VerifyBody {
    token: String,
}

/// Stream-owning verify handler that emits the `__Host-session`
/// cookie directly alongside the SPA-readable JSON envelope.
pub(crate) async fn serve_verify_stream(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let outcome = verify_outcome(req, store).await;
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

pub(crate) async fn verify_outcome(req: &str, store: Option<&Arc<SessionStore>>) -> VerifyOutcome {
    let source_ip = signup_source_ip(req);
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
    let (tenant_id, role) = match list_tenants_for_user(s, user.id).await {
        Ok(tenants) => match tenants.into_iter().next() {
            Some(t) => (Some(t.tenant_id.to_string()), Some(t.role.as_str().to_string())),
            None => (None, None),
        },
        Err(e) => return err_outcome("500 Internal Server Error", &format!("tenant: {e}")),
    };
    let jwt = match mint_jwt(
        &user.id.to_string(),
        &email,
        tenant_id.as_deref(),
        role.as_deref(),
        now,
    ) {
        Ok(t) => t,
        Err(msg) => return err_outcome("500 Internal Server Error", &msg),
    };
    emit_auth_succeeded(s, &user.id.to_string(), tenant_id.as_deref(), source_ip).await;
    let body = json!({
        "needs_onboarding": tenant_id.is_none(),
        "tenant_id": tenant_id,
    });
    VerifyOutcome {
        status: "200 OK",
        body: body.to_string(),
        set_cookie: Some(format!(
            "{SESSION_COOKIE_NAME}={jwt}; Max-Age={JWT_TTL_SECS}; Path=/; HttpOnly; Secure; SameSite=Lax"
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
