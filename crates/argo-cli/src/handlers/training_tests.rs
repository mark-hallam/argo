use super::*;
use tempfile::TempDir;

fn write_json(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join(".skycot/outputs/job-1");
    write_json(
        &root.join("job.json"),
        serde_json::json!({"vertical_id": "support"}),
    );
    write_json(
        &root.join("actions/a1/action.json"),
        serde_json::json!({
            "action_id": "a1",
            "job_id": "job-1",
            "action_type": "reply",
            "state": "completed",
            "created_at": "2026-05-15T00:00:00Z",
            "payload": {"ticket": 7},
            "result": {"body": "done"}
        }),
    );
    write_json(
        &root.join("actions/a2/action.json"),
        serde_json::json!({
            "action_id": "a2",
            "job_id": "job-1",
            "action_type": "draft",
            "state": "failed",
            "created_at": "2026-05-15T00:00:00Z",
            "payload": {},
            "result": null
        }),
    );
    dir
}

#[test]
fn collect_examples_filters_vertical_state_and_date() {
    let dir = fixture();
    let args = ExportArgs {
        vertical: "support".into(),
        start_micros: parse_start_date("2026-05-14").unwrap(),
        output: None,
        json: false,
    };
    let examples = collect_examples(&dir.path().join(".skycot/outputs"), &args).unwrap();
    assert_eq!(examples.len(), 1);
    assert_eq!(examples[0].metadata.action_id, "a1");
    assert_eq!(examples[0].input["payload"]["ticket"], 7);
    assert_eq!(examples[0].expected_output["body"], "done");

    let later = ExportArgs {
        start_micros: parse_start_date("2026-05-16").unwrap(),
        ..args
    };
    assert!(
        collect_examples(&dir.path().join(".skycot/outputs"), &later)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn render_jsonl_emits_one_training_row_per_line() {
    let dir = fixture();
    let args = ExportArgs {
        vertical: "support".into(),
        start_micros: 0,
        output: None,
        json: false,
    };
    let examples = collect_examples(&dir.path().join(".skycot/outputs"), &args).unwrap();
    let body = render_jsonl(&examples).unwrap();
    let lines: Vec<_> = std::str::from_utf8(&body).unwrap().lines().collect();
    assert_eq!(lines.len(), 1);
    let row: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(
        row["instruction"],
        "Perform the `reply` action for the `support` vertical."
    );
    assert_eq!(row["metadata"]["vertical_id"], "support");
}

#[test]
fn invalid_start_date_is_rejected_before_export() {
    let err = parse_start_date("not-a-date").unwrap_err();
    assert!(err.to_string().contains("invalid --start-date"));
}
