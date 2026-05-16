use super::*;
use serde_json::json;

fn payload() -> serde_json::Value {
    json!({
        "argo_toml": {"ok": true},
        "argo_home": {"ok": true},
        "bundle_dirs_missing": [],
        "memory_files_missing": [],
        "db": {"present": true, "workspace_registered": true},
        "audit": {"available": true, "ok": true},
        "cron": {"daemon_started": false},
        "creds": {"set": ["OPENAI_API_KEY"], "unset": []},
        "gateway": {"running": true},
        "gateway_health": {"reachable": true, "status_code": 200},
        "routes": {"total": 12},
        "auto_approval": {
            "policy_file": {"status": "OK"},
            "worker_budget": {"status": "OK"}
        },
        "channels": {
            "email": {"status": "OK"},
            "slack": {"status": "OK"}
        },
        "network_checks": {"skipped": false, "dns": [], "tls": [], "auth": []}
    })
}

#[test]
fn summary_ok_counts_all_checks() {
    let summary = summarize(&payload());
    assert_eq!(summary.state, SummaryState::Ok);
    assert_eq!(summary.warned, 0);
    assert_eq!(summary.failed, 0);
    assert!(summary.line().starts_with("OK "));
}

#[test]
fn summary_degraded_for_warn_statuses() {
    let mut p = payload();
    p["channels"]["slack"]["status"] = json!("WARN");
    let summary = summarize(&p);
    assert_eq!(summary.state, SummaryState::Degraded);
    assert_eq!(summary.warned, 1);
    assert_eq!(summary.failed, 0);
    assert!(summary.line().contains("warn: channels"));
}

#[test]
fn summary_fail_for_failed_checks() {
    let mut p = payload();
    p["audit"]["ok"] = json!(false);
    let summary = summarize(&p);
    assert_eq!(summary.state, SummaryState::Fail);
    assert_eq!(summary.failed, 1);
    assert!(summary.line().contains("fail: audit"));
}

#[test]
fn summary_json_shape_is_compact() {
    let value = summarize(&payload()).json();
    assert_eq!(value["state"], "OK");
    assert!(value["passed"].as_u64().unwrap() > 0);
    assert_eq!(value["warned"], 0);
    assert_eq!(value["failed"], 0);
}

#[tokio::test]
async fn collect_includes_build_info_block() {
    // `collect()` is the shared payload builder behind both the CLI
    // and the gateway's `GET /doctor` endpoint. The
    // `mission-cli-version-doctor-extras` brief requires a
    // `build_info` key on the JSON envelope with seven well-known
    // fields. Probe the keys, not the values — git SHA / target /
    // rustc all depend on the invoking environment.
    let payload = collect_with_skip(true).await.expect("doctor collect");
    let build_info = &payload["build_info"];
    assert!(build_info.is_object(), "build_info missing: {payload}");
    for key in [
        "version",
        "git_sha",
        "git_date",
        "target",
        "rustc",
        "profile",
        "features",
        "binary_path",
    ] {
        assert!(
            build_info.get(key).is_some(),
            "build_info.{key} missing: {build_info}",
        );
    }
    assert!(
        build_info["features"].is_array(),
        "features must be array, got: {}",
        build_info["features"]
    );
    // Version comes from CARGO_PKG_VERSION at compile time — it
    // can never be empty.
    assert!(
        !build_info["version"].as_str().unwrap_or("").is_empty(),
        "version must be non-empty",
    );
}

#[test]
fn summary_degraded_for_one_network_failure_and_fail_for_many() {
    let mut p = payload();
    p["network_checks"]["dns"] = json!([
        {"host": "api.anthropic.com", "status": "FAIL"}
    ]);
    let one = summarize(&p);
    assert_eq!(one.state, SummaryState::Degraded);
    assert_eq!(one.warned, 1);

    p["network_checks"]["tls"] = json!([
        {"host": "api.openai.com", "status": "FAIL"}
    ]);
    let many = summarize(&p);
    assert_eq!(many.state, SummaryState::Fail);
    assert_eq!(many.failed, 1);
}

#[test]
fn build_info_long_version_string_includes_provenance() {
    // `version_string_long()` is what clap renders for
    // `argo --version`. Pin the headline shape (multi-line with the
    // labelled fields) so accidental format regressions surface
    // here rather than in operator bug reports.
    let s = crate::build_info::version_string_long();
    assert!(s.starts_with("argo "), "expected leading `argo `, got: {s}");
    for needle in ["target:", "rustc:", "profile:", "features:"] {
        assert!(s.contains(needle), "missing {needle} in: {s}");
    }
}
