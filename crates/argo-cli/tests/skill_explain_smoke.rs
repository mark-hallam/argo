mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn skill_explain_json_reports_provenance_and_limits_history() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    assert_success(&argo(
        home.path(),
        work.path(),
        &[
            "skill",
            "create",
            "explainable",
            "--description",
            "Explainable",
            "--body",
            "first body",
        ],
    ));
    for (version, hint) in [("0.2", "second"), ("0.3", "third"), ("0.4", "fourth")] {
        assert_success(&argo(
            home.path(),
            work.path(),
            &[
                "skill",
                "refine",
                "explainable",
                "--hint",
                hint,
                "--new-version",
                version,
            ],
        ));
    }

    let output = argo(
        home.path(),
        work.path(),
        &[
            "skill",
            "explain",
            "explainable",
            "--history-depth",
            "2",
            "--json",
        ],
    );
    assert_success(&output);
    let json = json_stdout(&output);

    assert_eq!(json["name"], "explainable");
    assert_eq!(json["current_version"], "0.4");
    assert_eq!(json["provenance"]["kind"], "operator_authored");
    assert_eq!(json["version_history"].as_array().unwrap().len(), 2);
    assert!(json["usage_summary"].is_object());

    let missing = argo(
        home.path(),
        work.path(),
        &["skill", "explain", "does-not-exist", "--json"],
    );
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("not found"));
}

fn argo(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{}", stderr(output));
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
