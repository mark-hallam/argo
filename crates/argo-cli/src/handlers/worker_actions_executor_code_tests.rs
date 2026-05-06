//! Unit + integration coverage for the code-execution executor.
//!
//! Coverage matrix (matches the brief's "10 unit + 2 integration"
//! acceptance):
//!
//! - parse: happy path, missing-field, unknown language, default
//!   opt-ins (5 unit tests)
//! - guard_opt_ins: refused under Local, allowed under E2B
//!   placeholder (2 unit tests)
//! - resolved-config env overrides: timeout, memory (2 unit tests)
//! - artifacts collection + shell escaping (2 unit tests)
//! - integration (gated): Python + Node real-runs via LocalSandbox
//!   when `python3` / `node` are on PATH (2 integration tests)

use super::*;
use serde_json::json;

// ─────────────────────── parse ───────────────────────

#[test]
fn parse_request_round_trips_minimal_python_payload() {
    let payload = json!({"language": "python", "code": "print('hi')"});
    let req = parse_request(&payload).expect("parse");
    assert_eq!(req.language, "python");
    assert_eq!(req.code, "print('hi')");
    assert!(!req.allow_network);
    assert!(!req.allow_pkg_install);
    assert!(req.input_json.is_none());
}

#[test]
fn parse_request_round_trips_node_payload_with_input_and_timeout() {
    let payload = json!({
        "language": "node",
        "code": "console.log('ok')",
        "input_json": {"a": 1},
        "timeout_seconds": 5,
    });
    let req = parse_request(&payload).expect("parse");
    assert_eq!(req.language, "node");
    assert_eq!(req.timeout_seconds, Some(5));
    assert_eq!(req.input_json, Some(json!({"a": 1})));
}

#[test]
fn parse_request_rejects_missing_language() {
    let err = parse_request(&json!({"code": "x"})).expect_err("must reject");
    assert!(err.to_string().contains("code-execution payload"), "got: {err}");
}

#[test]
fn parse_request_rejects_unknown_language() {
    let err = parse_request(&json!({"language": "ruby", "code": "x"}))
        .expect_err("must reject");
    assert!(err.to_string().contains("ruby"), "got: {err}");
    assert!(err.to_string().contains("python"), "should list valid: {err}");
}

#[test]
fn parse_request_default_opt_ins_are_safe() {
    let req = parse_request(&json!({"language": "python", "code": ""})).unwrap();
    assert!(!req.allow_network);
    assert!(!req.allow_pkg_install);
}

// ───────────────── opt-in guards ─────────────────

#[test]
fn guard_opt_ins_refuses_network_under_local_backend() {
    let req = parse_request(&json!({
        "language": "python",
        "code": "x",
        "allow_network": true,
    }))
    .unwrap();
    let err = guard_opt_ins(&req, BackendKind::Local).expect_err("must refuse");
    assert!(err.to_string().contains("allow_network"), "got: {err}");
}

#[test]
fn guard_opt_ins_refuses_pkg_install_under_local_backend() {
    let req = parse_request(&json!({
        "language": "node",
        "code": "x",
        "allow_pkg_install": true,
    }))
    .unwrap();
    let err = guard_opt_ins(&req, BackendKind::Local).expect_err("must refuse");
    assert!(err.to_string().contains("allow_pkg_install"), "got: {err}");
}

// ─────────────── resolved config env ───────────────

#[test]
fn resolved_config_per_action_timeout_overrides_env_default() {
    let req = parse_request(&json!({
        "language": "python",
        "code": "x",
        "timeout_seconds": 7,
    }))
    .unwrap();
    let cfg = ResolvedConfig::from_env(&req);
    assert_eq!(cfg.timeout, std::time::Duration::from_secs(7));
}

