//! Smoke coverage for `argo eval list/show/compare`.

mod common;

use common::argo_bin;
use serde_json::json;
use std::fs;
use std::process::Command;

#[test]
fn eval_list_show_compare_reads_run_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let runs = temp.path().join("eval-runs");
    fs::create_dir_all(&runs).unwrap();
    write_run(
        &runs,
        "run-a",
        "2026-05-07T00:00:00Z",
        vec![
            ("invoice_missing", "invoice_review", "exact", true, 0.9),
            ("customer_reply", "customer_replies", "exact", false, 0.2),
        ],
    );
    write_run(
        &runs,
        "run-b",
        "2026-05-07T01:00:00Z",
        vec![
            ("invoice_missing", "invoice_review", "exact", false, 0.1),
            ("customer_reply", "customer_replies", "exact", true, 0.8),
        ],
    );

    let listed = argo(&runs, &["eval", "list", "--limit", "1", "--json"]);
    assert!(listed.status.success(), "stderr: {}", stderr(&listed));
    let list_json: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(list_json["ok"], true);
    assert_eq!(list_json["runs"][0]["id"], "run-b");

    let shown = argo(&runs, &["eval", "show", "run-a", "--json"]);
    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    let show_json: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(show_json["eval_run"]["id"], "run-a");
    assert_eq!(
        show_json["eval_run"]["fixtures"].as_array().unwrap().len(),
        2
    );

    let compared = argo(
        &runs,
        &[
            "eval", "compare", "run-a", "run-b", "--by", "skill", "--json",
        ],
    );
    assert!(compared.status.success(), "stderr: {}", stderr(&compared));
    let compare_json: serde_json::Value = serde_json::from_slice(&compared.stdout).unwrap();
    assert_eq!(compare_json["comparison"]["group_by"], "skill");
    assert_eq!(
        compare_json["comparison"]["regressions"][0]["fixture"],
        "invoice_missing"
    );

    let markdown = argo(
        &runs,
        &["eval", "compare", "run-a", "run-b", "--show-regressions"],
    );
    assert!(markdown.status.success(), "stderr: {}", stderr(&markdown));
    let body = String::from_utf8(markdown.stdout).unwrap();
    assert!(body.contains("## Comparison: run-a vs run-b"));
    assert!(body.contains("invoice_missing"));
    assert!(body.contains("### Regressions"));
    assert!(!body.contains("customer_replies"));
}

fn write_run(
    dir: &std::path::Path,
    id: &str,
    created_at: &str,
    fixtures: Vec<(&str, &str, &str, bool, f64)>,
) {
    let fixtures: Vec<_> = fixtures
        .into_iter()
        .map(|(fixture, skill, metric, passed, score)| {
            json!({
                "fixture": fixture,
                "skill": skill,
                "metric": metric,
                "passed": passed,
                "score": score,
            })
        })
        .collect();
    let body = json!({
        "id": id,
        "vertical": "support",
        "created_at": created_at,
        "fixtures": fixtures,
    });
    fs::write(dir.join(format!("{id}.json")), body.to_string()).unwrap();
}

fn argo(runs: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .env("ARGO_EVAL_RUNS_DIR", runs)
        .args(args)
        .output()
        .unwrap()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
