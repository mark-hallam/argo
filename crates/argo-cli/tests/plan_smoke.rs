//! Integration smoke for `argo plan generate/show/execute`
//! (Stream `claude-runtime-planner-trait`). Covers the offline
//! TemplatePlanner round-trip — generate → show → execute (dry).

use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn plan_generate_show_execute_round_trip() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    let yaml_path = workspace.path().join("plan.yaml");
    let yaml_str = yaml_path.to_str().unwrap();

    // Generate via offline TemplatePlanner. Should write the YAML
    // + a sidecar reasoning file.
    let gen = run(
        workspace.path(),
        home.path(),
        &[
            "plan",
            "generate",
            "research the latest skating gear trends",
            "--worker",
            "wkr-1",
            "--output",
            yaml_str,
            "--json",
        ],
    );
    assert_ok(&gen, "plan generate");
    let gen_json: serde_json::Value = serde_json::from_slice(&gen.stdout).unwrap();
    assert_eq!(gen_json["planner_kind"], "template");
    assert_eq!(gen_json["template_name"], "research-then-summarise");
    assert_eq!(gen_json["tasks"], 2);
    assert!(yaml_path.exists());
    let sidecar = workspace.path().join("plan.yaml.reasoning");
    assert!(sidecar.exists(), "sidecar should be written next to yaml");

    // Show should read back the YAML + sidecar.
    let show = run(
        workspace.path(),
        home.path(),
        &["plan", "show", yaml_str, "--json"],
    );
    assert_ok(&show, "plan show");
    let show_json: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(show_json["tasks"], 2);
    assert!(show_json["reasoning"]
        .as_str()
        .unwrap_or("")
        .contains("research-then-summarise"));

    // Execute should run via the mock executor cleanly.
    let exec = run(
        workspace.path(),
        home.path(),
        &["plan", "execute", yaml_str, "--json"],
    );
    assert_ok(&exec, "plan execute");
    let exec_json: serde_json::Value = serde_json::from_slice(&exec.stdout).unwrap();
    assert_eq!(exec_json["ok"], true);

    // Dry-run should print task order without running.
    let dry = run(
        workspace.path(),
        home.path(),
        &["plan", "execute", yaml_str, "--dry-run", "--json"],
    );
    assert_ok(&dry, "plan execute --dry-run");
    let dry_json: serde_json::Value = serde_json::from_slice(&dry.stdout).unwrap();
    assert_eq!(dry_json["dry_run"], true);
    let order = dry_json["tasks"].as_array().unwrap();
    // research-then-summarise has 2 tasks: gather, summarise (in topo order).
    assert_eq!(order.len(), 2);
    assert_eq!(order[0], "gather");
    assert_eq!(order[1], "summarise");
}

#[test]
fn plan_generate_rejects_unmatched_goal_offline() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &[
            "plan",
            "generate",
            "do something the planner has never heard of",
            "--worker",
            "wkr-1",
        ],
    );
    assert!(!bad.status.success(), "expected non-zero on no template match");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("no template matched") || stderr.contains("template"),
        "expected helpful error, got: {stderr}",
    );
}