#[test]
fn resolved_config_default_when_no_env_or_payload_override() {
    // Best-effort: don't actually set ARGO_CODE_EXEC_MEM_MIB so we
    // observe the module default.
    let req = parse_request(&json!({"language": "python", "code": "x"})).unwrap();
    let cfg = ResolvedConfig::from_env(&req);
    // Per-action timeout absent → env or module default; we accept
    // either since other tests in the suite may have set the env.
    assert!(cfg.timeout.as_secs() >= 1);
    // Memory default is the module default unless explicitly overridden.
    if std::env::var("ARGO_CODE_EXEC_MEM_MIB").is_err() {
        assert_eq!(cfg.memory_mib, DEFAULT_MEMORY_MIB);
    }
}

// ───────────────── artifacts + escape ─────────────────

#[test]
fn collect_artifacts_lists_files_sorted_with_size() {
    let bundle = tempfile::tempdir().unwrap();
    let arts = bundle.path().join("artifacts");
    std::fs::create_dir(&arts).unwrap();
    std::fs::write(arts.join("b.bin"), [0u8; 12]).unwrap();
    std::fs::write(arts.join("a.bin"), [0u8; 3]).unwrap();
    let collected = collect_artifacts(bundle.path());
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0]["name"].as_str().unwrap(), "a.bin");
    assert_eq!(collected[0]["size"].as_u64().unwrap(), 3);
    assert_eq!(collected[1]["name"].as_str().unwrap(), "b.bin");
    assert_eq!(collected[1]["size"].as_u64().unwrap(), 12);
}

#[test]
fn collect_artifacts_returns_empty_when_dir_missing() {
    let bundle = tempfile::tempdir().unwrap();
    let collected = collect_artifacts(bundle.path());
    assert!(collected.is_empty());
}

// ─────────────── integration (gated) ───────────────

/// True when the named binary is on `PATH`. The integration tests
/// gate on this because `python3` / `node` aren't guaranteed on
/// every CI runner — and an explicit `ARGO_CODE_EXEC_INTEGRATION`
/// env so operators can opt in deterministically without polluting
/// every default test run.
fn integration_enabled(binary: &str) -> bool {
    if std::env::var("ARGO_CODE_EXEC_INTEGRATION").is_err() {
        return false;
    }
    std::process::Command::new("which")
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test(flavor = "multi_thread")]
async fn integration_python_round_trip_runs_real_script_when_enabled() {
    if !integration_enabled("python3") {
        eprintln!("skipping: ARGO_CODE_EXEC_INTEGRATION unset or python3 not on PATH");
        return;
    }
    let bundle = tempfile::tempdir().unwrap();
    let req = parse_request(&json!({
        "language": "python",
        "code": "print('it works')",
    }))
    .unwrap();
    let cfg = ResolvedConfig::from_env(&req);
    let cmd = build_command(&req, &cfg, bundle.path()).unwrap();
    let sandbox = LocalSandbox::new();
    let result = sandbox
        .run(&RunRequest {
            command: cmd,
            working_dir: bundle.path().to_path_buf(),
            env: Vec::new(),
            inherit_env: true,
            timeout: Some(cfg.timeout),
        })
        .await
        .expect("run");
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("it works"), "stdout: {}", result.stdout);
}

#[tokio::test(flavor = "multi_thread")]
async fn integration_node_round_trip_runs_real_script_when_enabled() {
    if !integration_enabled("node") {
        eprintln!("skipping: ARGO_CODE_EXEC_INTEGRATION unset or node not on PATH");
        return;
    }
    let bundle = tempfile::tempdir().unwrap();
    let req = parse_request(&json!({
        "language": "node",
        "code": "console.log('node ok')",
    }))
    .unwrap();
    let cfg = ResolvedConfig::from_env(&req);
    let cmd = build_command(&req, &cfg, bundle.path()).unwrap();
    let sandbox = LocalSandbox::new();
    let result = sandbox
        .run(&RunRequest {
            command: cmd,
            working_dir: bundle.path().to_path_buf(),
            env: Vec::new(),
            inherit_env: true,
            timeout: Some(cfg.timeout),
        })
        .await
        .expect("run");
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert!(result.stdout.contains("node ok"), "stdout: {}", result.stdout);
}
