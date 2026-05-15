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
        }
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
