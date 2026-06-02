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
