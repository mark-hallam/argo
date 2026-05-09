//! Steer-channel tests (Amendment 14): channel mechanics + agent-loop
//! integration via `SteerChannel::drain` after a tool result.
//!
//! Loaded as a child of `tests` via `#[path = "tests_steer.rs"] mod`;
//! relies on helpers (`ScriptedTransport`, `fresh_deps`, `run_opts`,
//! `text_response`, `tool_call_response`) defined in `tests.rs`.

use super::*;
use argo_types::SessionId;

#[tokio::test]
async fn steer_channel_round_trips_enqueue_and_drain() {
    let s = SteerChannel::new();
    let sid = SessionId::new();
    s.enqueue(sid, "go faster".into());
    s.enqueue(sid, "and shorter".into());
    assert_eq!(s.depth(sid), 2);
    let drained = s.drain(sid);
    assert_eq!(
        drained,
        vec!["go faster".to_string(), "and shorter".to_string()]
    );
    assert_eq!(s.depth(sid), 0);
    // Second drain on an empty session is empty (slot was removed).
    assert!(s.drain(sid).is_empty());
}

#[tokio::test]
async fn steer_isolated_per_session() {
    let s = SteerChannel::new();
    let a = SessionId::new();
    let b = SessionId::new();
    s.enqueue(a, "for a".into());
    s.enqueue(b, "for b".into());
    assert_eq!(s.drain(a), vec!["for a".to_string()]);
    assert_eq!(s.drain(b), vec!["for b".to_string()]);
}

#[tokio::test]
async fn steer_injects_synthesized_user_message_after_tool_result() {
    // Amendment 14: after a tool result, the agent loop drains
    // pending steers and injects them as a user message BEFORE
    // the next provider call.
    //
    // Script: provider returns a tool call, then returns a final
    // text response. We enqueue a steer between turn start and
    // the first provider response so that, when the tool result
    // returns and the loop drains, the steer is appended.
    let transport = ScriptedTransport::new(vec![
        tool_call_response("file_read", serde_json::json!({"path": "x.txt"})),
        text_response("done"),
    ]);
    let (deps, session_id, ws) = fresh_deps(transport).await;
    // Pre-stage the file the model will "read" so dispatch succeeds.
    tokio::fs::write(ws.join("x.txt"), "hello").await.unwrap();
    // Enqueue the steer up front; the loop drains after the tool
    // result lands.
    deps.steer.enqueue(session_id, "be terse".into());
    let agent = Agent::new(deps);
    let _ = agent
        .run_turn(session_id, "read it".into(), run_opts(ws))
        .await
        .unwrap();
    // Persisted messages: user → assistant(tool_call) → tool →
    // STEER USER → assistant(final) = 5.
    let n = agent.deps().store.message_count(session_id).await.unwrap();
    assert_eq!(n, 5);
    // Channel is drained.
    assert_eq!(agent.deps().steer.depth(session_id), 0);
}
