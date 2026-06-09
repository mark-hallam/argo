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

/// Persist a transcript turn for a given tenant via the live write path
/// (which stamps `payload.tenant_id` + the `audit_record.tenant_id` column).
async fn persist_turn(store: &Arc<SessionStore>, tenant: &str, sid: &str, text: &str) {
    let body = json!({ "session_id": sid, "role": "user", "text": text, "ts_millis": 1, "mode": "coach" });
    let req = format!(
        "POST /api/v1/voice/transcript HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n{body}"
    );
    let (status, _) = transcript_turn_response(&req, Some(store)).await;
    assert!(status.starts_with("201"), "write turn for {tenant}: {status}");
}

async fn list_sessions(store: &Arc<SessionStore>, tenant: &str) -> (String, Value) {
    let get = format!(
        "GET /api/v1/voice/sessions HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n"
    );
    let (status, body) = voice_sessions_response(&get, Some(store)).await;
    assert_eq!(status, "200 OK", "list for {tenant}");
    let v: Value = serde_json::from_str(&body).expect("json");
    (body, v)
}

#[tokio::test]
async fn sessions_list_is_tenant_scoped_no_cross_tenant_bleed() {
    // Two tenants persist transcript turns into ONE shared store. Each
    // tenant's GET /api/v1/voice/sessions must return ONLY its own sessions
    // and must NOT leak the other tenant's session_id or text. This pins the
    // isolation guarantee (now enforced at the SQL level via the tenant
    // predicate, with the in-memory group_sessions check as belt-and-braces)
    // against a future refactor that drops either layer.
    let store = mem_store().await;
    persist_turn(&store, "ws-a", "a-session-1", "alpha tenant secret moat").await;
    persist_turn(&store, "ws-a", "a-session-2", "alpha second conversation").await;
    persist_turn(&store, "ws-b", "b-session-1", "bravo tenant private numbers").await;

    // Tenant A sees only A's two sessions; none of B's id/text.
    let (body_a, v_a) = list_sessions(&store, "ws-a").await;
    let sessions_a = v_a["sessions"].as_array().expect("a sessions array");
    assert_eq!(sessions_a.len(), 2, "ws-a sees its 2 sessions: {body_a}");
    assert!(sessions_a.iter().all(|s| s["session_id"] == "a-session-1" || s["session_id"] == "a-session-2"));
    assert!(!body_a.contains("b-session-1"), "B's session_id must not leak to A: {body_a}");
    assert!(!body_a.contains("bravo tenant private numbers"), "B's text must not leak to A: {body_a}");

    // Tenant B sees only B's one session; none of A's id/text.
    let (body_b, v_b) = list_sessions(&store, "ws-b").await;
    let sessions_b = v_b["sessions"].as_array().expect("b sessions array");
    assert_eq!(sessions_b.len(), 1, "ws-b sees its 1 session: {body_b}");
    assert_eq!(sessions_b[0]["session_id"], "b-session-1");
    assert!(!body_b.contains("a-session-1"), "A's session_id must not leak to B: {body_b}");
    assert!(!body_b.contains("a-session-2"), "A's session_id must not leak to B: {body_b}");
    assert!(!body_b.contains("alpha tenant secret moat"), "A's text must not leak to B: {body_b}");
}
