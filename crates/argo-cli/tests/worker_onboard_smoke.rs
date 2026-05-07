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

#[test]
fn worker_onboard_round_trips_through_show() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let spec = workspace.path().join("maya.toml");
    std::fs::write(
        &spec,
        r#"
        vertical_id = "gtm-engineer"
        memory_namespace = "maya-demo"

        [persona]
        name = "Maya"
        voice = "Be concise and commercial."
        signature = "Maya"

        [role]
        description = "Own approved marketplace follow-up."

        [[channels]]
        channel = "email"
        target = "ops@example.test"
        risk_level = "high"

        [[duties]]
        cron = "0 9 * * 1"
        entry_action = "weekly_pipeline_review"
        description = "Review approved leads."
        "#,
    )
    .unwrap();

    let out = run(
        workspace.path(),
        home.path(),
        &[
            "worker",
            "onboard",
            "--persona-spec",
            spec.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let worker_id = body["worker_id"].as_str().unwrap();
    let tenant_id = body["tenant_id"].as_str().unwrap();
    assert!(!worker_id.is_empty());

    let show = run(
        workspace.path(),
        home.path(),
        &["worker", "show", worker_id, "--tenant", tenant_id, "--json"],
    );
    assert!(
        show.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&show.stderr)
    );
    let shown: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(shown["worker"]["persona"]["name"], "Maya");
    assert_eq!(shown["worker"]["channels"].as_array().unwrap().len(), 1);
}
