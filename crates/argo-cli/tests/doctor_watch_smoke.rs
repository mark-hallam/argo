mod common;

use serde_json::Value;
use std::process::{Command, Output};

#[test]
fn doctor_watch_append_mode_emits_three_json_envelopes() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(home.path(), work.path());
    assert_success(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3, "stdout:\n{stdout}");
    for line in lines {
        let payload: Value = serde_json::from_str(line).unwrap();
        assert!(payload["cwd"].as_str().is_some());
        assert!(payload["argo_home"].is_object());
        assert!(payload["channels"].is_object());
    }
}

fn run(home: &std::path::Path, cwd: &std::path::Path) -> Output {
    let mut cmd = Command::new(common::argo_bin());
    cmd.env("ARGO_HOME", home)
        .env("ARGO_DOCTOR_WATCH_MAX_TICKS", "3")
        .env_remove("POSTMARK_SERVER_TOKEN")
        .env_remove("SLACK_APPROVALS_BOT_TOKEN")
        .env_remove("TWILIO_ACCOUNT_SID")
        .env_remove("TWILIO_AUTH_TOKEN")
        .env_remove("EXPO_ACCESS_TOKEN")
        .current_dir(cwd)
        .args(["doctor", "watch", "--interval", "1ms"])
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
