//! WS1-3 — provision-on-verify coverage.
//!
//! Lives in its own file (rather than the shared
//! `auth_magic_link_tests.rs`) so that test module stays under the
//! 500-LOC per-file ceiling. Asserts that a freshly-verified user with
//! no prior tenant is auto-provisioned their OWN tenant on verify, that
//! the minted session JWT carries that tenant as its `tnt` claim (so the
//! per-request resolver never falls back to the cwd/demo tenant), and
//! that a re-verify is idempotent (no duplicate tenant).

use super::super::sha256_hex;
use super::verify_outcome;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("in-memory store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn verify_req(raw_token: &str) -> String {
    let body = format!(r#"{{"token":"{raw_token}"}}"#);
    format!(
        "POST /api/v1/auth/verify HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Pull the raw `__Host-session` JWT out of the verify outcome's
/// `set_cookie` string (`__Host-session=<jwt>; Max-Age=…\r\nSet-Cookie: …`).
fn session_jwt_from_cookie(set_cookie: &str) -> String {
    set_cookie
        .strip_prefix("__Host-session=")
        .and_then(|rest| rest.split(';').next())
        .expect("session cookie value")
        .to_string()
}

/// Decode the (unverified) `tnt` claim from a JWT payload segment. The
/// signature is validated by the resolver in production; here we only
/// need to read the claim the verify handler stamped.
fn jwt_tnt_claim(jwt: &str) -> Option<String> {
    use base64::Engine;
    let payload_b64 = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("tnt").and_then(|t| t.as_str()).map(str::to_string)
}

/// Insert a magic link for `email` and run the verify handler against it.
async fn verify_once(
    store: &Arc<SessionStore>,
    email: &str,
    raw_token: &str,
) -> super::VerifyOutcome {
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(store.pool(), &sha256_hex(raw_token), email, now + 900)
        .await
        .expect("insert link");
    verify_outcome(&verify_req(raw_token), Some(store)).await
}

#[tokio::test]
async fn fresh_verify_provisions_own_tenant_and_binds_it_into_session_jwt() {
    use argo_identity::user_store::get_user_by_email;
    use argo_identity::user_tenant_store::list_tenants_for_user;
    use argo_store::team_roster::list_teams;
    use argo_store::team_roster_mutations::create_team;
    use argo_types::TenantId;

    let store = fresh_store().await;

    // Stand in for the cwd/demo workspace the OLD (no-provision) path
    // resolved every fresh signed-in user to: seed it with a team so a
    // leak into demo data would be observable.
    let demo_tenant = TenantId::new();
    create_team(store.pool(), demo_tenant, "Demo Reception", None)
        .await
        .expect("seed demo team");

    // A brand-new stranger verifies for the first time — no tenant yet.
    let outcome = verify_once(&store, "stranger@example.com", "ws1-3-provision-token-aaaaaaaa").await;
    assert!(
        outcome.status.starts_with("200"),
        "{} {}",
        outcome.status,
        outcome.body
    );
    // A brand-new user still needs to finish onboarding...
    assert!(
        outcome.body.contains("\"needs_onboarding\":true"),
        "body={}",
        outcome.body
    );

    // ...but they ALREADY own exactly one tenant — no cwd/demo window.
    let user = get_user_by_email(&store, "stranger@example.com")
        .await
        .unwrap()
        .expect("user row");
    let owned = list_tenants_for_user(&store, user.id).await.unwrap();
    assert_eq!(owned.len(), 1, "fresh verify must provision exactly one tenant");
    let tenant_id = owned[0].tenant_id;
    assert_ne!(
        tenant_id, demo_tenant,
        "the provisioned tenant must not be the demo/cwd tenant"
    );

    // The session cookie's JWT carries THAT tenant as its `tnt` claim, so
    // the per-request resolver resolves the user to their own tenant from
    // the very first data read.
    let cookie = outcome.set_cookie.expect("verify must emit Set-Cookie");
    let jwt = session_jwt_from_cookie(&cookie);
    assert_eq!(
        jwt_tnt_claim(&jwt).as_deref(),
        Some(tenant_id.to_string().as_str()),
        "session JWT `tnt` claim must equal the provisioned tenant"
    );

    // Their private workspace is empty — the demo team never leaks in.
    let roster = list_teams(store.pool(), tenant_id).await.expect("roster");
    assert!(roster.is_empty(), "fresh user's private workspace is empty");
}

#[tokio::test]
async fn re_verify_is_idempotent_and_does_not_reprovision() {
    use argo_identity::user_store::get_user_by_email;
    use argo_identity::user_tenant_store::list_tenants_for_user;

    let store = fresh_store().await;

    // First verify auto-provisions the user's tenant (needs onboarding).
    let first = verify_once(&store, "repeat@example.com", "ws1-3-provision-token-bbbbbbbb").await;
    assert!(first.status.starts_with("200"), "{}", first.status);
    assert!(
        first.body.contains("\"needs_onboarding\":true"),
        "body={}",
        first.body
    );
    let user = get_user_by_email(&store, "repeat@example.com")
        .await
        .unwrap()
        .expect("user row");
    let tenant_id = list_tenants_for_user(&store, user.id).await.unwrap()[0].tenant_id;

    // Second verify (e.g. a re-click): already provisioned → no duplicate,
    // and the user no longer needs onboarding.
    let second = verify_once(&store, "repeat@example.com", "ws1-3-provision-token-cccccccc").await;
    assert!(second.status.starts_with("200"), "{}", second.status);
    assert!(
        second.body.contains("\"needs_onboarding\":false"),
        "body={}",
        second.body
    );
    let owned = list_tenants_for_user(&store, user.id).await.unwrap();
    assert_eq!(owned.len(), 1, "re-verify must not create a second tenant");
    assert_eq!(owned[0].tenant_id, tenant_id, "same tenant on re-verify");
}
