//! Integration tests for the live skill-trace WS handler (L5).
//!
//! Drive the per-topic broadcast registry directly so we don't have
//! to stand up a real WS handshake. The runtime-side trace hook is
//! installed by [`subscribe_topic`]; the four scenarios cover the
//! brief's acceptance criteria:
//!
//! 1. Synthetic run emits trace events through to a subscriber.
//! 2. Tenant-scoped isolation — worker A's run does not leak to
//!    worker B's view.
//! 3. Disconnect/reconnect handled cleanly (a dropped subscriber
//!    leaves the topic in a clean state for the next subscriber).
//! 4. Topic isolation — concurrent runs of the same skill across
//!    distinct `run_id`s don't cross-leak.

use super::*;
use argo_agent_runner::instrumentation::ir_trace::{
    self, emit_node_completed, emit_node_entered, TraceEvent, TraceEventKind,
};
use serde_json::Value;

#[test]
fn topic_path_parses_skill_and_run_segments() {
    let req = "GET /skill-trace/skill-foo/run-bar HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    let parsed = parse_topic_path(req).expect("valid path parses");
    assert_eq!(parsed, ("skill-foo".into(), "run-bar".into()));

    let with_query = "GET /skill-trace/sf/rb?token=x HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    let parsed = parse_topic_path(with_query).expect("query string ignored");
    assert_eq!(parsed, ("sf".into(), "rb".into()));

    let no_run = "GET /skill-trace/sf HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert!(parse_topic_path(no_run).is_none());

    let too_deep = "GET /skill-trace/sf/rb/extra HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert!(parse_topic_path(too_deep).is_none());

    let wrong_root = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert!(parse_topic_path(wrong_root).is_none());
}

