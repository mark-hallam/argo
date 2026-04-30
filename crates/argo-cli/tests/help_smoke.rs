//! Smoke tests for the `--help` surface.
//!
//! Catches regressions where a subcommand goes missing from clap's
//! generated help (which is what users see when they type
//! `argo --help`) — particularly relevant after the cli.rs ↔
//! cli_subcommands.rs split landed in #122.
use std::process::Command;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn help(args: &[&str]) -> String {
    let out = Command::new(argo_bin())
        .args(args)
        .args(["--help"])
        .output()
        .unwrap_or_else(|e| panic!("spawn {args:?} --help: {e}"));
    assert!(
        out.status.success(),
        "{args:?} --help exited non-zero: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("help is utf-8")
}

#[test]
fn root_help_lists_every_top_level_subcommand() {
    let h = help(&[]);
    // Pin a representative slice — full list is in cli.rs.
    for cmd in [
        "audit",
        "cron",
        "costs",
        "db",
        "doctor",
        "memory",
        "provider",
        "session",
        "skill",
        "workspace",
    ] {
        assert!(
            h.contains(cmd),
            "`argo --help` should advertise '{cmd}', got:\n{h}"
        );
    }
}

#[test]
fn family_help_pages_render_without_crashing() {
    // One rep per Subcommand enum lifted into cli_subcommands.rs.
    for family in [
        "agents",
        "audit",
        "cron",
        "costs",
        "db",
        "eval",
        "forge",
        "gateway",
        "judge",
        "mcp",
        "memory",
        "model",
        "plugin",
        "provider",
        "route",
        "session",
        "skill",
        "workspace",
    ] {
        let h = help(&[family]);
        assert!(
            !h.is_empty(),
            "`argo {family} --help` produced empty stdout"
        );
        // clap always emits `Usage:` somewhere in the help body.
        assert!(
            h.contains("Usage:"),
            "`argo {family} --help` lacks 'Usage:' header:\n{h}"
        );
    }
}

#[test]
fn version_flag_round_trips_through_main() {
    let out = Command::new(argo_bin())
        .arg("--version")
        .output()
        .expect("spawn argo --version");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("argo "));
}
