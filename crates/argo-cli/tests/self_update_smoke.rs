//! Integration smoke for `argo self-update`.
//!
//! Gated on `ARGO_SELF_UPDATE_INTEGRATION=1` because it hits the live
//! GitHub Releases API. CI leaves this off; operators run it manually
//! before cutting a release. The default path is a CLI-surface check
//! (clap parses, `--help` prints) which never touches the network.

use std::process::Command;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

#[test]
fn self_update_help_lists_flags() {
    let out = Command::new(argo_bin())
        .args(["self-update", "--help"])
        .output()
        .expect("run argo self-update --help");
    assert!(
        out.status.success(),
        "argo self-update --help failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for flag in ["--check", "--channel", "--force", "--json"] {
        assert!(
            stdout.contains(flag),
            "self-update --help missing {flag}: {stdout}"
        );
    }
}

#[test]
fn self_update_check_against_live_github() {
    if std::env::var("ARGO_SELF_UPDATE_INTEGRATION").ok().as_deref() != Some("1") {
        eprintln!("skipping: ARGO_SELF_UPDATE_INTEGRATION!=1");
        return;
    }
    let out = Command::new(argo_bin())
        .args(["self-update", "--check", "--json"])
        .output()
        .expect("run argo self-update --check --json");
    assert!(
        out.status.success(),
        "self-update --check --json failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--json envelope parses");
    for key in ["current", "latest", "channel", "update_available"] {
        assert!(v.get(key).is_some(), "envelope missing key {key}: {v}");
    }
}
