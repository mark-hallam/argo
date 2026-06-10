//! Unit tests for `routes_voice_realtime`. Included via `#[path]` from
//! `routes_voice_realtime.rs` so the tests stay private to the crate while
//! keeping that file under the 500-LOC ceiling.

use super::*;

#[test]
fn parse_channel_maps_known_and_defaults_rehearsal() {
    assert!(matches!(parse_channel(Some("WebEmbed")), PitchChannel::WebEmbed));
    assert!(matches!(parse_channel(Some("LiveMeeting")), PitchChannel::LiveMeeting));
    assert!(matches!(parse_channel(Some("Chat")), PitchChannel::Chat));
    assert!(matches!(parse_channel(Some("  Rehearsal  ")), PitchChannel::Rehearsal));
    assert!(matches!(parse_channel(None), PitchChannel::Rehearsal));
    assert!(matches!(parse_channel(Some("bogus")), PitchChannel::Rehearsal));
}

#[tokio::test]
async fn bad_body_is_400() {
    let req = "POST /api/v1/voice/realtime-session HTTP/1.1\r\nContent-Type: application/json\r\n\r\nnot json";
    let (status, _body) = realtime_session_response(req, None).await;
    assert!(status.starts_with("400"), "expected 400, got {status}");
}

#[test]
fn coach_and_interview_are_grounded_in_company_facts() {
    // Cora's coach/interview voices must carry the SAME canonical SKYCOT facts
    // Portia's pitch uses — else she coaches generically and can't catch an
    // over-claim. (`load_cached` reads repo facts in tests, like the pitch-facts tests.)
    let coach = ground_in_skycot(COACH_INSTRUCTIONS);
    assert!(coach.contains("Cora"), "persona preserved");
    assert!(
        coach.contains("What you know about SKYCOT"),
        "company facts section must be appended (len {})",
        coach.len()
    );
    assert!(
        coach.len() > COACH_INSTRUCTIONS.len() + 200,
        "facts must materially expand the prompt"
    );
    assert!(coach.contains("underscore"), "spoken-delivery rule present");
    assert!(ground_in_skycot(INTERVIEW_INSTRUCTIONS).contains("What you know about SKYCOT"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

#[tokio::test]
async fn transcript_turns_persist_and_sessions_group_by_session_id() {
    let store = mem_store().await;
    for (sid, role, text) in [
        ("s1", "user", "what is our strongest moat"),
        ("s1", "agent", "the hash-chained tamper-evident audit"),
        ("s2", "user", "hello"),
    ] {
        let body = json!({ "session_id": sid, "role": role, "text": text, "ts_millis": 1, "mode": "coach" });
        let req = format!(
            "POST /api/v1/voice/transcript HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: ws-default\r\n\r\n{body}"
        );
        let (status, _) = transcript_turn_response(&req, Some(&store)).await;
        assert!(status.starts_with("201"), "write turn: {status}");
    }
    let get = "GET /api/v1/voice/sessions?mode=coach HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: ws-default\r\n\r\n";
    let (status, body) = voice_sessions_response(get, Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).expect("json");
    let sessions = v["sessions"].as_array().expect("sessions array");
    assert_eq!(sessions.len(), 2, "two distinct sessions: {body}");
    let s1 = sessions.iter().find(|s| s["session_id"] == "s1").expect("s1 present");
    assert_eq!(s1["turn_count"], 2);
    assert_eq!(s1["turns"].as_array().expect("turns").len(), 2);
}

#[tokio::test]
async fn transcript_turn_rejects_blank_text_and_bad_role() {
    let store = mem_store().await;
    let blank = "POST /api/v1/voice/transcript HTTP/1.1\r\nHost: x\r\n\r\n{\"session_id\":\"s\",\"role\":\"user\",\"text\":\"   \"}";
    assert!(transcript_turn_response(blank, Some(&store)).await.0.starts_with("400"));
    let bad_role = "POST /api/v1/voice/transcript HTTP/1.1\r\nHost: x\r\n\r\n{\"session_id\":\"s\",\"role\":\"bot\",\"text\":\"hi\"}";
    assert!(transcript_turn_response(bad_role, Some(&store)).await.0.starts_with("400"));
}

// --- Tenant-resolution isolation (fix-genesis-intake-tenant-resolution) ------
//
// The transcript write + sessions read must key on the JWT-bound tenant, not a
// client-controlled `X-Tenant-Id` header — otherwise an authed tenant-A caller
// could write into, or read, tenant B's voice history by sending the header.
// These mirror #2710's `resolve_tenant_tests` pattern (real provisioned UUID
// tenants + a `__Host-session` cookie JWT) and use the process-global gateway
// env lock so the `SKYCOT_AUTH_JWT_SECRET` mutation can't be torn across modules.

const VOICE_TEST_SECRET: &str = "voice-realtime-tenant-test-secret";

#[derive(serde::Serialize)]
struct VoiceTestClaims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: Option<String>,
    iat: i64,
    exp: i64,
}

/// Mint a `__Host-session` cookie JWT carrying `tnt = tenant`.
fn mint_session(tenant: &argo_types::TenantId) -> String {
    let now = chrono::Utc::now().timestamp();
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &VoiceTestClaims {
            sub: "11111111-1111-1111-1111-111111111111",
            email: "operator@example.com",
            tnt: Some(tenant.to_string()),
            iat: now - 10,
            exp: now + 3600,
        },
        &jsonwebtoken::EncodingKey::from_secret(VOICE_TEST_SECRET.as_bytes()),
    )
    .expect("mint session jwt")
}

/// Provision a fresh user → returns the tenant they own.
async fn provision(store: &SessionStore, email: &str, ws: &str) -> argo_types::TenantId {
    use argo_identity::user::UserAccount;
    use argo_identity::user_store::insert_user;
    use argo_identity::{provision_for_user, ProvisionRequest};
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await.expect("insert user");
    provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: ws,
            company_name: None,
        },
    )
    .await
    .expect("provision")
    .tenant_id
}