#[test]
fn hello_frame_carries_topic_and_capabilities() {
    let body = hello_frame("skill-A", "run-1");
    let parsed: Value = serde_json::from_str(&body).expect("hello is JSON");
    assert_eq!(parsed["type"], "hello");
    assert_eq!(parsed["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(parsed["topic"], "skill-trace.skill-A.run-1");
    assert_eq!(parsed["seq"], 0);
    let caps = parsed["capabilities"]
        .as_array()
        .expect("capabilities array");
    let names: Vec<&str> = caps.iter().filter_map(Value::as_str).collect();
    assert!(names.contains(&"node.entered"));
    assert!(names.contains(&"node.completed"));
    assert!(names.contains(&"node.failed"));
}

#[test]
fn authorize_rejects_missing_token() {
    let req = "GET /skill-trace/sf/rb HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    let now = chrono::Utc::now().timestamp();
    let err = authorize(req, now).expect_err("missing token -> 401");
    assert_eq!(err.0, "401 Unauthorized");
    assert!(err.1.contains("missing JWT token"));
}

#[tokio::test]
async fn synthetic_run_emits_node_lifecycle_through_to_subscriber() {
    // Subscribing first registers the trace hook + creates the
    // per-topic broadcaster. Subsequent `ir_trace::emit_*` calls
    // route through the hook into the subscriber receiver.
    let mut rx = subscribe_topic("tenant-syn", "skill-syn", "run-syn");
    emit_node_entered("tenant-syn", "skill-syn", "run-syn", "n1");
    emit_node_completed(
        "tenant-syn",
        "skill-syn",
        "run-syn",
        "n1",
        Some(42),
    );

    let entered = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("entered event fires")
        .expect("frame");
    let completed = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("completed event fires")
        .expect("frame");

    let entered_json: Value = serde_json::from_str(&entered).expect("entered is JSON");
    let completed_json: Value =
        serde_json::from_str(&completed).expect("completed is JSON");

    assert_eq!(entered_json["type"], "node.entered");
    assert_eq!(entered_json["node_id"], "n1");
    assert_eq!(completed_json["type"], "node.completed");
    assert_eq!(completed_json["node_id"], "n1");
    assert_eq!(completed_json["duration_ms"], 42);
    assert!(entered_json["seq"].as_u64().unwrap() < completed_json["seq"].as_u64().unwrap());
    assert!(entered_json["ts"].is_string());
}

#[tokio::test]
async fn per_tenant_isolation_no_cross_leak() {
    // Two tenants on the SAME (skill, run) pair must still be isolated
    // because the broadcast key includes tenant_id. Worker A's run
    // events stay on tenant A's subscriber.
    let mut rx_a = subscribe_topic("tenant-iso-a", "skill-iso", "run-iso");
    let mut rx_b = subscribe_topic("tenant-iso-b", "skill-iso", "run-iso");

    ir_trace::dispatch_event(TraceEvent {
        tenant_id: "tenant-iso-a".into(),
        skill_id: "skill-iso".into(),
        run_id: "run-iso".into(),
        node_id: "node-A".into(),
        kind: TraceEventKind::NodeEntered,
    });
    ir_trace::dispatch_event(TraceEvent {
        tenant_id: "tenant-iso-b".into(),
        skill_id: "skill-iso".into(),
        run_id: "run-iso".into(),
        node_id: "node-B".into(),
        kind: TraceEventKind::NodeEntered,
    });

    let a = tokio::time::timeout(std::time::Duration::from_millis(50), rx_a.recv())
        .await
        .expect("A receives")
        .expect("frame A");
    let b = tokio::time::timeout(std::time::Duration::from_millis(50), rx_b.recv())
        .await
        .expect("B receives")
        .expect("frame B");

    assert!(a.contains("node-A"), "A frame: {a}");
    assert!(!a.contains("node-B"), "A leaked B node: {a}");
    assert!(b.contains("node-B"), "B frame: {b}");
    assert!(!b.contains("node-A"), "B leaked A node: {b}");
    assert!(rx_a.try_recv().is_err());
    assert!(rx_b.try_recv().is_err());
}

#[tokio::test]
async fn disconnect_reconnect_handled_cleanly() {
    // A subscriber drop must not poison the topic for the next
    // subscriber. After the original receiver goes out of scope,
    // re-subscribing returns a fresh receiver that observes a new
    // publish without seeing the stale pre-drop frame.
    {
        let mut rx = subscribe_topic("tenant-dc", "skill-dc", "run-dc");
        emit_node_entered("tenant-dc", "skill-dc", "run-dc", "n-pre");
        let pre = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv())
            .await
            .expect("pre event fires")
            .expect("frame");
        let pre_json: Value = serde_json::from_str(&pre).expect("pre is JSON");
        assert_eq!(pre_json["node_id"], "n-pre");
        // `rx` drops at the end of this block; the broadcast sender
        // stays alive in the registry, so the next subscribe is
        // structurally identical to a fresh page-load reconnect.
    }

    let mut rx2 = subscribe_topic("tenant-dc", "skill-dc", "run-dc");
    assert!(
        rx2.try_recv().is_err(),
        "reconnected subscriber must not see pre-disconnect frames"
    );
    emit_node_entered("tenant-dc", "skill-dc", "run-dc", "n-post");
    let post = tokio::time::timeout(std::time::Duration::from_millis(50), rx2.recv())
        .await
        .expect("post event fires")
        .expect("frame");
    let post_json: Value = serde_json::from_str(&post).expect("post is JSON");
    assert_eq!(post_json["node_id"], "n-post");
}

#[tokio::test]
async fn per_run_topic_isolation_no_cross_leak() {
    // Concurrent runs of the same skill must not leak across each
    // other — even within the same tenant, a separate run_id is a
    // distinct topic.
    let mut rx_run_a =
        subscribe_topic("tenant-multi", "skill-multi", "run-A");
    let mut rx_run_b =
        subscribe_topic("tenant-multi", "skill-multi", "run-B");

    emit_node_entered("tenant-multi", "skill-multi", "run-A", "node-only-A");
    emit_node_entered("tenant-multi", "skill-multi", "run-B", "node-only-B");

    let a = tokio::time::timeout(std::time::Duration::from_millis(50), rx_run_a.recv())
        .await
        .expect("run A receives")
        .expect("frame A");
    let b = tokio::time::timeout(std::time::Duration::from_millis(50), rx_run_b.recv())
        .await
        .expect("run B receives")
        .expect("frame B");

    assert!(a.contains("node-only-A"));
    assert!(!a.contains("node-only-B"));
    assert!(b.contains("node-only-B"));
    assert!(!b.contains("node-only-A"));
    assert!(rx_run_a.try_recv().is_err());
    assert!(rx_run_b.try_recv().is_err());
}
