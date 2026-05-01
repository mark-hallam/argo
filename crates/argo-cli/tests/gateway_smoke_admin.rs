//! Smoke tests for the gateway's admin/recovery POSTs —
//! `/sessions/<id>/skills/clear`, `/forge/runs/abort-all`,
//! `/forge/runs/sweep`. Lifted out of `gateway_smoke_writes.rs`
//! so that file stays under the 500-LOC budget as more
//! mutator endpoints land.
mod common;
use common::{body_of, boot, http_post, pick_free_port};

#[test]
fn session_skills_clear_returns_400_on_bad_id_and_200_on_empty_set() {
    // Pins POST /sessions/<id>/skills/clear (the gateway-side
    // companion to `argo session clear-skills` shipped in #496).
    // Bad uuid → 400; valid-but-unknown uuid → 200 with
    // detached:0 since there's nothing to clear.
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/sessions/not-a-uuid/skills/clear", "");
    assert!(resp.contains("HTTP/1.1 400 Bad Request"), "resp={resp}");

    let resp = http_post(
        port,
        "/sessions/00000000-0000-0000-0000-000000000000/skills/clear",
        "",
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("\"detached\":0"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_runs_abort_all_returns_200_with_zero_count_on_empty_workspace() {
    // Pins POST /forge/runs/abort-all (gateway companion to
    // `argo forge runs abort-all` shipped in #495). With no
    // forge runs in the workspace, the response should be
    // 200 + {aborted:0}.
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 1);

    let resp = http_post(port, "/forge/runs/abort-all", "");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("\"aborted\":0"), "body={body}");

    assert!(child.wait().expect("wait").success());
}

#[test]
fn forge_runs_sweep_returns_200_and_echoes_param_on_empty_workspace() {
    // Pins POST /forge/runs/sweep (#520) — the gateway
    // companion to `argo forge runs sweep`. With no forge runs,
    // the response is 200 + {swept:0, older_than_minutes:N}.
    // Default is 60 when no query string is set.
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_post(port, "/forge/runs/sweep", "");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("\"swept\":0"), "body={body}");
    assert!(body.contains("\"older_than_minutes\":60"), "default 60: {body}");

    let resp = http_post(port, "/forge/runs/sweep?older-than-minutes=15", "");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let body = body_of(&resp);
    assert!(body.contains("\"older_than_minutes\":15"), "echoed: {body}");

    assert!(child.wait().expect("wait").success());
}
