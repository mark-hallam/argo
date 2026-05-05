//! Gateway integration tests for `GET /verticals/<id>/doctor`
//! (#674). Mirrors `vertical_doctor.rs` (the CLI surface) but
//! drives the same behaviour over HTTP.

mod common;

use common::{argo_bin, body_of, http_get, pick_free_port};
use std::path::Path;
use std::process::Command;

fn boot_with_vertical(
    vertical_root: &Path,
    workspace_cwd: &Path,
    max_requests: u32,
) -> (
    std::process::Child,
    std::io::BufReader<std::process::ChildStderr>,
    u16,
) {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap().keep();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", vertical_root)
        .current_dir(workspace_cwd)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let mut stderr = std::io::BufReader::new(child.stderr.take().unwrap());
    common::await_listening(&mut stderr);
    (child, stderr, port)
}

fn repo_verticals_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals")
}

#[test]
fn gateway_get_doctor_pr_checker_returns_200_ok_passing_all_checks() {
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(&repo_verticals_dir(), wd.path(), 2);

    let resp = http_get(port, "/verticals/pr-checker/doctor");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["validate"]["ok"], true);
    assert_eq!(v["eval"]["ok"], true);
    assert_eq!(v["lint"]["ok"], true);

    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_receptionist_returns_200_ok() {
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(&repo_verticals_dir(), wd.path(), 2);
    let resp = http_get(port, "/verticals/receptionist/doctor");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true);
    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_unknown_vertical_returns_404() {
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);
    let resp = http_get(port, "/verticals/ghost/doctor");
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp: {resp}");
    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_synthetic_lint_warn_still_returns_200_ok() {
    // Lint warns don't fail the doctor — only errors do, and
    // there are no error-severity checks today.
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let v = verticals.path().join("warner");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"warner\"\nname = \"W\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\", \"file_reed_typo\"]\ndenied = []\n",
    )
    .unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);
    let resp = http_get(port, "/verticals/warner/doctor");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true, "warn-only doctor must pass: {v}");
    assert_eq!(v["lint"]["warn_count"].as_u64().unwrap(), 1);
    let _ = child.kill();
}

#[test]
fn gateway_routes_catalog_lists_doctor_endpoint() {
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(&repo_verticals_dir(), wd.path(), 1);
    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"));
    let body = body_of(&resp);
    assert!(
        body.contains("/verticals/<id>/doctor"),
        "catalog should list doctor route; body: {body}"
    );
    let _ = child.kill();
}

// ---- scored_eval gateway parity (#696) -------------------------

/// Synthetic vertical builder for scored-eval gateway tests.
/// Mirrors `tests/vertical_doctor.rs` so both surfaces exercise
/// the same convention (eval/cases.jsonl + optional sibling
/// eval/known_good_actuals.jsonl).
fn write_synthetic_vertical_with_actuals(
    verticals: &Path,
    id: &str,
    suite_body: &str,
    actuals_body: Option<&str>,
) {
    let v = verticals.join(id);
    std::fs::create_dir_all(v.join("eval")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"S\"\nversion = \"0.1.0\"\n\n\
             [evaluation]\nsuites = [\"eval/cases.jsonl\"]\n",
        ),
    )
    .unwrap();
    std::fs::write(v.join("eval/cases.jsonl"), suite_body).unwrap();
    if let Some(body) = actuals_body {
        std::fs::write(v.join("eval/known_good_actuals.jsonl"), body).unwrap();
    }
}

