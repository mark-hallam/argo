//! Smoke tests for the write-side gateway endpoints —
//! `POST /skills`, `POST /memory`. Pulled out of
//! `gateway_smoke_v2.rs` so each integration-test binary stays
//! under the 500-LOC ceiling.
mod common;
use common::{body_of, boot, http_post, pick_free_port};

#[test]
fn skill_install_returns_400_on_missing_raw_and_201_on_valid_doc() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/skills", "{}");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let raw = "---\\nname: zeta\\nversion: \\\"0.1\\\"\\ndescription: D\\nlock: none\\n\
               provenance:\\n  source: smoke\\n  hash: sha256-x\\n---\\nbody-of-zeta";
    let body = format!("{{\"raw\": \"{raw}\"}}");
    let resp = http_post(port, "/skills", &body);
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn memory_append_returns_400_on_unknown_file_and_auto_bootstraps_workspace() {
    // #718: invalid file rejected with 400; valid append now
    // auto-bootstraps the cwd-resolved workspace via ensure_workspace
    // (matches every CLI writer's path) instead of 400'ing on a
    // missing first_workspace row.
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/memory", r#"{"file":"bogus","body":"x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(port, "/memory", r#"{"file":"memory","body":"hello"}"#);
    assert!(
        resp.contains("HTTP/1.1 201 Created"),
        "valid memory append must auto-bootstrap workspace; resp={resp}"
    );
    let body = body_of(&resp);
    assert!(
        body.contains("\"id\""),
        "201 body should carry the new entry id; body={body}"
    );

    assert!(child.wait().expect("wait").success());
}

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

#[test]
fn skill_promote_returns_400_on_missing_version_and_unknown_skill() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    // Missing version → 400.
    let resp = http_post(port, "/skills/anything/promote", r#"{}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("missing or empty `version`"), "body={body}");

    // Unknown skill → 400 from SkillStore::promote.
    let resp = http_post(
        port,
        "/skills/never-installed/promote",
        r#"{"version":"0.1"}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("promote"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn memory_lock_validates_id_and_unknown_lock_label() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    let resp = http_post(port, "/memory/not-a-uuid/lock", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Bogus lock label → 400.
    let resp = http_post(
        port,
        "/memory/00000000-0000-0000-0000-000000000000/lock",
        r#"{"lock":"super-secret"}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("unknown lock"), "body={body}");

    // `lock=none` rejected on /lock — caller must use /unlock.
    let resp = http_post(
        port,
        "/memory/00000000-0000-0000-0000-000000000000/lock",
        r#"{"lock":"none"}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn memory_edit_returns_400_on_missing_body_and_unknown_id() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    // Bad uuid → 400.
    let resp = http_post(port, "/memory/not-a-uuid/edit", r#"{"body":"x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Missing body → 400.
    let resp = http_post(
        port,
        "/memory/00000000-0000-0000-0000-000000000000/edit",
        r#"{}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid with body but no entry exists → 400 from supersede.
    let resp = http_post(
        port,
        "/memory/00000000-0000-0000-0000-000000000000/edit",
        r#"{"body":"hello"}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn memory_forget_returns_400_on_bad_id_and_unknown_uuid() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/memory/not-a-uuid/forget", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid that isn't in the DB → 400 from MemoryStore::get
    // (no row matches).
    let resp = http_post(
        port,
        "/memory/00000000-0000-0000-0000-000000000000/forget",
        "",
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("forget"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn judge_override_returns_400_on_missing_to_and_404_on_unknown_record() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    // Bad uuid → 400.
    let resp = http_post(port, "/judge/not-a-uuid/override", r#"{"to":"success"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Missing `to` → 400.
    let resp = http_post(
        port,
        "/judge/00000000-0000-0000-0000-000000000000/override",
        r#"{}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid, valid body, no row → 404.
    let resp = http_post(
        port,
        "/judge/00000000-0000-0000-0000-000000000000/override",
        r#"{"to":"success"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn db_vacuum_and_prune_return_200() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    // Prune defaults to 30 days → 0 pruned on a clean store.
    let resp = http_post(port, "/db/prune", "");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(body_of(&resp).contains(r#""pruned":0"#));

    // Negative older_than_days → 400.
    let resp = http_post(port, "/db/prune", r#"{"older_than_days":-1}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Vacuum a clean store → 200.
    let resp = http_post(port, "/db/vacuum", "");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert!(body_of(&resp).contains(r#""vacuumed":true"#));

    assert!(child.wait().expect("wait").success());
}

#[test]
fn workspace_init_validates_name_and_creates_row() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 3);

    // Missing name → 400.
    let resp = http_post(port, "/workspace/init", r#"{"root_path":"/tmp/x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Path-traversal in name → 400.
    let resp = http_post(
        port,
        "/workspace/init",
        r#"{"name":"../escape","root_path":"/tmp/x"}"#,
    );
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid → 201 + workspace_id.
    let resp = http_post(
        port,
        "/workspace/init",
        r#"{"name":"smoke-ws","root_path":"/tmp/smoke-ws"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("workspace_id"), "body={body}");
    assert!(body.contains("smoke-ws"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn session_end_returns_400_on_bad_id_and_404_on_missing() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/sessions/not-a-uuid/end", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/end",
        "",
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn session_title_returns_400_on_bad_id_and_404_on_missing() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/sessions/not-a-uuid/title", r#"{"title":"x"}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/title",
        r#"{"title":"renamed"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn session_delete_returns_400_on_bad_id_and_404_on_missing() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    // Bad uuid → 400.
    let resp = http_post(port, "/sessions/not-a-uuid/delete", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Valid uuid that isn't in the DB → 404.
    let resp = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/delete",
        "",
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("no session"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn session_skills_endpoints_validate_session_id_and_unknown_skill() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 4);

    // GET on bad uuid → 400.
    let resp = http_get_v(port, "/sessions/not-a-uuid/skills");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // GET on valid uuid with no rows → empty array.
    let resp = http_get_v(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/skills",
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    assert_eq!(body_of(&resp), "[]");

    // POST attach with unknown skill → 404.
    let resp = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/skills/attach",
        r#"{"skill":"never-installed"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    // POST detach with unknown skill → 404.
    let resp = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/skills/detach",
        r#"{"skill":"never-installed"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

fn http_get_v(port: u16, path: &str) -> String {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn skill_refine_returns_400_on_missing_hint_and_404_on_unknown_skill() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    // Missing hint → 400.
    let resp = http_post(port, "/skills/anything/refine", r#"{}"#);
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    // Unknown skill → 404.
    let resp = http_post(
        port,
        "/skills/never-installed/refine",
        r#"{"hint":"be terser"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn skill_lock_returns_404_on_unknown_skill() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/skills/never-installed/lock", "");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    let resp = http_post(port, "/skills/never-installed/unlock", "");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp={resp}");

    assert!(child.wait().expect("wait").success());
}

// session_skills_clear / forge_runs_abort_all / forge_runs_sweep
// tests live in `gateway_smoke_admin.rs` so this file stays under
// the 500-LOC budget.
