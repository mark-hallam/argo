//! Pin the top-level subcommand surface so the
//! `claude-cli-subcommands-split` refactor (and any future split)
//! cannot silently drop a subcommand. The list mirrors what
//! `argo --help` advertises today; add to it when a new top-level
//! subcommand is intentionally introduced.

use std::process::Command;

const ARGO_BIN: &str = env!("CARGO_BIN_EXE_argo");

const EXPECTED_TOP_LEVEL_SUBCOMMANDS: &[&str] = &[
    "new",
    "resume",
    "agents",
    "gateway",
    "mcp",
    "plugin",
    "provider",
    "model",
    "skill",
    "memory",
    "cron",
    "work",
    "worker",
    "tenant",
    "org",
    "workflow",
    "trajectory",
    "action",
    "activity",
    "audit",
    "setup",
    "vertical",
    "genesis",
    "messages",
    "workspace",
    "db",
    "costs",
    "budget",
    "session",
    "handoff",
    "forge",
    "judge",
    "route",
    "steer",
    "trace",
    "eval",
    "chat",
    "tui",
    "tools",
    "doctor",
    "health",
    "import",
    "update",
    "help",
];

#[test]
fn top_level_subcommands_unchanged_post_split() {
    let out = Command::new(ARGO_BIN)
        .arg("--help")
        .output()
        .expect("argo --help should run");
    assert!(out.status.success(), "argo --help exited non-zero");
    let help = String::from_utf8(out.stdout).expect("stdout should be utf8");
    for sub in EXPECTED_TOP_LEVEL_SUBCOMMANDS {
        let line_marker = format!("\n  {sub} ");
        let line_marker_eol = format!("\n  {sub}\n");
        assert!(
            help.contains(&line_marker) || help.contains(&line_marker_eol),
            "subcommand `{sub}` missing from `argo --help`:\n{help}",
        );
    }
}

#[test]
fn skill_subcommands_unchanged_post_split() {
    let out = Command::new(ARGO_BIN)
        .args(["skill", "--help"])
        .output()
        .expect("argo skill --help should run");
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    for sub in [
        "list",
        "count",
        "view",
        "verify",
        "create",
        "search",
        "versions",
        "ab-test",
        "ab-status",
        "ab-promote",
        "review",
        "conflicts",
        "refine",
        "install",
        "lock",
        "unlock",
    ] {
        assert!(help.contains(sub), "skill sub `{sub}` missing:\n{help}");
    }
}

#[test]
fn provider_subcommands_unchanged_post_split() {
    let out = Command::new(ARGO_BIN)
        .args(["provider", "--help"])
        .output()
        .expect("argo provider --help should run");
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    for sub in ["list", "show", "validate", "validate-all", "test"] {
        assert!(help.contains(sub), "provider sub `{sub}` missing:\n{help}");
    }
}

#[test]
fn cron_subcommands_unchanged_post_split() {
    let out = Command::new(ARGO_BIN)
        .args(["cron", "--help"])
        .output()
        .expect("argo cron --help should run");
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    for sub in ["list", "create", "edit", "delete", "trigger", "tick", "daemon"] {
        assert!(help.contains(sub), "cron sub `{sub}` missing:\n{help}");
    }
}