#[test]
fn gateway_get_doctor_gtm_engineer_includes_scored_eval_passed_ten_of_ten() {
    // GTM Engineer ships verticals/gtm-engineer/eval/known_good_
    // actuals.jsonl from #694 covering the core suite. #759 added a
    // second worker-v3-emission suite WITHOUT bundled actuals; the
    // gateway must still report scored_eval passed for the core
    // suite (10/10) and the new suite as actuals_present=true (file
    // exists in the same dir) but scored_count=0, without flipping
    // overall status.
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(&repo_verticals_dir(), wd.path(), 2);

    let resp = http_get(port, "/verticals/gtm-engineer/doctor");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true);
    let scored = &v["scored_eval"];
    assert_eq!(scored["status"], "passed", "scored_eval: {scored}");
    assert_eq!(scored["ok"], true);
    let suites = scored["suites"].as_array().unwrap();
    assert_eq!(suites.len(), 2, "expected core + worker_v3_emission suites");
    let core = suites
        .iter()
        .find(|s| s["path"] == "eval/gtm_engineer_core.jsonl")
        .expect("core suite present");
    assert_eq!(core["actuals_present"], true);
    assert_eq!(core["actuals_path"], "eval/known_good_actuals.jsonl");
    assert_eq!(core["scored_count"], 10);
    assert_eq!(core["passed"], 10);
    assert_eq!(core["failed"], 0);
    assert_eq!(core["pass_rate"].as_f64().unwrap(), 1.0);
    let emission = suites
        .iter()
        .find(|s| s["path"] == "eval/worker_v3_emission.jsonl")
        .expect("worker_v3_emission suite present");
    assert_eq!(emission["actuals_present"], true);
    assert_eq!(emission["passed"], 0);
    assert_eq!(emission["failed"], 0);

    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_pr_checker_includes_scored_eval_skipped() {
    // pr-checker has no known_good_actuals.jsonl. Gateway must
    // still return 200 OK; scored_eval.status="skipped"; ok=true.
    // Locks the back-compat behaviour for verticals that don't
    // ship actuals yet.
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(&repo_verticals_dir(), wd.path(), 2);

    let resp = http_get(port, "/verticals/pr-checker/doctor");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], true, "pr-checker must continue to pass: {v}");
    let scored = &v["scored_eval"];
    assert_eq!(scored["status"], "skipped");
    let s = &scored["suites"][0];
    assert_eq!(s["actuals_present"], false);
    // Path is reported even when absent so callers know what to
    // create — same shape as the CLI doctor JSON.
    assert_eq!(s["actuals_path"], "eval/known_good_actuals.jsonl");

    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_returns_422_when_known_good_actuals_contain_failing_actual() {
    // Synthetic vertical: suite expects must_include=["alpha"];
    // actuals deliberately supply a wrong response. Gateway
    // must return 422 (mirrors the CLI exit-non-zero) and the
    // body must carry scored_eval.status="failed" so the caller
    // can see WHICH check failed without parsing twice.
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    write_synthetic_vertical_with_actuals(
        verticals.path(),
        "bad-actuals-gw",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{\"must_include\":[\"alpha\"]}}\n",
        Some("{\"id\":\"a\",\"actual\":\"only beta here\"}\n"),
    );
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);

    let resp = http_get(port, "/verticals/bad-actuals-gw/doctor");
    assert!(
        resp.contains("HTTP/1.1 422 Unprocessable Entity"),
        "failing scored fixture must return 422; resp: {resp}"
    );
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], false);
    let scored = &v["scored_eval"];
    assert_eq!(scored["status"], "failed");
    assert_eq!(scored["ok"], false);
    let s = &scored["suites"][0];
    assert_eq!(s["passed"], 0);
    assert_eq!(s["failed"], 1);
    assert!(
        s["error"].as_str().unwrap().contains("fixture"),
        "per-suite error should describe failure: {s}"
    );

    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_returns_422_when_actuals_file_is_malformed() {
    // The actuals file has invalid JSON on line 2. Gateway must
    // return 422 + scored_eval.status="failed"; the per-suite
    // error must name the bad line so the operator can jump
    // straight to it.
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    write_synthetic_vertical_with_actuals(
        verticals.path(),
        "malformed-gw",
        "{\"id\":\"a\",\"input\":\"\",\"expected\":{}}\n",
        Some("{\"id\":\"a\",\"actual\":\"ok\"}\nNOT JSON\n"),
    );
    let (mut child, _stderr, port) = boot_with_vertical(verticals.path(), wd.path(), 1);

    let resp = http_get(port, "/verticals/malformed-gw/doctor");
    assert!(
        resp.contains("HTTP/1.1 422 Unprocessable Entity"),
        "malformed actuals must return 422; resp: {resp}"
    );
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["ok"], false);
    let s = &v["scored_eval"]["suites"][0];
    let err = s["error"].as_str().unwrap();
    assert!(err.contains("parse error"), "err: {err}");
    assert!(err.contains("line 2"), "err must name line: {err}");

    let _ = child.kill();
}

#[test]
fn gateway_get_doctor_existing_three_check_fields_remain_intact() {
    // Regression lock for #674's CLI/gateway parity: the
    // existing `validate`/`eval`/`lint` fields must still be
    // present + carry the same shape after #696. Adding
    // `scored_eval` is purely additive.
    let wd = tempfile::tempdir().unwrap();
    let (mut child, _stderr, port) = boot_with_vertical(&repo_verticals_dir(), wd.path(), 1);

    let resp = http_get(port, "/verticals/pr-checker/doctor");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    // The four blocks the JSON shape promises.
    for key in ["validate", "eval", "lint", "scored_eval"] {
        assert!(v.get(key).is_some(), "missing key `{key}`: {v}");
    }
    // validate / eval / lint shapes unchanged from before #696.
    assert!(v["validate"]["ok"].is_boolean());
    assert!(v["eval"]["ok"].is_boolean());
    assert!(v["lint"]["error_count"].is_number());
    assert!(v["lint"]["findings"].is_array());

    let _ = child.kill();
}
