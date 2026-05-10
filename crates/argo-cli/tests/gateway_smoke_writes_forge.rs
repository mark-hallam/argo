//! Smoke tests for the gateway's forge-side write endpoints —
//! `POST /forge/optimize`, `POST /forge/bench`,
//! `POST /forge/runs/<id>/promote`, `POST /forge/runs/<id>/abort`.
//! Lifted out of `gateway_smoke_writes.rs` so each integration-test
//! binary stays under the 500-LOC ceiling.
mod common;
use common::{body_of, boot, http_post, pick_free_port};

#[test]
fn forge_optimize_post_validates_body_and_returns_202_on_kick_off() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    // Missing skill → 400
    let resp = http_post(port, "/forge/optimize", r#"{"examples":"foo.jsonl"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Missing examples → 400
    let resp = http_post(port, "/forge/optimize", r#"{"skill":"x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid body → 202 Accepted. Default path enqueues against
    // forge_queue (since #581); the daemon claims it later, so
    // the response carries `enqueued:true` + `started:false` +
    // `mode:queued`. The synchronous reply only acknowledges
    // the enqueue, not a running task.
    let resp = http_post(
        port,
        "/forge/optimize",
        r#"{"skill":"missing-skill","examples":"missing.jsonl","iterations":1,"budget_usd":0.5}"#,
    );
    assert!(resp.contains("HTTP/1.1 202 Accepted"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains(r#""enqueued":true"#), "body={body}");
    assert!(body.contains(r#""started":false"#), "body={body}");
    assert!(body.contains(r#""mode":"queued""#), "body={body}");
    assert!(body.contains(r#""kind":"optimize""#), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_bench_post_returns_202_with_provider_defaults() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/forge/bench", r#"{}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(
        port,
        "/forge/bench",
        r#"{"skill":"missing-skill","examples":"missing.jsonl"}"#,
    );
    assert!(resp.contains("HTTP/1.1 202 Accepted"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains(r#""kind":"bench""#), "body={body}");
    assert!(body.contains(r#""provider":"anthropic""#), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_promote_returns_400_on_bad_uuid_and_404_on_missing_run() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/forge/runs/abcdef12/promote", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(
        port,
        "/forge/runs/00000000-0000-0000-0000-000000000000/promote",
        "",
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_abort_returns_400_on_short_id_and_404_on_missing_uuid() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    // Short id (8 hex) — rejected at parse step.
    let resp = http_post(port, "/forge/runs/abcdef12/abort", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid that isn't in the DB — 404.
    let resp = http_post(
        port,
        "/forge/runs/00000000-0000-0000-0000-000000000000/abort",
        "",
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("no forge run"), "body={body}");

    assert!(child.wait().expect("wait").success());
}
