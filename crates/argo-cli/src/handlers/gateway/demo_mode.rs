//! Env-var-gated demo-mode signup bypass for v1.0 mission-MVP
//! recording (`demo-ops-add-magic-link-bypass-for-recording`).
//!
//! `POST /api/v1/demo/mint-principal` mints a verified
//! `human:<email>` principal session WITHOUT a real magic-link
//! email round-trip, so the operator can record the 5-7 minute
//! demo as a single take. Active only when `ARGO_DEMO_MODE=1` is
//! set in the gateway's startup environment — the flag is cached
//! on first read, no runtime flip.
//!
//! Every mint emits a `tracing::warn!` line and writes an
//! `AuditKind::AuthSucceeded` row whose payload carries
//! `demo_mode_bypass: true` — so an operator who accidentally
//! booted with the flag in a prod-shaped environment sees obvious
//! noise plus a permanent forensic record.
//!
//! Mounted ahead of the bearer-auth gate (mirroring `auth_magic_link::dispatch`)
//! so the SPA can call it anonymously. Startup-time refusal to
//! co-exist with bearer-auth lives in [`super::startup_check`].
use super::auth_magic_link::{jwt_secret, looks_like_email, JWT_TTL_SECS};
use argo_audit::{AuditKind, AuditSink, ANONYMOUS_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, OnceLock};

/// Env var that gates the bypass. Read once via [`is_enabled`] and
/// cached for the lifetime of the gateway process.
pub(super) const DEMO_MODE_ENV: &str = "ARGO_DEMO_MODE";

/// Returns `true` when `ARGO_DEMO_MODE=1` was set at gateway start.
/// Cached on first read so a malicious caller can't flip the flag
/// mid-process via `std::env::set_var`.
pub(super) fn is_enabled() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        std::env::var(DEMO_MODE_ENV)
            .map(|v| v == "1")
            .unwrap_or(false)
    })
}

/// Route classifier — `true` for `POST /api/v1/demo/mint-principal`.
pub(super) fn is_mint_principal_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/demo/mint-principal ")
        || req.starts_with("POST /api/v1/demo/mint-principal?")
}

#[derive(Debug, Deserialize)]
struct MintBody {
    email: String,
}

/// Role stamped into every demo-mint JWT.
///
/// The demo-mode flow mints a verified `human:<email>` principal who
/// is the sole occupant of a freshly-provisioned demo workspace — the
/// same standing a real workspace's first/owner user holds. The
/// magic-link verify path (`auth_magic_link_token::mint_jwt`) carries
/// the membership-derived role; demo-mint has no membership lookup, so
/// it stamps `owner` directly.
///
/// Without this claim the verify-side `SessionClaims.role` decoded
/// `None`, `role_gate::resolve_role` returned `None`, and
/// `routes_workflow_execute::resolve_principal` fell back to the
/// most-restrictive `guest` role (`unwrap_or(UserTenantRole::Guest)`).
/// `guest` holds `workflow.read` but NOT `workflow.edit`, so the
/// `workflow_edit_capability()` check 403'd every demo "Run workflow"
/// POST ("Demo principal cannot execute workflows yet"), and Scene 5's
/// computer-use beat never fired.
///
/// Scope: this string is only ever minted into a demo-mode JWT (the
/// route is mounted only when [`is_enabled`] / `ARGO_DEMO_MODE=1`), so
/// it cannot elevate any non-demo principal. The capability gate is
/// untouched — `user` / `guest` JWTs and anonymous callers still 403.
const DEMO_PRINCIPAL_ROLE: &str = "owner";

#[derive(Debug, Serialize)]
struct Claims<'a> {
    sub: &'a str,
    email: &'a str,
    /// Per-principal workspace tenant id (UUID stringified). Decoded
    /// request-side by [`super::resolve_tenant::resolve_tenant_for_request`]
    /// via the JWT `tnt` branch, which wins over the cwd fallback — so two
    /// demo-mint principals resolve to DIFFERENT tenants (isolated rosters)
    /// instead of all sharing the one cwd/seeded tenant.
    ///
    /// `None` only when the gateway has no `SessionStore` to provision
    /// against (not reachable in the running gateway); the resolver then
    /// falls back to the cwd tenant exactly as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    tnt: Option<&'a str>,
    /// Membership-equivalent role. Decoded verify-side by
    /// [`super::session_jwt`]'s `SessionClaims.role` and consumed by
    /// the role-gate. Demo principals are workspace owners — see
    /// [`DEMO_PRINCIPAL_ROLE`].
    role: &'a str,
    iat: i64,
    exp: i64,
    demo: bool,
}

/// Dispatch entry-point. Returns `None` when demo mode is off or
/// the request isn't the mint route; the caller falls through to
/// the normal route table (which serves the path as a 404).
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !is_enabled() {
        return None;
    }
    if !is_mint_principal_request(req) {
        return None;
    }
    Some(mint_principal_response(req, store).await)
}