/// Persist a transcript turn as the JWT-bound caller. `override_tenant` adds an
/// `X-Tenant-Id` header that the resolver MUST ignore in favor of the JWT.
async fn persist_turn(
    store: &Arc<SessionStore>,
    jwt: &str,
    override_tenant: Option<&argo_types::TenantId>,
    sid: &str,
    text: &str,
) {
    let header = override_tenant
        .map(|t| format!("X-Tenant-Id: {t}\r\n"))
        .unwrap_or_default();
    let body = json!({ "session_id": sid, "role": "user", "text": text, "ts_millis": 1, "mode": "coach" });
    let req = format!(
        "POST /api/v1/voice/transcript HTTP/1.1\r\nHost: x\r\n\
         Cookie: __Host-session={jwt}\r\n{header}\r\n{body}"
    );
    let (status, _) = transcript_turn_response(&req, Some(store)).await;
    assert!(status.starts_with("201"), "write turn: {status}");
}

/// List sessions as the JWT-bound caller. `override_tenant` adds an
/// `X-Tenant-Id` header that the resolver MUST ignore in favor of the JWT.
async fn list_sessions(
    store: &Arc<SessionStore>,
    jwt: &str,
    override_tenant: Option<&argo_types::TenantId>,
) -> (String, Value) {
    let header = override_tenant
        .map(|t| format!("X-Tenant-Id: {t}\r\n"))
        .unwrap_or_default();
    let get = format!(
        "GET /api/v1/voice/sessions HTTP/1.1\r\nHost: x\r\n\
         Cookie: __Host-session={jwt}\r\n{header}\r\n"
    );
    let (status, body) = voice_sessions_response(&get, Some(store)).await;
    assert_eq!(status, "200 OK", "list sessions");
    let v: Value = serde_json::from_str(&body).expect("json");
    (body, v)
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn sessions_list_is_tenant_scoped_no_cross_tenant_bleed() {
    // Two tenants persist transcript turns into ONE shared store, each as its
    // own JWT-bound caller. Each tenant's GET /api/v1/voice/sessions must
    // return ONLY its own sessions and must NOT leak the other tenant's
    // session_id or text — AND an authed tenant-A caller must not be able to
    // read tenant B by sending `X-Tenant-Id: B` (the resolution fix).
    let _g = crate::handlers::gateway::shared_env_lock().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", VOICE_TEST_SECRET);

    let store = mem_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme A").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta B").await;
    let jwt_a = mint_session(&tenant_a);
    let jwt_b = mint_session(&tenant_b);

    persist_turn(&store, &jwt_a, None, "a-session-1", "alpha tenant secret moat").await;
    persist_turn(&store, &jwt_a, None, "a-session-2", "alpha second conversation").await;
    persist_turn(&store, &jwt_b, None, "b-session-1", "bravo tenant private numbers").await;

    // Tenant A sees only A's two sessions; none of B's id/text.
    let (body_a, v_a) = list_sessions(&store, &jwt_a, None).await;
    let sessions_a = v_a["sessions"].as_array().expect("a sessions array");
    assert_eq!(sessions_a.len(), 2, "ws-a sees its 2 sessions: {body_a}");
    assert!(sessions_a.iter().all(|s| s["session_id"] == "a-session-1" || s["session_id"] == "a-session-2"));
    assert!(!body_a.contains("b-session-1"), "B's session_id must not leak to A: {body_a}");
    assert!(!body_a.contains("bravo tenant private numbers"), "B's text must not leak to A: {body_a}");

    // Tenant B sees only B's one session; none of A's id/text.
    let (body_b, v_b) = list_sessions(&store, &jwt_b, None).await;
    let sessions_b = v_b["sessions"].as_array().expect("b sessions array");
    assert_eq!(sessions_b.len(), 1, "ws-b sees its 1 session: {body_b}");
    assert_eq!(sessions_b[0]["session_id"], "b-session-1");
    assert!(!body_b.contains("a-session-1"), "A's session_id must not leak to B: {body_b}");
    assert!(!body_b.contains("alpha tenant secret moat"), "A's text must not leak to B: {body_b}");

    // THE FIX — tenant A with `X-Tenant-Id: B` still reads ONLY A: the JWT wins
    // over the client header, so the cross-tenant read is denied.
    let (body_ab, v_ab) = list_sessions(&store, &jwt_a, Some(&tenant_b)).await;
    let sessions_ab = v_ab["sessions"].as_array().expect("a-with-b-header array");
    assert_eq!(sessions_ab.len(), 2, "A+X-Tenant-Id:B still sees only A's 2 sessions: {body_ab}");
    assert!(!body_ab.contains("b-session-1"), "X-Tenant-Id:B must NOT surface B's session to A: {body_ab}");
    assert!(!body_ab.contains("bravo tenant private numbers"), "B's text must not leak via header: {body_ab}");

    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
}
