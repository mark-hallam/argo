mod common;

use common::argo_bin;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn connector_list_and_show_read_workspace_manifests() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let manifest_dir = workspace.path().join("manifests/connectors");
    std::fs::create_dir_all(&manifest_dir).unwrap();
    std::fs::write(
        manifest_dir.join("gmail.toml"),
        r#"
id = "gmail"
display_name = "Gmail"
required_env = ["ARGO_GMAIL_CLIENT_ID"]

[[tools]]
name = "gmail.search"

[[tools]]
name = "gmail.send"
"#,
    )
    .unwrap();

    let list = run(
        workspace.path(),
        home.path(),
        &["connector", "list", "--installed-only", "--json"],
    );
    assert_success(&list);
    let list_stdout = stdout(&list);
    assert!(list_stdout.contains("\"kind\": \"gmail\""));
    assert!(list_stdout.contains("\"installed\": true"));
    assert!(!list_stdout.contains("\"kind\": \"slack\""));

    let show = run(
        workspace.path(),
        home.path(),
        &["connector", "show", "gmail"],
    );
    assert_success(&show);
    let show_stdout = stdout(&show);
    assert!(show_stdout.contains("# connector gmail"));
    assert!(show_stdout.contains("methods: 2"));
    assert!(show_stdout.contains("gmail.search"));
    assert!(show_stdout.contains("```toml"));
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}