async fn mint_principal_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let parsed: MintBody =
        match serde_json::from_str(crate::handlers::gateway::body_from_request(req)) {
            Ok(v) => v,
            Err(e) => return error_resp("400 Bad Request", &format!("invalid body: {e}")),
        };
    let email = parsed.email.trim().to_ascii_lowercase();
    if !looks_like_email(&email) {
        return error_resp("400 Bad Request", "email is required");
    }
    let now = Utc::now().timestamp();
    // Give this demo principal its OWN tenant so two demo-mint logins
    // never share a roster. Mirrors the magic-link verify path (upsert
    // user → provision/look-up tenant → stamp `tnt`); idempotent, so a
    // repeat mint of the same email reuses its tenant rather than minting
    // a second one. Without a store we can't provision, so we mint a
    // `tnt`-less token and the resolver keeps its prior cwd fallback.
    let tenant_id = match store {
        Some(s) => match resolve_demo_tenant(s, &email).await {
            Ok(t) => t,
            Err(msg) => return error_resp("500 Internal Server Error", &msg),
        },
        None => None,
    };
    let token = match mint_demo_session(&email, tenant_id.as_deref(), now) {
        Ok(t) => t,
        Err(msg) => return error_resp("500 Internal Server Error", &msg),
    };
    tracing::warn!(
        target: "argo_cli::demo_mode",
        email = %email,
        "demo-mode magic-link bypass minted principal for {email}",
    );
    if let Some(s) = store {
        write_demo_audit(s, &email).await;
    }
    let principal = format!("human:{email}");
    let body = json!({
        "ok": true,
        "principal": principal,
        "session_token": token,
    });
    ("200 OK", body.to_string())
}

fn mint_demo_session(email: &str, tenant_id: Option<&str>, now: i64) -> Result<String, String> {
    let key = EncodingKey::from_secret(&jwt_secret());
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: email,
            email,
            tnt: tenant_id,
            role: DEMO_PRINCIPAL_ROLE,
            iat: now,
            exp: now + JWT_TTL_SECS,
            demo: true,
        },
        &key,
    )
    .map_err(|e| format!("jwt encode: {e}"))
}

/// Provision (or look up) this demo principal's own tenant and return
/// its id as a string for the JWT `tnt` claim. Makes the demo principal
/// exactly a real first-time signed-up user: a `user_account` row, an
/// owned `tenant` + `workspace`, and a `user_tenant` owner link.
///
/// Mirrors `auth_magic_link_verify`'s upsert-user → provision-tenant
/// path against the same `argo_identity` API (that module's helpers are
/// private, so this duplicates the few lines rather than coupling to
/// them — the same pattern `onboarding_provision` already follows).
/// [`provision_for_user`] is idempotent on `user_id`, so a repeat mint
/// of the same email reuses its tenant instead of minting a second one.
async fn resolve_demo_tenant(
    store: &Arc<SessionStore>,
    email: &str,
) -> Result<Option<String>, String> {
    use argo_identity::user::UserAccount;
    use argo_identity::user_store::{get_user_by_email, insert_user, update_last_login};
    use argo_identity::{provision_for_user, ProvisionRequest};

    let user = match get_user_by_email(store, email)
        .await
        .map_err(|e| format!("demo user lookup: {e}"))?
    {
        Some(u) => {
            update_last_login(store, u.id, Utc::now())
                .await
                .map_err(|e| format!("demo user touch: {e}"))?;
            u
        }
        None => {
            let u = UserAccount::new(email.to_string());
            insert_user(store, &u)
                .await
                .map_err(|e| format!("demo user insert: {e}"))?;
            u
        }
    };

    let workspace_name = demo_workspace_name(email);
    let outcome = provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: email,
            workspace_name: &workspace_name,
            company_name: None,
        },
    )
    .await
    .map_err(|e| format!("demo tenant provision: {e}"))?;
    Ok(Some(outcome.tenant_id.to_string()))
}

/// A non-empty default workspace label from the email local-part
/// (`alice@acme.com` → `alice's Workspace`). Only needs to be non-empty
/// so [`provision_for_user`] accepts it; the demo user can rename it
/// later. Mirrors `auth_magic_link_verify::default_workspace_name`.
fn demo_workspace_name(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("").trim();
    if local.is_empty() {
        "My Workspace".to_string()
    } else {
        format!("{local}'s Workspace")
    }
}

async fn write_demo_audit(store: &Arc<SessionStore>, email: &str) {
    let payload = json!({
        "method": "demo_mode_bypass",
        "demo_mode_bypass": true,
        "email": email,
    });
    let sink = AuditSink::new_with_tenant(
        Arc::clone(store),
        ANONYMOUS_AUDIT_TENANT_ID.to_string(),
    );
    if let Err(e) = sink
        .write(None, AuditKind::AuthSucceeded, payload, None)
        .await
    {
        tracing::warn!(
            target: "argo_cli::demo_mode",
            error = %e,
            "demo-mode audit write failed",
        );
    }
}

