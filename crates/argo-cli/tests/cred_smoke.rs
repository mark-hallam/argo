mod common;

use common::argo_bin;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn cred_subcommands_round_trip() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let set = run(
        workspace.path(),
        home.path(),
        &["cred", "set", "POSTMARK_API_KEY", "pm_123"],
    );
    assert_success(&set);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(workspace.path().join(".argo/credentials.toml"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    let masked = run(workspace.path(), home.path(), &["cred", "list"]);
    assert_success(&masked);
    let masked_stdout = stdout(&masked);
    assert!(masked_stdout.contains("POSTMARK_API_KEY=********"));
    assert!(!masked_stdout.contains("pm_123"));

    let revealed = run(workspace.path(), home.path(), &["cred", "list", "--reveal"]);
    assert_success(&revealed);
    assert!(stdout(&revealed).contains("POSTMARK_API_KEY=pm_123"));

    let export = run(
        workspace.path(),
        home.path(),
        &["cred", "export", "--shell", "bash"],
    );
    assert_success(&export);
    assert!(stdout(&export).contains("export POSTMARK_API_KEY='pm_123'"));

    let clear = run(
        workspace.path(),
        home.path(),
        &["cred", "clear", "POSTMARK_API_KEY"],
    );
    assert_success(&clear);
    let after_clear = run(workspace.path(), home.path(), &["cred", "list", "--reveal"]);
    assert_success(&after_clear);
    assert!(!stdout(&after_clear).contains("POSTMARK_API_KEY"));

    let import = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .env("ARGO_CRED_SMOKE_TOKEN", "smoke-token")
        .args(["cred", "import", "--shell", "bash"])
        .output()
        .unwrap();
    assert_success(&import);
    let imported = run(workspace.path(), home.path(), &["cred", "list", "--reveal"]);
    assert_success(&imported);
    assert!(stdout(&imported).contains("ARGO_CRED_SMOKE_TOKEN=smoke-token"));

    let clear_all = run(workspace.path(), home.path(), &["cred", "clear-all"]);
    assert_success(&clear_all);
    let empty = run(workspace.path(), home.path(), &["cred", "list", "--reveal"]);
    assert_success(&empty);
    assert!(stdout(&empty).trim().is_empty());
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env_remove("POSTMARK_API_KEY")
        .env_remove("ARGO_CRED_SMOKE_TOKEN")
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