fn error_resp(status: &'static str, msg: &str) -> (&'static str, String) {
    (status, json!({ "error": msg }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_mint_principal_request_matches_both_forms() {
        assert!(is_mint_principal_request(
            "POST /api/v1/demo/mint-principal HTTP/1.1\r\n\r\n"
        ));
        assert!(is_mint_principal_request(
            "POST /api/v1/demo/mint-principal?source=spa HTTP/1.1\r\n\r\n"
        ));
        assert!(!is_mint_principal_request(
            "POST /api/v1/demo/other HTTP/1.1\r\n\r\n"
        ));
        assert!(!is_mint_principal_request(
            "GET /api/v1/demo/mint-principal HTTP/1.1\r\n\r\n"
        ));
    }

    #[test]
    fn mint_demo_session_round_trips() {
        let jwt = mint_demo_session(
            "demo@example.com",
            Some("11111111-1111-1111-1111-111111111111"),
            1_700_000_000,
        )
        .expect("mint");
        let key = jsonwebtoken::DecodingKey::from_secret(&jwt_secret());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = false;
        #[derive(Debug, serde::Deserialize)]
        struct Verify {
            email: String,
            demo: bool,
            role: String,
            tnt: Option<String>,
        }
        let data = jsonwebtoken::decode::<Verify>(&jwt, &key, &validation).expect("decode");
        assert_eq!(data.claims.email, "demo@example.com");
        assert!(data.claims.demo);
        // Regression: the demo JWT MUST carry a role claim. A missing
        // role decoded `None` verify-side, fell back to `guest`, and
        // 403'd the demo "Run workflow" POST.
        assert_eq!(data.claims.role, "owner");
        // Tenant-isolation fix: the demo JWT MUST carry a `tnt` claim so
        // `resolve_tenant_for_request` routes the principal to its own
        // tenant via the JWT branch instead of the shared cwd fallback.
        assert_eq!(
            data.claims.tnt.as_deref(),
            Some("11111111-1111-1111-1111-111111111111"),
        );
    }

    #[test]
    fn mint_demo_session_omits_tnt_when_unprovisioned() {
        // No store → no provisioning → no `tnt` claim, so the resolver
        // keeps its prior cwd fallback (graceful degrade, not a panic).
        let jwt = mint_demo_session("demo@example.com", None, 1_700_000_000).expect("mint");
        let key = jsonwebtoken::DecodingKey::from_secret(&jwt_secret());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = false;
        #[derive(Debug, serde::Deserialize)]
        struct Verify {
            tnt: Option<String>,
        }
        let data = jsonwebtoken::decode::<Verify>(&jwt, &key, &validation).expect("decode");
        assert_eq!(data.claims.tnt, None);
    }

    /// End-to-end of the fix: a demo-minted JWT, decoded through the
    /// *real* verify path (`session_jwt::verify_session_jwt`), surfaces
    /// `role = "owner"`. That role is what `routes_workflow_execute`'s
    /// `resolve_principal` reads, and `owner` holds `workflow.edit` (via
    /// the `*` `ToolDispatch` glob), so the `workflow_edit_capability()`
    /// gate passes instead of 403-ing the demo principal.
    #[test]
    fn demo_jwt_resolves_to_owner_and_passes_workflow_edit_capability() {
        use argo_capability::{workflow_edit_capability, CapabilityEngine};
        use argo_types::Principal;

        // 1. Mint a demo session token (the demo-mode bypass path).
        let jwt = mint_demo_session(
            "portia@skycot.com",
            Some("22222222-2222-2222-2222-222222222222"),
            Utc::now().timestamp(),
        )
        .expect("mint");

        // 2. Decode it through the production verify-side parser the
        //    role-gate actually uses — proves the `role` claim survives
        //    the `SessionClaims` round-trip and is not silently dropped.
        let bearer_header = format!(
            "POST /api/v1/verticals/customer_research/workflows/customer_research_followup/execute HTTP/1.1\r\nAuthorization: Bearer {jwt}\r\n\r\n"
        );
        let claims = crate::handlers::gateway::session_jwt::extract_session_claims(&bearer_header)
            .expect("demo JWT must verify through the production session-jwt parser");
        assert_eq!(
            claims.role.as_deref(),
            Some("owner"),
            "demo principal must resolve to owner, not fall through to guest"
        );

        // 3. The resolved role must pass the exact capability the
        //    workflow-execute route checks. `owner` is granted the `*`
        //    ToolDispatch glob in the default role presets, which covers
        //    `workflow.edit`.
        let engine = CapabilityEngine::with_defaults();
        let principal = Principal::Gateway {
            adapter: "human".into(),
            identity: "portia@skycot.com".into(),
        };
        engine
            .bind(principal.clone(), claims.role.clone().unwrap())
            .expect("bind owner role");
        engine
            .check(&principal, &workflow_edit_capability())
            .expect("owner must hold workflow.edit — the demo Run-now gate");

        // 4. Negative control: the prior behaviour (no role → guest)
        //    still 403s. Guest holds workflow.read but NOT workflow.edit.
        let guest_principal = Principal::Gateway {
            adapter: "human".into(),
            identity: "guest@example.com".into(),
        };
        engine
            .bind(guest_principal.clone(), "guest".into())
            .expect("bind guest role");
        assert!(
            engine
                .check(&guest_principal, &workflow_edit_capability())
                .is_err(),
            "the workflow-execute gate must still refuse a guest — the fix is scoped to the demo principal, not a blanket disable"
        );
    }
}
